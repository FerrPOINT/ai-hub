use aihub_domain::error::HubError;
use std::{net::SocketAddr, path::PathBuf};
use url::Url;
use uuid::Uuid;
use zeroize::Zeroizing;

pub struct Config {
    pub installation_id: Uuid,
    pub listen: SocketAddr,
    pub database_url: Zeroizing<String>,
    pub vault_key: Zeroizing<Vec<u8>>,
    pub auth_issuer: Url,
    pub public_origin: Url,
    pub admin_origin: Option<Url>,
    pub external_calls: bool,
    pub max_body_bytes: usize,
    pub maintenance_tick_seconds: u64,
}

fn required(name: &str) -> Result<String, HubError> {
    std::env::var(name)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .ok_or(HubError::Invalid("отсутствует обязательная конфигурация"))
}

fn secret_file(name: &str) -> Result<Vec<u8>, HubError> {
    let path = PathBuf::from(required(name)?);
    if !path.is_absolute() || !path.is_file() {
        return Err(HubError::Invalid(
            "secret file должен быть существующим абсолютным путём",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = path
            .metadata()
            .map_err(|_| HubError::Unavailable)?
            .permissions()
            .mode();
        if mode & 0o077 != 0 {
            return Err(HubError::Invalid(
                "secret file доступен другим пользователям",
            ));
        }
    }
    std::fs::read(path).map_err(|_| HubError::Unavailable)
}

pub fn trusted_origin(value: &str) -> Result<Url, HubError> {
    let url = Url::parse(value).map_err(|_| HubError::Invalid("URL конфигурации"))?;
    let loopback = url.host_str().is_some_and(|host| {
        host == "localhost"
            || host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    });
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || (url.scheme() != "https" && !(url.scheme() == "http" && loopback))
    {
        return Err(HubError::Invalid("недоверенный origin конфигурации"));
    }
    Ok(url)
}

impl Config {
    pub fn load() -> Result<Self, HubError> {
        let maintenance_tick_seconds = std::env::var("AIHUB_MAINTENANCE_TICK_SECONDS")
            .unwrap_or_else(|_| "5".into())
            .parse::<u64>()
            .map_err(|_| HubError::Invalid("maintenance interval"))?;
        if !(1..=60).contains(&maintenance_tick_seconds) {
            return Err(HubError::Invalid("maintenance interval"));
        }
        let installation_id = Uuid::parse_str(&required("AIHUB_INSTALLATION_ID")?)
            .map_err(|_| HubError::Invalid("installation UUID"))?;
        if installation_id.is_nil() {
            return Err(HubError::Invalid("installation UUID не может быть пустым"));
        }
        let auth_issuer = trusted_origin(&required("AIHUB_AUTH_ISSUER")?)?;
        let jwks = trusted_origin(&required("AIHUB_AUTH_JWKS_URI")?)?;
        let auth_base = trusted_origin(&required("AIHUB_AUTH_BASE_URL")?)?;
        if auth_issuer.origin() != jwks.origin()
            || auth_issuer.origin() != auth_base.origin()
            || jwks.path() != "/oidc/jwks"
            || auth_issuer.path() != "/"
            || auth_base.path() != "/"
        {
            return Err(HubError::Invalid(
                "Auth issuer/JWKS/bridge origins должны совпадать",
            ));
        }
        let public_origin = trusted_origin(&required("AIHUB_PUBLIC_ORIGIN")?)?;
        let admin_origin = std::env::var("AIHUB_ADMIN_ORIGIN")
            .ok()
            .map(|value| trusted_origin(&value))
            .transpose()?;
        if admin_origin.as_ref().is_some_and(|url| url.path() != "/") {
            return Err(HubError::Invalid("Admin origin без path"));
        }
        if public_origin.path() != "/" {
            return Err(HubError::Invalid(
                "public origin должен быть origin без path",
            ));
        }
        let database_bytes = Zeroizing::new(secret_file("AIHUB_DATABASE_URL_FILE")?);
        let database_url = Zeroizing::new(
            std::str::from_utf8(&database_bytes)
                .map_err(|_| HubError::Invalid("DSN encoding"))?
                .trim()
                .to_owned(),
        );
        let dsn = Url::parse(&database_url).map_err(|_| HubError::Invalid("DSN конфигурации"))?;
        if !matches!(dsn.scheme(), "postgres" | "postgresql")
            || !dsn.path().starts_with("/aihub_")
            || !dsn.username().starts_with("aihub_")
        {
            return Err(HubError::Invalid("требуется собственная aihub_ БД и роль"));
        }
        let vault_key = Zeroizing::new(secret_file("AIHUB_VAULT_KEY_FILE")?);
        if vault_key.len() != 32 {
            return Err(HubError::Invalid("vault key должен содержать 32 байта"));
        }
        let external_calls = match std::env::var("AIHUB_EXTERNAL_CALLS").as_deref() {
            Ok("true") => true,
            Ok("false") | Err(_) => false,
            _ => return Err(HubError::Invalid("external_calls boolean")),
        };
        let max_body_bytes = std::env::var("AIHUB_MAX_BODY_BYTES")
            .unwrap_or_else(|_| "2097152".into())
            .parse::<usize>()
            .map_err(|_| HubError::Invalid("body limit"))?;
        if !(1024..=2097152).contains(&max_body_bytes) {
            return Err(HubError::Invalid("body limit вне диапазона"));
        }
        let listen = required("AIHUB_LISTEN")?
            .parse()
            .map_err(|_| HubError::Invalid("listen address"))?;
        Ok(Self {
            installation_id,
            listen,
            database_url,
            vault_key,
            auth_issuer,
            public_origin,
            admin_origin,
            external_calls,
            max_body_bytes,
            maintenance_tick_seconds,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn http_is_allowed_only_for_literal_loopback() {
        for value in [
            "http://127.0.0.3:8192",
            "http://localhost:8101",
            "https://auth.example.test",
        ] {
            assert!(trusted_origin(value).is_ok(), "{value}");
        }
        for value in [
            "http://127.0.0.3.evil.test",
            "http://192.168.1.1",
            "http://user:secret@localhost",
            "https://auth.test/?token=private",
            "file:///tmp",
        ] {
            assert!(trusted_origin(value).is_err(), "{value}");
        }
    }
}
