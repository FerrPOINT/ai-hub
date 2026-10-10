use crate::{error::HubError, model_context::validate_model_id};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProfileMode {
    Development,
    PinnedTest,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DeploymentInput {
    pub connection_id: Uuid,
    #[schema(minimum = 1)]
    pub generation: i64,
    #[schema(min_length = 1, max_length = 256)]
    pub model_id: String,
}
fn optional_parameter<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerationParameters {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_parameter"
    )]
    #[schema(minimum = 0, maximum = 2, nullable = false)]
    pub temperature: Option<f64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_parameter"
    )]
    #[schema(exclusive_minimum = 0, maximum = 1, nullable = false)]
    pub top_p: Option<f64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_parameter"
    )]
    #[schema(min_length = 1, max_length = 32, nullable = false)]
    pub reasoning_effort: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Text,
    Stream,
    FunctionTools,
    JsonSchema,
    Responses,
    Cancel,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallerOverride {
    Temperature,
    TopP,
    MaxOutputTokens,
    ReasoningEffort,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfileInput {
    #[schema(min_length = 2, max_length = 63, pattern = "^[a-z][a-z0-9-]{1,62}$")]
    pub slug: String,
    #[schema(min_length = 1, max_length = 120)]
    pub display_name: String,
    pub mode: ProfileMode,
    #[schema(min_items = 1, max_items = 5)]
    pub deployments: Vec<DeploymentInput>,
    pub parameters: GenerationParameters,
    #[schema(minimum = 1,maximum=4294967295u64,format=Int64)]
    pub input_limit: u32,
    #[schema(minimum = 1,maximum=4294967295u64,format=Int64)]
    pub output_limit: u32,
    #[schema(minimum = 1,maximum=4294967295u64,format=Int64)]
    pub context_limit: u32,
    pub required_capabilities: Vec<Capability>,
    #[schema(minimum = 1, maximum = 600)]
    pub timeout_seconds: u16,
    #[schema(minimum = 1, maximum = 5)]
    pub max_attempts: u8,
    #[serde(default)]
    pub allowed_overrides: Vec<CallerOverride>,
}
impl ProfileInput {
    pub fn validate(&self) -> Result<(), HubError> {
        let slug = self.slug.as_bytes();
        if !(2..=63).contains(&slug.len())
            || !slug[0].is_ascii_lowercase()
            || slug
                .iter()
                .any(|c| !c.is_ascii_lowercase() && !c.is_ascii_digit() && *c != b'-')
            || self.display_name.trim().is_empty()
            || self.display_name.chars().count() > 120
        {
            return Err(HubError::Invalid("profile identity"));
        }
        if self.deployments.is_empty()
            || self.deployments.len() > 5
            || self.timeout_seconds == 0
            || self.timeout_seconds > 600
            || self.max_attempts == 0
            || self.max_attempts > 5
            || self.input_limit == 0
            || self.output_limit == 0
            || self.context_limit == 0
            || self.input_limit > self.context_limit
            || self.output_limit > self.context_limit
        {
            return Err(HubError::Invalid("profile bounds"));
        }
        if self.mode == ProfileMode::PinnedTest
            && (self.deployments.len() != 1 || self.max_attempts != 1)
        {
            return Err(HubError::InvalidSemantics(
                "pinned profile requires one target and one attempt",
            ));
        }
        for (ordinal, target) in self.deployments.iter().enumerate() {
            if target.connection_id.is_nil() || target.generation < 1 {
                return Err(HubError::Invalid("profile deployment"));
            }
            validate_model_id(&target.model_id)?;
            if self.deployments[..ordinal].contains(target) {
                return Err(HubError::InvalidSemantics(
                    "duplicate deployment is an implicit upstream retry",
                ));
            }
        }
        if self.required_capabilities.len() > 6
            || self
                .required_capabilities
                .iter()
                .enumerate()
                .any(|(i, c)| self.required_capabilities[..i].contains(c))
            || self.allowed_overrides.len() > 4
            || self
                .allowed_overrides
                .iter()
                .enumerate()
                .any(|(i, c)| self.allowed_overrides[..i].contains(c))
        {
            return Err(HubError::Invalid("profile capability or override set"));
        }
        let p = &self.parameters;
        if p.temperature
            .is_some_and(|v| !v.is_finite() || !(0.0..=2.0).contains(&v))
            || p.top_p
                .is_some_and(|v| !v.is_finite() || v <= 0.0 || v > 1.0)
            || p.reasoning_effort
                .as_ref()
                .is_some_and(|v| v.is_empty() || v.len() > 32 || v.chars().any(char::is_control))
        {
            return Err(HubError::Invalid("generation parameters"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub id: Uuid,
    pub draft: ProfileInput,
    #[schema(minimum = 1)]
    pub draft_version: i64,
    #[schema(required = true)]
    pub active_revision_id: Option<Uuid>,
    pub status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftMutation {
    pub operation_id: Uuid,
    pub profile: Profile,
}
#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> ProfileInput {
        serde_json::from_value(serde_json::json!({"slug":"model-api","display_name":"same label","mode":"development","deployments":[{"connection_id":Uuid::new_v4(),"generation":1,"model_id":"vendor/Exact"}],"parameters":{"temperature":0.4,"top_p":1.0},"input_limit":1000,"output_limit":100,"context_limit":1200,"required_capabilities":["text"],"timeout_seconds":120,"max_attempts":3})).unwrap()
    }
    #[test]
    fn full_draft_preserves_exact_fields_and_rejects_unsafe_routing() {
        let p = sample();
        p.validate().unwrap();
        assert!(p.allowed_overrides.is_empty());
        let mut duplicate = p.clone();
        duplicate.deployments.push(p.deployments[0].clone());
        assert!(duplicate.validate().is_err());
        let mut pinned = p.clone();
        pinned.mode = ProfileMode::PinnedTest;
        assert!(pinned.validate().is_err());
        pinned.max_attempts = 1;
        pinned.validate().unwrap();
        let mut nan = p.clone();
        nan.parameters.temperature = Some(f64::NAN);
        assert!(nan.validate().is_err());
        let mut changed = p.clone();
        changed.deployments[0].model_id = "vendor/exact".into();
        changed.validate().unwrap();
        assert_ne!(p.deployments, changed.deployments);
        assert!(
            serde_json::from_value::<GenerationParameters>(serde_json::json!({"temperature":null}))
                .is_err()
        );
        assert!(
            serde_json::from_value::<GenerationParameters>(serde_json::json!({"top_p":null}))
                .is_err()
        );
        assert!(
            serde_json::from_value::<GenerationParameters>(
                serde_json::json!({"reasoning_effort":null})
            )
            .is_err()
        );
        let mut zero = p.clone();
        zero.parameters.top_p = Some(0.0);
        assert!(zero.validate().is_err());
        let absent: GenerationParameters = serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(absent.temperature.is_none());
    }
}
