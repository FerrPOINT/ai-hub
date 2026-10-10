use crate::financial::{Amount, Rate};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelMetadata {
    pub provider_model_id: String,
    #[schema(required = true)]
    pub input_limit: Option<i64>,
    #[schema(required = true)]
    pub output_limit: Option<i64>,
    pub capabilities: Vec<String>,
    pub evidence_status: String,
    pub observed_at: DateTime<Utc>,
}
/// Parsed catalog quote, not an account currency witness or capability proof.
pub struct CatalogModel {
    pub metadata: ModelMetadata,
    pub input_uncached: Option<Rate>,
    pub input_cached: Option<Rate>,
    pub cache_write: Option<Rate>,
    pub output_billable: Option<Rate>,
    pub request_fee: Option<Amount>,
}
pub struct CatalogObservation {
    pub models: Vec<CatalogModel>,
    pub observed_at: DateTime<Utc>,
    pub digest: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CatalogPage {
    pub connection_id: uuid::Uuid,
    pub generation: i64,
    pub models: Vec<ModelMetadata>,
    pub as_of: DateTime<Utc>,
    pub data_status: String,
    pub next_cursor: Option<String>,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MetadataRefreshInput {
    #[schema(minimum = 1)]
    pub expected_generation: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AccountAuthorityView {
    pub connection_id: uuid::Uuid,
    pub generation: i64,
    pub status: String,
    #[schema(required = true)]
    pub currency: Option<crate::financial::Currency>,
    #[schema(required = true)]
    pub statement_usage: Option<Amount>,
    #[schema(required = true)]
    pub observed_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub expires_at: Option<DateTime<Utc>>,
}
/// No Debug/Serialize: provider key metadata may contain credential fragments and account identities.
pub struct AccountObservation {
    pub limit: Option<Amount>,
    pub remaining: Option<Amount>,
    pub usage: Option<Amount>,
    pub byok_usage: Option<Amount>,
    pub is_free_tier: Option<bool>,
    pub is_management_key: Option<bool>,
    pub include_byok_in_limit: Option<bool>,
    pub expires_at: Option<DateTime<Utc>>,
    pub digest: String,
}
