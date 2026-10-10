use crate::{financial::db_failure, postgres::PgStore};
use aihub_domain::{
    admission::Purpose, error::HubError, financial::Currency, model_context::ModelContextSnapshot,
};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

pub(crate) enum TargetAuthorityId {
    Model(Uuid),
    Account(Uuid),
}
impl TargetAuthorityId {
    pub(crate) fn from_ids(model: Option<Uuid>, account: Option<Uuid>) -> Result<Self, HubError> {
        match (model, account) {
            (Some(id), None) if !id.is_nil() => Ok(Self::Model(id)),
            (None, Some(id)) if !id.is_nil() => Ok(Self::Account(id)),
            _ => Err(HubError::Invalid("exactly one target authority required")),
        }
    }
}
pub(crate) struct TargetAuthority {
    pub currency: Currency,
    pub adapter_revision: String,
    pub endpoint_hash: String,
    pub capabilities: serde_json::Value,
    pub model_context: ModelContextSnapshot,
    pub scope: &'static str,
}
impl PgStore {
    pub(crate) async fn target_authority_locked(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        connection: Uuid,
        generation: i64,
        model: &str,
        id: TargetAuthorityId,
        purpose: Purpose,
    ) -> Result<TargetAuthority, HubError> {
        match id {
            TargetAuthorityId::Model(id) => {
                let row=sqlx::query("SELECT q.billing_currency,q.adapter_revision,q.endpoint_policy_hash,q.capabilities,q.model_context_version,q.model_context_revision_id FROM runtime_qualifications q JOIN connections c ON c.installation_id=q.installation_id AND c.id=q.connection_id JOIN connection_generations g ON g.connection_id=q.connection_id AND g.generation=q.generation JOIN verification_evidence e ON e.installation_id=q.installation_id AND e.id=q.proof_id WHERE q.installation_id=$1 AND q.id=$2 AND q.connection_id=$3 AND q.generation=$4 AND q.provider_model_id=$5 AND q.state='active' AND q.billing_currency IS NOT NULL AND q.billing_currency_origin IS NOT NULL AND c.status='enabled' AND c.generation=q.generation AND g.authorization_state='active' AND g.adapter_revision=q.adapter_revision AND g.endpoint_policy_hash=q.endpoint_policy_hash AND e.state='verified' AND e.expires_at>clock_timestamp() FOR SHARE OF q,c,g,e")
                    .bind(self.installation_id).bind(id).bind(connection).bind(generation).bind(model).fetch_optional(&mut **tx).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::Forbidden)?;
                let context = self
                    .qualified_model_context_locked(tx, connection, model, &row)
                    .await?;
                Ok(TargetAuthority {
                    currency: Currency::parse(&row.get::<String, _>("billing_currency"))?,
                    adapter_revision: row.get("adapter_revision"),
                    endpoint_hash: row.get("endpoint_policy_hash"),
                    capabilities: row.get("capabilities"),
                    model_context: context,
                    scope: "connection_model",
                })
            }
            TargetAuthorityId::Account(id) => {
                if purpose != Purpose::Verification {
                    return Err(HubError::Forbidden);
                }
                let row=sqlx::query("SELECT a.currency,a.adapter_revision,a.endpoint_policy_hash,a.billing_capabilities FROM verification_account_authorities a JOIN connections c ON c.installation_id=a.installation_id AND c.id=a.connection_id JOIN connection_generations g ON g.connection_id=c.id AND g.generation=c.generation JOIN credential_versions v ON v.connection_id=c.id AND v.generation=c.generation WHERE a.installation_id=$1 AND a.id=$2 AND a.connection_id=$3 AND a.generation=$4 AND c.generation=a.generation AND c.status IN ('authorization_unknown','enabled') AND g.authorization_state IN ('prepared','active') AND v.state IN ('prepared','active') AND a.state='active' AND a.observed_at<=clock_timestamp() AND a.expires_at>clock_timestamp() AND a.adapter_revision=g.adapter_revision AND a.endpoint_policy_hash=g.endpoint_policy_hash AND a.billing_tier=g.billing_tier FOR SHARE OF a,c,g,v")
                    .bind(self.installation_id).bind(id).bind(connection).bind(generation).fetch_optional(&mut **tx).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::Forbidden)?;
                Ok(TargetAuthority {
                    currency: Currency::parse(&row.get::<String, _>("currency"))?,
                    adapter_revision: row.get("adapter_revision"),
                    endpoint_hash: row.get("endpoint_policy_hash"),
                    capabilities: row.get("billing_capabilities"),
                    model_context: self
                        .model_context_snapshot_locked(tx, connection, model)
                        .await?,
                    scope: "account_only",
                })
            }
        }
    }
}
