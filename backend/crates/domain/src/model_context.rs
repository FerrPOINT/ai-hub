use crate::error::HubError;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelContextInput {
    #[schema(min_length = 1, max_length = 256)]
    pub model_id: String,
    #[schema(minimum = 1, maximum = 4294967295u64,format=Int64)]
    pub context_window_tokens: u32,
}
impl ModelContextInput {
    pub fn validate(&self) -> Result<(), HubError> {
        validate_model_id(&self.model_id)?;
        if self.context_window_tokens == 0 {
            return Err(HubError::Invalid("configured context budget"));
        }
        Ok(())
    }
}
pub fn validate_model_id(model: &str) -> Result<(), HubError> {
    if model.is_empty() || model.len() > 256 || model.chars().any(char::is_control) {
        return Err(HubError::Invalid("exact model ID"));
    }
    Ok(())
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelContextPreference {
    pub connection_id: Uuid,
    #[schema(min_length = 1, max_length = 256)]
    pub model_id: String,
    #[schema(minimum=1,maximum=4294967295u64,format=Int64)]
    pub context_window_tokens: u32,
    pub version: i64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelContextMutation {
    pub operation_id: Uuid,
    pub preference: ModelContextPreference,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModelContextSnapshot {
    pub revision_id: Option<Uuid>,
    pub version: i64,
    pub context_window_tokens: Option<u32>,
}
