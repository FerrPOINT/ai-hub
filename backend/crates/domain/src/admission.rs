use crate::{
    error::HubError,
    financial::{Amount, Currency, Usage},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    Inference,
    Verification,
    Evaluation,
}
impl Purpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inference => "inference",
            Self::Verification => "verification",
            Self::Evaluation => "evaluation",
        }
    }
    pub fn action(self) -> &'static str {
        match self {
            Self::Inference => "infer",
            Self::Verification => "verification",
            Self::Evaluation => "evaluation",
        }
    }
}
/// Internal frozen context, never deserialized from public caller metadata.
#[derive(Clone)]
pub struct AdmissionIntent {
    pub client_id: Uuid,
    pub grant_id: Uuid,
    pub principal_id: String,
    pub purpose: Purpose,
    pub profile_revision_id: Option<Uuid>,
    pub probe_snapshot_id: Option<Uuid>,
    pub idempotency_key: Uuid,
    pub payload_hmac: [u8; 32],
    pub connection_id: Uuid,
    pub generation: i64,
    pub model_id: String,
    pub tier: String,
    pub price_revision_id: Option<Uuid>,
    pub qualification_id: Uuid,
    pub upper_usage: Usage,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PurposeBounds {
    pub connection_id: Uuid,
    pub generation: i64,
    pub model_id: String,
    pub currency: Currency,
    pub max_input_tokens: u64,
    pub max_output_tokens: u64,
    pub max_requests: u64,
    pub max_concurrency: u32,
    pub max_total_provider_cost: Option<Amount>,
    pub cost_unknown_allowed: bool,
}
impl PurposeBounds {
    pub fn validate(&self) -> Result<(), HubError> {
        if self.connection_id.is_nil()
            || self.generation < 1
            || self.model_id.is_empty()
            || self.model_id.len() > 256
            || self.max_input_tokens == 0
            || self.max_input_tokens > 100000000
            || self.max_output_tokens == 0
            || self.max_output_tokens > 10000000
            || self.max_requests == 0
            || self.max_requests > 1000000
            || !(1..=100).contains(&self.max_concurrency)
            || (!self.cost_unknown_allowed && self.max_total_provider_cost.is_none())
        {
            return Err(HubError::Invalid("purpose grant bounds"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmissionReceipt {
    pub request_id: Uuid,
    pub attempt_id: Uuid,
    pub state: String,
    pub replay: bool,
    pub upper_provider_cost: Option<Amount>,
    pub currency: Currency,
}

/// Internal dispatch capability. A committed claim permits exactly one transport send.
#[derive(Debug)]
pub struct DispatchClaim {
    pub attempt_id: Uuid,
    pub owner_id: Uuid,
    pub fence: Uuid,
    pub deployment_snapshot: serde_json::Value,
}
