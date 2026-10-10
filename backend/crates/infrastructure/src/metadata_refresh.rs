//! Durable, fenced metadata reads. This service never invokes a model endpoint.
use crate::{
    financial::db_failure,
    postgres::PgStore,
    vault::{Sealed, Vault},
};
use aihub_application::{FoundationStore, OperationBinding};
use aihub_domain::{
    catalog::{AccountObservation, CatalogObservation},
    error::HubError,
    records::Operation,
};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;
use zeroize::Zeroizing;

/// No Debug/Serialize; the capability contains a decrypted credential.
pub struct MetadataClaim {
    pub operation: Operation,
    connection_id: Uuid,
    generation: i64,
    endpoint_hash: String,
    fence: Uuid,
    secret: Zeroizing<Vec<u8>>,
}
pub enum MetadataStart {
    Claim(MetadataClaim),
    Readback(Operation),
}
pub struct MetadataRefresh {
    store: Arc<PgStore>,
    vault: Arc<Vault>,
    http: crate::metadata_http::MetadataHttp,
}
impl MetadataRefresh {
    pub fn new(store: Arc<PgStore>, vault: Arc<Vault>) -> Result<Self, HubError> {
        if store.vault_fingerprint != vault.fingerprint() {
            return Err(HubError::InstallationMismatch);
        }
        Ok(Self {
            store,
            vault,
            http: crate::metadata_http::MetadataHttp::new()?,
        })
    }
    pub async fn begin(
        &self,
        subject: &str,
        key: Uuid,
        connection: Uuid,
        expected_generation: i64,
    ) -> Result<MetadataStart, HubError> {
        if key.is_nil() || connection.is_nil() || expected_generation < 1 {
            return Err(HubError::Invalid("metadata operation"));
        }
        self.store.ready().await?;
        let binding=self.vault.bind(&serde_json::json!({"installation":self.store.installation_id,"principal":subject,"action":"catalog.refresh","connection":connection,"generation":expected_generation}))?;
        let mut tx = self
            .store
            .pool
            .begin()
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let (operation, replay) = self
            .store
            .begin_control_operation(&mut tx, subject, key, binding, "catalog.refresh")
            .await?;
        if let Some(replay) = replay {
            return Ok(MetadataStart::Readback(
                serde_json::from_value(replay).map_err(|_| HubError::Unavailable)?,
            ));
        }
        let row=sqlx::query("SELECT c.generation,c.status,p.kind,g.endpoint_policy_hash,g.endpoint_snapshot,v.ciphertext,v.nonce,v.key_id FROM connections c JOIN providers p ON p.installation_id=c.installation_id AND p.id=c.provider_id JOIN connection_generations g ON g.connection_id=c.id AND g.generation=c.generation JOIN credential_versions v ON v.connection_id=c.id AND v.generation=c.generation AND v.state IN ('prepared','active') WHERE c.installation_id=$1 AND c.id=$2 FOR SHARE OF c,g,v")
            .bind(self.store.installation_id).bind(connection).fetch_optional(&mut *tx).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::PreconditionFailed)?;
        if row.get::<i64, _>("generation") != expected_generation
            || !matches!(
                row.get::<String, _>("status").as_str(),
                "enabled" | "authorization_unknown"
            )
            || row.get::<String, _>("kind") != "openai_compatible"
        {
            return Err(HubError::PreconditionFailed);
        }
        let endpoint: serde_json::Value = row
            .get::<Option<serde_json::Value>, _>("endpoint_snapshot")
            .ok_or(HubError::PreconditionFailed)?;
        if endpoint.get("base_url").and_then(|v| v.as_str())
            != Some("https://openrouter.ai/api/v1/")
            || endpoint.get("allow_loopback").and_then(|v| v.as_bool()) != Some(false)
        {
            return Err(HubError::InvalidSemantics(
                "OpenRouter metadata requires its exact owned preset",
            ));
        }
        if row.get::<String, _>("key_id") != hex::encode(self.vault.fingerprint()) {
            return Err(HubError::InstallationMismatch);
        }
        let secret = self.vault.open(
            self.store.installation_id,
            connection,
            expected_generation,
            "credential",
            &Sealed {
                ciphertext: row.get("ciphertext"),
                nonce: row
                    .get::<Vec<u8>, _>("nonce")
                    .try_into()
                    .map_err(|_| HubError::InstallationMismatch)?,
            },
        )?;
        let fence = Uuid::new_v4();
        let endpoint_hash: String = row.get("endpoint_policy_hash");
        sqlx::query("INSERT INTO metadata_refreshes(operation_id,installation_id,connection_id,generation,endpoint_policy_hash,fence,state,lease_until) VALUES($1,$2,$3,$4,$5,$6,'running',clock_timestamp()+interval '60 seconds')")
            .bind(operation).bind(self.store.installation_id).bind(connection).bind(expected_generation).bind(&endpoint_hash).bind(fence).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        let result = Operation {
            id: operation,
            status: "pending".into(),
            resource_id: Some(connection),
            safe_error: None,
            version: 1,
        };
        sqlx::query("UPDATE operations SET resource_id=$3,safe_result=$4 WHERE installation_id=$1 AND id=$2").bind(self.store.installation_id).bind(operation).bind(connection).bind(serde_json::to_value(&result).map_err(|_|HubError::Unavailable)?).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,reason) VALUES($1,$2,$3,'catalog.refresh.start',$4,$5,'Сохранено намерение чтения метаданных')")
            .bind(Uuid::new_v4()).bind(self.store.installation_id).bind(subject).bind(connection.to_string()).bind(operation).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        tx.commit().await.map_err(|e| db_failure(e, line!()))?;
        Ok(MetadataStart::Claim(MetadataClaim {
            operation: result,
            connection_id: connection,
            generation: expected_generation,
            endpoint_hash,
            fence,
            secret,
        }))
    }
    pub async fn fail(
        &self,
        claim: &MetadataClaim,
        uncertain: bool,
    ) -> Result<Operation, HubError> {
        let mut tx = self
            .store
            .pool
            .begin()
            .await
            .map_err(|e| db_failure(e, line!()))?;
        self.lock_claim(&mut tx, claim, false).await?;
        let state = if uncertain { "unknown" } else { "failed" };
        let result = self
            .store
            .finish_metadata(&mut tx, claim.operation.id, claim.connection_id, state)
            .await?;
        tx.commit().await.map_err(|e| db_failure(e, line!()))?;
        Ok(result)
    }
    async fn lock_claim(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        claim: &MetadataClaim,
        require_lease: bool,
    ) -> Result<(), HubError> {
        let row=sqlx::query("SELECT state,lease_until>clock_timestamp() AS live FROM metadata_refreshes WHERE installation_id=$1 AND operation_id=$2 AND fence=$3 AND connection_id=$4 AND generation=$5 AND endpoint_policy_hash=$6 FOR UPDATE")
            .bind(self.store.installation_id).bind(claim.operation.id).bind(claim.fence).bind(claim.connection_id).bind(claim.generation).bind(&claim.endpoint_hash).fetch_optional(&mut **tx).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::PreconditionFailed)?;
        if row.get::<String, _>("state") != "running"
            || require_lease && !row.get::<bool, _>("live")
        {
            return Err(HubError::PreconditionFailed);
        }
        Ok(())
    }
    /// Atomically publish sanitized metadata and operation readback. This grants no inference rights.
    pub async fn complete(
        &self,
        claim: &MetadataClaim,
        catalog: &CatalogObservation,
        account: &AccountObservation,
    ) -> Result<Operation, HubError> {
        if account.digest.len() != 64
            || !account.digest.bytes().all(|b| b.is_ascii_hexdigit())
            || account.is_management_key != Some(false)
            || account.expires_at.is_some_and(|t| t <= chrono::Utc::now())
        {
            return Err(HubError::Invalid("account metadata"));
        }
        let mut tx = self
            .store
            .pool
            .begin()
            .await
            .map_err(|e| db_failure(e, line!()))?;
        self.lock_claim(&mut tx, claim, true).await?;
        let current=sqlx::query("SELECT c.id FROM connections c JOIN credential_versions v ON v.connection_id=c.id AND v.generation=c.generation WHERE c.installation_id=$1 AND c.id=$2 AND c.generation=$3 AND c.status IN ('authorization_unknown','enabled') AND v.state IN ('prepared','active') FOR UPDATE OF c FOR SHARE OF v")
            .bind(self.store.installation_id).bind(claim.connection_id).bind(claim.generation).fetch_optional(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        if current.is_none() {
            return Err(HubError::PreconditionFailed);
        }
        let snapshot = self
            .store
            .store_catalog_locked(
                &mut tx,
                claim.connection_id,
                claim.generation,
                &claim.endpoint_hash,
                catalog,
            )
            .await?;
        let summary = serde_json::json!({"limit":account.limit.as_ref().map(ToString::to_string),"remaining":account.remaining.as_ref().map(ToString::to_string),"usage":account.usage.as_ref().map(ToString::to_string),"byok_usage":account.byok_usage.as_ref().map(ToString::to_string),"is_free_tier":account.is_free_tier,"is_management_key":account.is_management_key,"include_byok_in_limit":account.include_byok_in_limit,"expires_at":account.expires_at,"currency_status":"unqualified"});
        sqlx::query("INSERT INTO metadata_account_observations(operation_id,catalog_snapshot_id,source_digest,summary,observed_at) VALUES($1,$2,$3,$4,$5)")
            .bind(claim.operation.id).bind(snapshot).bind(&account.digest).bind(summary).bind(catalog.observed_at).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        let result = self
            .store
            .finish_metadata(
                &mut tx,
                claim.operation.id,
                claim.connection_id,
                "succeeded",
            )
            .await?;
        tx.commit().await.map_err(|e| db_failure(e, line!()))?;
        Ok(result)
    }
}
impl PgStore {
    async fn finish_metadata(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        operation: Uuid,
        connection: Uuid,
        state: &str,
    ) -> Result<Operation, HubError> {
        sqlx::query("UPDATE metadata_refreshes SET state=$3 WHERE installation_id=$1 AND operation_id=$2 AND state='running'").bind(self.installation_id).bind(operation).bind(state).execute(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
        let row=sqlx::query("UPDATE operations SET state=$3,version=version+1,updated_at=now() WHERE installation_id=$1 AND id=$2 AND state='pending' RETURNING version,principal_id")
            .bind(self.installation_id).bind(operation).bind(state).fetch_optional(&mut **tx).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::PreconditionFailed)?;
        let result = Operation {
            id: operation,
            status: state.into(),
            resource_id: Some(connection),
            safe_error: if state == "succeeded" {
                None
            } else {
                Some("Не удалось сохранить результат чтения метаданных".into())
            },
            version: row.get("version"),
        };
        sqlx::query("UPDATE operations SET safe_result=$3 WHERE installation_id=$1 AND id=$2")
            .bind(self.installation_id)
            .bind(operation)
            .bind(serde_json::to_value(&result).map_err(|_| HubError::Unavailable)?)
            .execute(&mut **tx)
            .await
            .map_err(|e| db_failure(e, line!()))?;
        sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,reason) VALUES($1,$2,$3,'catalog.refresh.finish',$4,$5,$6)")
            .bind(Uuid::new_v4()).bind(self.installation_id).bind(row.get::<String,_>("principal_id")).bind(connection.to_string()).bind(operation).bind(state).execute(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
        Ok(result)
    }
    pub async fn recover_expired_metadata(&self) -> Result<u64, HubError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let rows=sqlx::query("SELECT operation_id,connection_id FROM metadata_refreshes WHERE installation_id=$1 AND state='running' AND lease_until<=clock_timestamp() ORDER BY lease_until,operation_id LIMIT 100 FOR UPDATE SKIP LOCKED")
            .bind(self.installation_id).fetch_all(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        for row in &rows {
            self.finish_metadata(
                &mut tx,
                row.get("operation_id"),
                row.get("connection_id"),
                "unknown",
            )
            .await?;
        }
        tx.commit().await.map_err(|e| db_failure(e, line!()))?;
        Ok(rows.len() as u64)
    }
}
#[async_trait::async_trait]
impl aihub_application::MetadataOperations for MetadataRefresh {
    async fn refresh(
        &self,
        principal: &aihub_domain::access::HumanPrincipal,
        key: Uuid,
        connection: Uuid,
        expected_generation: i64,
    ) -> Result<Operation, HubError> {
        principal.require_config(true)?;
        let claim = match self
            .begin(&principal.subject, key, connection, expected_generation)
            .await?
        {
            MetadataStart::Readback(result) => return Ok(result),
            MetadataStart::Claim(claim) => claim,
        };
        match self.http.read(&claim.secret).await {
            Ok((catalog, account)) => match self.complete(&claim, &catalog, &account).await {
                Ok(result) => Ok(result),
                Err(HubError::PreconditionFailed | HubError::Invalid(_)) => {
                    self.fail(&claim, false).await
                }
                // A DB failure can be a lost commit acknowledgement. Recovery/readback owns uncertainty.
                Err(error) => Err(error),
            },
            Err(failure) => {
                self.fail(
                    &claim,
                    matches!(failure, crate::metadata_http::ReadFailure::Unknown),
                )
                .await
            }
        }
    }
}
