//! Private result delivery; no provider I/O and no plaintext audit/operation fields.
use crate::{
    financial::db_failure,
    postgres::PgStore,
    vault::{Sealed, Vault},
};
use aihub_application::{OperationBinding, ResultDelivery};
use aihub_domain::{
    error::HubError,
    replay::{MAX_RESULT_BYTES, ReplayOutcome, ReplayProtocol, ResultPayload, ResultReader},
    settlement::{SettlementFact, SettlementReceipt},
};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
use std::sync::Arc;
use uuid::Uuid;

pub struct ProtectedResults {
    store: Arc<PgStore>,
    vault: Arc<Vault>,
}
pub(crate) struct EncryptedResult {
    pub protocol: ReplayProtocol,
    pub status: u16,
    pub binding: [u8; 32],
    pub sealed: Sealed,
    pub key_id: String,
    pub ttl: u32,
}
fn purpose(protocol: ReplayProtocol, status: u16) -> String {
    format!("result:{}:{status}", protocol.as_str())
}
impl ProtectedResults {
    pub fn new(store: Arc<PgStore>, vault: Arc<Vault>) -> Result<Self, HubError> {
        if store.vault_fingerprint != vault.fingerprint() {
            return Err(HubError::InstallationMismatch);
        }
        Ok(Self { store, vault })
    }
    fn binding(&self, request: Uuid, payload: &ResultPayload) -> Result<[u8; 32], HubError> {
        self.vault.bind(&serde_json::json!({"purpose":"result","installation":self.store.installation_id,"request":request,"protocol":payload.protocol,"status":payload.status_code,"bytes_sha256":hex::encode(Sha256::digest(&payload.body))}))
    }
}

pub(crate) async fn check_existing(
    tx: &mut Transaction<'_, Postgres>,
    request: Uuid,
    result: &EncryptedResult,
) -> Result<bool, HubError> {
    let existing = sqlx::query(
        "SELECT protocol,status_code,body_binding FROM replay_receipts WHERE request_id=$1",
    )
    .bind(request)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| db_failure(e, line!()))?;
    if let Some(row) = existing {
        if row.get::<String, _>("protocol") != result.protocol.as_str()
            || row.get::<i32, _>("status_code") != i32::from(result.status)
            || row.get::<Vec<u8>, _>("body_binding").as_slice() != result.binding
        {
            return Err(HubError::IdempotencyConflict);
        }
        return Ok(true);
    }
    Ok(false)
}
pub(crate) async fn persist(
    tx: &mut Transaction<'_, Postgres>,
    installation: Uuid,
    request: Uuid,
    result: &EncryptedResult,
) -> Result<(), HubError> {
    if check_existing(tx, request, result).await? {
        return Ok(());
    }
    // First receipt and first ciphertext are inseparable from settlement commit.
    sqlx::query("INSERT INTO replay_receipts(request_id,installation_id,protocol,status_code,body_binding,expires_at) VALUES($1,$2,$3,$4,$5,now()+make_interval(secs=>$6))")
        .bind(request).bind(installation).bind(result.protocol.as_str()).bind(i32::from(result.status)).bind(result.binding.as_slice()).bind(f64::from(result.ttl)).execute(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
    sqlx::query("INSERT INTO replay_payloads(request_id,encrypted_response,nonce,key_id,expires_at,protocol) SELECT request_id,$2,$3,$4,expires_at,protocol FROM replay_receipts WHERE request_id=$1")
        .bind(request).bind(&result.sealed.ciphertext).bind(result.sealed.nonce.as_slice()).bind(&result.key_id).execute(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
    Ok(())
}

#[async_trait]
impl ResultDelivery for ProtectedResults {
    async fn settle_with_result(
        &self,
        fact: &SettlementFact,
        payload: &ResultPayload,
        ttl_seconds: u32,
    ) -> Result<SettlementReceipt, HubError> {
        payload.validate()?;
        if !(1..=86400).contains(&ttl_seconds) {
            return Err(HubError::Invalid("result TTL"));
        }
        let request: Uuid = sqlx::query_scalar(
            "SELECT request_id FROM attempts WHERE installation_id=$1 AND id=$2",
        )
        .bind(self.store.installation_id)
        .bind(fact.attempt_id)
        .fetch_optional(&self.store.pool)
        .await
        .map_err(|e| db_failure(e, line!()))?
        .ok_or(HubError::NotFound)?;
        let result = EncryptedResult {
            protocol: payload.protocol,
            status: payload.status_code,
            binding: self.binding(request, payload)?,
            sealed: self.vault.seal(
                self.store.installation_id,
                request,
                1,
                &purpose(payload.protocol, payload.status_code),
                &payload.body,
            )?,
            key_id: hex::encode(self.vault.fingerprint()),
            ttl: ttl_seconds,
        };
        self.store.settle_fact_and_result(fact, Some(&result)).await
    }
    async fn read_result(
        &self,
        reader: &ResultReader,
        request: Uuid,
    ) -> Result<ReplayOutcome, HubError> {
        if request.is_nil()
            || reader.client_id.is_nil()
            || reader.grant_id.is_nil()
            || reader.principal_id.is_empty()
        {
            return Err(HubError::Invalid("result owner"));
        }
        let mut tx = self
            .store
            .pool
            .begin()
            .await
            .map_err(|e| db_failure(e, line!()))?;
        // Lock client and current grant through decryption. Metadata/expense grants do not suffice.
        sqlx::query("SELECT id FROM clients WHERE installation_id=$1 AND id=$2 AND status='enabled' AND expires_at>clock_timestamp() AND scopes @> '[\"read_result\"]' FOR SHARE")
            .bind(self.store.installation_id).bind(reader.client_id).fetch_optional(&mut *tx).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::Forbidden)?;
        let context=sqlx::query("SELECT project_binding,namespace_binding_id FROM requests WHERE installation_id=$1 AND id=$2 AND client_id=$3 AND principal_id=$4")
            .bind(self.store.installation_id).bind(request).bind(reader.client_id).bind(&reader.principal_id).fetch_optional(&mut *tx).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::NotFound)?;
        sqlx::query("SELECT id FROM grants WHERE installation_id=$1 AND id=$2 AND client_id=$3 AND principal_id=$4 AND project_binding=$5 AND namespace_binding_id IS NOT DISTINCT FROM $6 AND action='read_result' AND revoked_at IS NULL AND expires_at>clock_timestamp() FOR SHARE")
            .bind(self.store.installation_id).bind(reader.grant_id).bind(reader.client_id).bind(&reader.principal_id).bind(context.get::<String,_>("project_binding")).bind(context.get::<Option<Uuid>,_>("namespace_binding_id")).fetch_optional(&mut *tx).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::Forbidden)?;
        let row=sqlx::query("SELECT r.*,p.status_code,p.protocol AS result_protocol,p.body_binding,p.expires_at>clock_timestamp() AS valid,bytes.encrypted_response,bytes.nonce,bytes.key_id FROM requests r LEFT JOIN replay_receipts p ON p.request_id=r.id AND p.installation_id=r.installation_id LEFT JOIN replay_payloads bytes ON bytes.request_id=p.request_id WHERE r.installation_id=$1 AND r.id=$2 FOR SHARE OF r")
            .bind(self.store.installation_id).bind(request).fetch_one(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        let state: String = row.get("state");
        if state == "unknown" {
            return Ok(ReplayOutcome::Unknown);
        }
        if !matches!(state.as_str(), "completed" | "failed" | "cancelled") {
            return Ok(ReplayOutcome::InProgress);
        }
        if row.get::<Option<bool>, _>("streaming") == Some(true) {
            return Ok(ReplayOutcome::StreamUnavailable);
        }
        let Some(protocol) = row.get::<Option<String>, _>("result_protocol") else {
            return Ok(ReplayOutcome::NotStored);
        };
        if row.get::<Option<bool>, _>("valid") != Some(true) {
            return Ok(ReplayOutcome::Expired);
        }
        let Some(ciphertext) = row.get::<Option<Vec<u8>>, _>("encrypted_response") else {
            return Err(HubError::Unavailable);
        };
        let protocol = ReplayProtocol::from_storage(&protocol)?;
        if row.get::<Option<String>, _>("wire_protocol").as_deref() != Some(protocol.as_str())
            || row.get::<Option<bool>, _>("streaming") != Some(false)
        {
            return Err(HubError::InstallationMismatch);
        }
        let status: u16 = row
            .get::<i32, _>("status_code")
            .try_into()
            .map_err(|_| HubError::Unavailable)?;
        if row.get::<String, _>("key_id") != hex::encode(self.vault.fingerprint())
            || ciphertext.len() > MAX_RESULT_BYTES + 16
        {
            return Err(HubError::InstallationMismatch);
        }
        let nonce: Vec<u8> = row.get("nonce");
        let payload = ResultPayload {
            protocol,
            status_code: status,
            body: self.vault.open(
                self.store.installation_id,
                request,
                1,
                &purpose(protocol, status),
                &Sealed {
                    nonce: nonce.try_into().map_err(|_| HubError::Unavailable)?,
                    ciphertext,
                },
            )?,
        };
        payload.validate()?;
        if row.get::<Vec<u8>, _>("body_binding").as_slice() != self.binding(request, &payload)? {
            return Err(HubError::InstallationMismatch);
        }
        sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,namespace_binding_id,reason) VALUES($1,$2,$3,'request.result.read',$4,$5,$6,'Authorized encrypted result delivery')")
            .bind(Uuid::new_v4()).bind(self.store.installation_id).bind(&reader.principal_id).bind(request.to_string()).bind(row.get::<Uuid,_>("operation_id")).bind(row.get::<Option<Uuid>,_>("namespace_binding_id")).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        tx.commit().await.map_err(|e| db_failure(e, line!()))?;
        Ok(ReplayOutcome::Available(payload))
    }
    async fn purge_expired_results(&self, limit: i64) -> Result<u64, HubError> {
        if !(1..=1000).contains(&limit) {
            return Err(HubError::Invalid("result purge limit"));
        }
        // Includes preserved legacy ciphertext using its original TTL, never deletes dedupe receipts.
        sqlx::query("DELETE FROM replay_payloads WHERE request_id IN (SELECT p.request_id FROM replay_payloads p JOIN requests r ON r.id=p.request_id WHERE r.installation_id=$1 AND p.expires_at<=now() ORDER BY p.expires_at,p.request_id LIMIT $2 FOR UPDATE OF p SKIP LOCKED)")
            .bind(self.store.installation_id).bind(limit).execute(&self.store.pool).await.map(|r|r.rows_affected()).map_err(|e|db_failure(e,line!()))
    }
}
