use crate::{
    NamespaceRef,
    error::HubError,
    financial::{Adjustment, Amount, Currency},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BudgetScope {
    Installation,
    Project,
    Client,
    Profile,
}
impl BudgetScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Installation => "installation",
            Self::Project => "project",
            Self::Client => "client",
            Self::Profile => "profile",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BudgetPeriod {
    UtcDay,
    UtcMonth,
}
impl BudgetPeriod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UtcDay => "utc_day",
            Self::UtcMonth => "utc_month",
        }
    }
    pub fn grain(self) -> &'static str {
        match self {
            Self::UtcDay => "day",
            Self::UtcMonth => "month",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BudgetInput {
    pub scope_type: BudgetScope,
    pub scope_id: Uuid,
    pub currency: Currency,
    pub period: BudgetPeriod,
    pub hard_limit: Amount,
    pub warning_thresholds: Vec<u8>,
    #[serde(deserialize_with = "required_namespace")]
    #[schema(required = true)]
    pub namespace: Option<NamespaceRef>,
}
fn required_namespace<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<NamespaceRef>, D::Error> {
    Option::<NamespaceRef>::deserialize(deserializer)
}
impl BudgetInput {
    pub fn validate(&self, installation: Uuid) -> Result<(), HubError> {
        if self.scope_id.is_nil()
            || self.warning_thresholds.is_empty()
            || self.warning_thresholds.len() > 5
            || self
                .warning_thresholds
                .iter()
                .any(|n| !(1..=99).contains(n))
            || self
                .warning_thresholds
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != self.warning_thresholds.len()
        {
            return Err(HubError::Invalid("budget policy"));
        }
        match (self.scope_type, &self.namespace) {
            (BudgetScope::Project, Some(namespace))
                if !namespace.registry_instance_id.is_nil()
                    && self.scope_id == namespace.namespace_id =>
            {
                Ok(())
            }
            (BudgetScope::Installation, None) if self.scope_id == installation => Ok(()),
            (BudgetScope::Client | BudgetScope::Profile, None) => Ok(()),
            _ => Err(HubError::Invalid("budget scope/Namespace mismatch")),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Budget {
    pub id: Uuid,
    pub policy: BudgetInput,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub charged: Amount,
    pub reserved: Amount,
    pub remaining: Adjustment,
    pub version: i64,
    pub namespace: Option<NamespaceRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetMutation {
    pub operation_id: Uuid,
    pub value: Budget,
}
