use crate::NamespaceRef;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub subject: String,
    pub installation_id: Uuid,
    pub capabilities: Vec<String>,
    pub project_grants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NamespaceBinding {
    pub namespace: NamespaceRef,
    pub tracker_instance_id: Uuid,
    pub tracker_project_id: Uuid,
    pub state: String,
    pub observed_at: DateTime<Utc>,
    pub generation: i64,
    pub label: String,
    pub tracker_project_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AuditEvent {
    pub id: Uuid,
    pub actor: String,
    pub action: String,
    pub object_id: String,
    pub revision: Option<String>,
    pub operation_id: Uuid,
    pub reason: Option<String>,
    pub happened_at: DateTime<Utc>,
    pub namespace: Option<NamespaceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub id: Uuid,
    pub status: String,
    pub resource_id: Option<Uuid>,
    pub safe_error: Option<String>,
    pub version: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Health {
    pub status: String,
    pub schema_revision: Option<String>,
}
