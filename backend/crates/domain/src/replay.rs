use crate::error::HubError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::Zeroizing;

pub const MAX_RESULT_BYTES: usize = 2 * 1024 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayProtocol {
    ChatCompletions,
    Responses,
}
impl ReplayProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ChatCompletions => "chat_completions",
            Self::Responses => "responses",
        }
    }
    pub fn from_storage(value: &str) -> Result<Self, HubError> {
        match value {
            "chat_completions" => Ok(Self::ChatCompletions),
            "responses" => Ok(Self::Responses),
            _ => Err(HubError::Unavailable),
        }
    }
}
/// Internal adapter output. S3 owns protocol normalization; bytes are never caller metadata.
/// Deliberately has no Debug/Serialize implementation.
pub struct ResultPayload {
    pub protocol: ReplayProtocol,
    pub status_code: u16,
    pub body: Zeroizing<Vec<u8>>,
}
impl ResultPayload {
    pub fn validate(&self) -> Result<(), HubError> {
        if !(200..=599).contains(&self.status_code)
            || self.body.is_empty()
            || self.body.len() > MAX_RESULT_BYTES
            || self.body.iter().copied().find(|b| !b.is_ascii_whitespace()) != Some(b'{')
            || serde_json::from_slice::<serde::de::IgnoredAny>(&self.body).is_err()
        {
            return Err(HubError::Invalid("bounded JSON result"));
        }
        Ok(())
    }
}
/// Constructed by verified application authentication, not deserialized from request JSON.
pub struct ResultReader {
    pub client_id: Uuid,
    pub principal_id: String,
    pub grant_id: Uuid,
}
pub enum ReplayOutcome {
    Available(ResultPayload),
    InProgress,
    Unknown,
    StreamUnavailable,
    Expired,
    NotStored,
}
