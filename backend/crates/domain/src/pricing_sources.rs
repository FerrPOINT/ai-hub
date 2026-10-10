use crate::{error::HubError, financial::Currency};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PricingMode {
    Manual,
    ProviderAuto,
}
impl PricingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::ProviderAuto => "provider_auto",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PricingSourceInput {
    pub connection_id: Uuid,
    pub model_id: String,
    pub currency: Currency,
    pub mode: PricingMode,
    #[serde(deserialize_with = "required_nullable")]
    #[schema(required = true)]
    pub manual_price_revision_id: Option<Uuid>,
    pub expected_version: i64,
    pub effective_from: DateTime<Utc>,
    #[serde(deserialize_with = "required_nullable")]
    #[schema(required = true)]
    pub effective_to: Option<DateTime<Utc>>,
}
fn required_nullable<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d)
}
impl PricingSourceInput {
    pub fn validate(&self) -> Result<(), HubError> {
        if self.connection_id.is_nil()
            || self.model_id.is_empty()
            || self.model_id.len() > 120
            || self.expected_version < 0
            || self
                .effective_to
                .is_some_and(|end| end <= self.effective_from)
            || match self.mode {
                PricingMode::Manual => self.manual_price_revision_id.is_none_or(|id| id.is_nil()),
                PricingMode::ProviderAuto => self.manual_price_revision_id.is_some(),
            }
        {
            return Err(HubError::Invalid("pricing source configuration"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PricingDataStatus {
    Complete,
    Stale,
    Unavailable,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PricingSourceRevision {
    pub id: Uuid,
    pub config: PricingSourceInput,
    pub version: i64,
    pub actor_subject: String,
    pub created_at: DateTime<Utc>,
    pub catalog_observed_at: Option<DateTime<Utc>>,
    pub data_status: PricingDataStatus,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingSourceMutation {
    pub operation_id: Uuid,
    pub value: PricingSourceRevision,
}
/// Internal frozen resolution; an expired revision keeps its identity with an unknown quote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingResolution {
    pub source_revision_id: Option<Uuid>,
    pub policy_version: i64,
    pub price_revision_id: Option<Uuid>,
    pub tier: Option<String>,
    pub as_of: DateTime<Utc>,
    pub data_status: PricingDataStatus,
}
