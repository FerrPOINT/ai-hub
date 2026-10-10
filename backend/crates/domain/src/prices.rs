use crate::{
    error::HubError,
    financial::{Amount, Currency, Rate},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PriceUnit {
    PerMillionTokens,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PriceInput {
    pub connection_id: Uuid,
    pub model_id: String,
    pub tier: String,
    pub currency: Currency,
    pub unit: PriceUnit,
    pub input_uncached: Rate,
    #[serde(deserialize_with = "required_nullable")]
    #[schema(required = true)]
    pub input_cached: Option<Rate>,
    pub output_billable: Rate,
    pub effective_from: DateTime<Utc>,
    #[serde(deserialize_with = "required_nullable")]
    #[schema(required = true)]
    pub effective_to: Option<DateTime<Utc>>,
    pub source: String,
    /// Missing fee stays unknown, never a default zero.
    #[serde(default)]
    pub request_fee: Option<Amount>,
}

fn required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_nullable_rates_are_required_and_missing_fee_stays_unknown() {
        let input = serde_json::json!({"connection_id":Uuid::new_v4(),"model_id":"fixture-model","tier":"metered","currency":"USD","unit":"per_million_tokens","input_uncached":"2","input_cached":null,"output_billable":"8","effective_from":"2026-10-10T00:00:00Z","effective_to":null,"source":"synthetic-fixture"});
        let valid: PriceInput = serde_json::from_value(input.clone()).unwrap();
        assert!(valid.request_fee.is_none());
        assert!(valid.validate().is_ok());
        for field in ["input_cached", "effective_to"] {
            let mut missing = input.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<PriceInput>(missing).is_err());
        }
        let mut numeric = input.clone();
        numeric["input_uncached"] = serde_json::json!(2.0);
        assert!(serde_json::from_value::<PriceInput>(numeric).is_err());
    }
}
impl PriceInput {
    pub fn validate(&self) -> Result<(), HubError> {
        if self.connection_id.is_nil()
            || self.model_id.is_empty()
            || self.model_id.len() > 256
            || self.tier.is_empty()
            || self.tier.len() > 120
            || self.source.is_empty()
            || self.source.len() > 256
            || self
                .effective_to
                .is_some_and(|end| end <= self.effective_from)
        {
            return Err(HubError::Invalid("price quote"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PriceRevision {
    pub id: Uuid,
    pub price: PriceInput,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PriceMutation {
    pub operation_id: Uuid,
    pub value: PriceRevision,
}
