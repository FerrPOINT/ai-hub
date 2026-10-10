use crate::error::HubError;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenaiCompatible,
    Ollama,
    Zai,
    ChatgptManaged,
}
impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenaiCompatible => "openai_compatible",
            Self::Ollama => "ollama",
            Self::Zai => "zai",
            Self::ChatgptManaged => "chatgpt_managed",
        }
    }
    pub fn parse(value: &str) -> Result<Self, HubError> {
        match value {
            "openai_compatible" => Ok(Self::OpenaiCompatible),
            "ollama" => Ok(Self::Ollama),
            "zai" => Ok(Self::Zai),
            "chatgpt_managed" => Ok(Self::ChatgptManaged),
            _ => Err(HubError::Invalid("provider kind")),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BillingMode {
    Metered,
    Subscription,
    Local,
    Unknown,
}
impl BillingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Metered => "metered",
            Self::Subscription => "subscription",
            Self::Local => "local",
            Self::Unknown => "unknown",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectionInput {
    pub display_name: String,
    pub endpoint_policy_ref: String,
    pub billing_mode: BillingMode,
}
impl ConnectionInput {
    pub fn validate(&self) -> Result<(), HubError> {
        if self.display_name.trim().is_empty()
            || self.display_name.chars().count() > 120
            || self.endpoint_policy_ref.is_empty()
            || self.endpoint_policy_ref.len() > 120
            || !self
                .endpoint_policy_ref
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        {
            return Err(HubError::Invalid("connection settings"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Connection {
    pub id: Uuid,
    pub provider_kind: ProviderKind,
    pub display_name: String,
    pub generation: i64,
    pub status: String,
    pub has_credentials: bool,
    #[schema(required = true)]
    pub quota: Option<serde_json::Value>,
    pub version: i64,
    pub settings: ConnectionInput,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMutation {
    pub operation_id: Uuid,
    pub value: Connection,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CredentialInput {
    #[serde(deserialize_with = "secret_string")]
    #[schema(value_type=String,write_only=true,min_length=1,max_length=16384)]
    pub secret: zeroize::Zeroizing<String>,
    pub credential_type: CredentialType,
    pub expected_generation: i64,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CredentialType {
    ApiKey,
}
fn secret_string<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<zeroize::Zeroizing<String>, D::Error> {
    String::deserialize(d).map(zeroize::Zeroizing::new)
}
impl CredentialInput {
    pub fn validate(&self) -> Result<(), HubError> {
        if self.secret.is_empty()
            || self.secret.len() > 16384
            || !self.secret.bytes().all(|b| (33..=126).contains(&b))
            || self.expected_generation < 1
            || self.expected_generation == i64::MAX
        {
            return Err(HubError::Invalid("credential payload"));
        }
        Ok(())
    }
}
/// Internal encrypted material; no Debug/Serialize and no public read path.
pub struct ProtectedCredential {
    pub ciphertext: Vec<u8>,
    pub nonce: [u8; 12],
    pub key_id: String,
}
/// Operator-owned allowlist; never read from public connection body or caller metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointPolicyInput {
    pub policy_ref: String,
    pub provider_kind: ProviderKind,
    pub base_url: String,
    pub allow_loopback: bool,
}
impl EndpointPolicyInput {
    pub fn validate(&self) -> Result<(), HubError> {
        if self.policy_ref.is_empty()
            || self.policy_ref.len() > 120
            || !self
                .policy_ref
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        {
            return Err(HubError::Invalid("endpoint policy reference"));
        }
        let url = url::Url::parse(&self.base_url).map_err(|_| HubError::Invalid("endpoint URL"))?;
        let loopback = url.host_str().is_some_and(|host| {
            host == "localhost"
                || host
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        });
        if url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || self.base_url.len() > 2048
            || (url.scheme() != "https"
                && !(url.scheme() == "http" && loopback && self.allow_loopback))
            || (loopback && !self.allow_loopback)
        {
            return Err(HubError::Invalid("endpoint allowlist origin"));
        }
        Ok(())
    }
}
