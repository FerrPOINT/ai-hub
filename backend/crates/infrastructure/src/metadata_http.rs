//! OpenRouter metadata only. No configurable origin or model invocation method.
use crate::openrouter_metadata::{MAX_METADATA_BYTES, decode_account, decode_catalog};
use aihub_domain::{
    catalog::{AccountObservation, CatalogObservation},
    error::HubError,
};
use reqwest::{
    Client,
    dns::{Addrs, Name, Resolve, Resolving},
    header::HeaderValue,
};
use std::{net::IpAddr, sync::Arc, time::Duration};
use zeroize::Zeroizing;

pub(crate) struct MetadataHttp {
    client: Client,
}
pub(crate) enum ReadFailure {
    Known,
    Unknown,
}
struct PublicOpenRouterDns;
impl Resolve for PublicOpenRouterDns {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            if host != "openrouter.ai" {
                return Err("metadata origin denied".into());
            }
            let addresses = tokio::time::timeout(
                Duration::from_secs(5),
                tokio::net::lookup_host((host.as_str(), 443)),
            )
            .await
            .map_err(|_| "metadata DNS timeout")??
            .collect::<Vec<_>>();
            if addresses.is_empty() || addresses.iter().any(|a| !public_ip(a.ip())) {
                return Err("metadata DNS address denied".into());
            }
            Ok(Box::new(addresses.into_iter()) as Addrs)
        })
    }
}
fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_multicast()
                || ip.is_broadcast()
                || ip.is_documentation()
                || a == 0
                || a >= 240
                || a == 100 && (64..=127).contains(&b)
                || a == 192 && b == 0 && c == 0
                || a == 192 && b == 88 && c == 99
                || a == 198 && (b == 18 || b == 19))
        }
        IpAddr::V6(ip) => {
            let s = ip.segments();
            // Ordinary global unicast only; reject transition/documentation/special-purpose ranges.
            s[0] & 0xe000 == 0x2000
                && s[0] != 0x2002
                && !(s[0] == 0x2001 && (s[1] < 0x0200 || s[1] == 0x0db8))
                && !(s[0] == 0x3fff && s[1] < 0x1000)
        }
    }
}
fn builder() -> reqwest::ClientBuilder {
    Client::builder()
        .dns_resolver(Arc::new(PublicOpenRouterDns))
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .https_only(true)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(20))
        .pool_idle_timeout(Duration::from_secs(30))
}
impl MetadataHttp {
    pub(crate) fn new() -> Result<Self, HubError> {
        Ok(Self {
            client: builder().build().map_err(|_| HubError::Unavailable)?,
        })
    }
    pub(crate) async fn read(
        &self,
        secret: &[u8],
    ) -> Result<(CatalogObservation, AccountObservation), ReadFailure> {
        let account = get(
            &self.client,
            "https://openrouter.ai/api/v1/key",
            secret,
            65536,
        )
        .await?;
        let account = decode_account(&account).map_err(|_| ReadFailure::Known)?;
        if account.is_management_key != Some(false)
            || account.expires_at.is_some_and(|t| t <= chrono::Utc::now())
        {
            return Err(ReadFailure::Known);
        }
        let models = get(
            &self.client,
            "https://openrouter.ai/api/v1/models",
            secret,
            MAX_METADATA_BYTES,
        )
        .await?;
        let catalog =
            decode_catalog(&models, chrono::Utc::now()).map_err(|_| ReadFailure::Known)?;
        Ok((catalog, account))
    }
}
async fn get(
    client: &Client,
    url: &str,
    secret: &[u8],
    bound: usize,
) -> Result<Zeroizing<Vec<u8>>, ReadFailure> {
    let mut bearer = Zeroizing::new(b"Bearer ".to_vec());
    bearer.extend_from_slice(secret);
    let mut header = HeaderValue::from_bytes(&bearer).map_err(|_| ReadFailure::Known)?;
    header.set_sensitive(true);
    let mut response = client
        .get(url)
        .header(reqwest::header::AUTHORIZATION, header)
        .send()
        .await
        .map_err(|_| ReadFailure::Unknown)?;
    if !response.status().is_success() {
        return Err(if response.status().is_server_error() {
            ReadFailure::Unknown
        } else {
            ReadFailure::Known
        });
    }
    if response.content_length().is_some_and(|n| n > bound as u64) {
        return Err(ReadFailure::Known);
    }
    let mut body = Zeroizing::new(Vec::new());
    while let Some(chunk) = response.chunk().await.map_err(|_| ReadFailure::Unknown)? {
        if body.len().saturating_add(chunk.len()) > bound {
            return Err(ReadFailure::Known);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dns_denies_internal_and_transition_addresses() {
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "169.254.169.254",
            "100.64.0.1",
            "192.0.0.1",
            "198.18.0.1",
            "192.0.2.2",
            "0.1.2.3",
            "240.0.0.1",
            "::1",
            "::ffff:8.8.8.8",
            "fc00::1",
            "fe80::1",
            "2002:7f00:1::1",
            "2001:db8::1",
            "2001::1",
            "3fff::1",
        ] {
            assert!(!public_ip(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["104.18.3.115", "8.8.8.8", "2606:4700::1111"] {
            assert!(public_ip(ip.parse().unwrap()), "{ip}");
        }
    }
    #[tokio::test]
    async fn resolver_rejects_unowned_host_before_lookup() {
        assert!(
            PublicOpenRouterDns
                .resolve("example.test".parse().unwrap())
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn bounded_metadata_transport_rejects_redirects_oversize_and_never_retries() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        for (response, known) in [
            (
                "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/stolen\r\nContent-Length: 0\r\n\r\n",
                true,
            ),
            (
                "HTTP/1.1 503 Unavailable\r\nContent-Length: 0\r\n\r\n",
                false,
            ),
            ("HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n", true),
            (
                "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n9\r\n123456789\r\n0\r\n\r\n",
                true,
            ),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}/metadata", listener.local_addr().unwrap());
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut header = [0u8; 4096];
                let n = socket.read(&mut header).await.unwrap();
                assert!(
                    String::from_utf8_lossy(&header[..n])
                        .to_lowercase()
                        .contains("authorization: bearer synthetic")
                );
                socket.write_all(response.as_bytes()).await.unwrap();
                socket.shutdown().await.unwrap();
                drop(socket);
                assert!(
                    tokio::time::timeout(Duration::from_millis(150), listener.accept())
                        .await
                        .is_err(),
                    "no transport retry"
                );
            });
            // Only this controlled test enables plaintext loopback. Production is HTTPS-only with owned DNS.
            let client = builder().https_only(false).build().unwrap();
            let result = get(&client, &url, b"synthetic", 8).await;
            assert!(
                matches!(result, Err(ReadFailure::Known)) && known
                    || matches!(result, Err(ReadFailure::Unknown)) && !known
            );
            server.await.unwrap();
        }
    }
}
