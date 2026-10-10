use crate::financial::{Acceptance, Amount, Currency, Usage};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminalState {
    Completed,
    Failed,
    Cancelled,
}
impl TerminalState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Adapter-owned receipt, never accepted from inference caller metadata.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderCharge {
    pub amount: Amount,
    pub currency: Currency,
    pub external_id: String,
    pub digest: String,
}

/// Internal terminal/reconciliation fact. Unknown acceptance keeps its reserve.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SettlementAuthority {
    Dispatch {
        owner_id: Uuid,
        fence: Uuid,
    },
    BeforeDispatchCancellation {
        cancellation: crate::admission::RequestOwner,
    },
}
#[derive(Debug, Clone, Serialize)]
pub struct SettlementFact {
    pub attempt_id: Uuid,
    #[serde(flatten)]
    pub authority: SettlementAuthority,
    pub source: String,
    pub source_event_id: String,
    pub acceptance: Acceptance,
    pub terminal: TerminalState,
    pub usage: Option<Usage>,
    pub receipt: Option<ProviderCharge>,
}

#[derive(Debug, Clone)]
pub struct SettlementReceipt {
    pub ledger_id: Uuid,
    pub amount: Option<Amount>,
    pub confidence: String,
    pub duplicate: bool,
}
