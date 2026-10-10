//! Atomic encrypted credential ownership. Possession of a key is not account qualification.
use crate::{financial::db_failure, postgres::PgStore};
use aihub_domain::{connections::ProtectedCredential, error::HubError, records::Operation};
use sqlx::Row;
use uuid::Uuid;

impl PgStore {
    pub(crate) async fn write_credential_control(
        &self,
        subject: &str,
        key: Uuid,
        binding: [u8; 32],
        id: Uuid,
        expected_generation: i64,
        secret: &ProtectedCredential,
    ) -> Result<Operation, HubError> {
        if id.is_nil()
            || expected_generation < 1
            || expected_generation == i64::MAX
            || secret.ciphertext.len() < 16
            || secret.ciphertext.len() > 16400
            || secret.key_id != hex::encode(&self.vault_fingerprint)
        {
            return Err(HubError::Invalid("protected credential context"));
        }
        self.change_authorization(
            subject,
            key,
            binding,
            id,
            Some((expected_generation, secret)),
            None,
        )
        .await
    }
    pub(crate) async fn revoke_credential_control(
        &self,
        subject: &str,
        key: Uuid,
        binding: [u8; 32],
        id: Uuid,
        expected_version: i64,
    ) -> Result<Operation, HubError> {
        if id.is_nil() || expected_version < 1 {
            return Err(HubError::Invalid("credential revoke context"));
        }
        self.change_authorization(subject, key, binding, id, None, Some(expected_version))
            .await
    }
    async fn change_authorization(
        &self,
        subject: &str,
        key: Uuid,
        binding: [u8; 32],
        id: Uuid,
        write: Option<(i64, &ProtectedCredential)>,
        expected_version: Option<i64>,
    ) -> Result<Operation, HubError> {
        let action = if write.is_some() {
            "credential.write"
        } else {
            "credential.revoke"
        };
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let (operation, replay) = self
            .begin_control_operation(&mut tx, subject, key, binding, action)
            .await?;
        if let Some(replay) = replay {
            return serde_json::from_value(replay).map_err(|_| HubError::Unavailable);
        }
        let row=sqlx::query("SELECT c.*,p.kind FROM connections c JOIN providers p ON p.installation_id=c.installation_id AND p.id=c.provider_id WHERE c.installation_id=$1 AND c.id=$2 FOR UPDATE OF c")
            .bind(self.installation_id).bind(id).fetch_optional(&mut *tx).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::NotFound)?;
        if row.get::<String, _>("status") == "archived" {
            return Err(HubError::PreconditionFailed);
        }
        if write.is_some() && row.get::<String, _>("kind") == "chatgpt_managed" {
            return Err(HubError::InvalidSemantics(
                "managed provider requires own login",
            ));
        }
        let current: i64 = row.get("generation");
        if write.is_some_and(|(expected, _)| expected != current)
            || expected_version.is_some_and(|v| v != row.get::<i64, _>("version"))
        {
            return Err(HubError::PreconditionFailed);
        }
        let generation = current
            .checked_add(1)
            .ok_or(HubError::InvalidSemantics("generation exhausted"))?;
        let old=sqlx::query("SELECT adapter_revision,endpoint_policy_hash,endpoint_snapshot FROM connection_generations WHERE connection_id=$1 AND generation=$2 FOR SHARE")
            .bind(id).bind(current).fetch_one(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        let endpoint: serde_json::Value = old
            .get::<Option<serde_json::Value>, _>("endpoint_snapshot")
            .ok_or(HubError::PreconditionFailed)?;
        sqlx::query("INSERT INTO connection_generations(connection_id,generation,authorization_state,adapter_revision,endpoint_policy_hash,endpoint_snapshot) VALUES($1,$2,$3,$4,$5,$6)")
            .bind(id).bind(generation).bind(if write.is_some(){"prepared"}else{"revoked"}).bind(old.get::<String,_>("adapter_revision")).bind(old.get::<String,_>("endpoint_policy_hash")).bind(endpoint).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        sqlx::query("UPDATE credential_versions SET state='revoked' WHERE connection_id=$1 AND state IN ('prepared','active')").bind(id).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        if let Some((_, secret)) = write {
            sqlx::query("INSERT INTO credential_versions(connection_id,generation,ciphertext,nonce,key_id,state) VALUES($1,$2,$3,$4,$5,'prepared')")
                .bind(id).bind(generation).bind(&secret.ciphertext).bind(secret.nonce.as_slice()).bind(&secret.key_id).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        }
        sqlx::query("UPDATE connections SET generation=$3,version=version+1,status=CASE WHEN status='disabled' THEN 'disabled' ELSE $4 END WHERE installation_id=$1 AND id=$2")
            .bind(self.installation_id).bind(id).bind(generation).bind(if write.is_some(){"authorization_unknown"}else{"revoked"}).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        sqlx::query("UPDATE runtime_qualifications SET state='invalidated',invalidated_reason='Authorization generation changed' WHERE installation_id=$1 AND connection_id=$2 AND state='active'")
            .bind(self.installation_id).bind(id).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        let result = Operation {
            id: operation,
            status: "succeeded".into(),
            resource_id: Some(id),
            safe_error: None,
            version: 2,
        };
        sqlx::query("UPDATE operations SET state='succeeded',resource_id=$3,safe_result=$4,version=version+1,updated_at=now() WHERE installation_id=$1 AND id=$2")
            .bind(self.installation_id).bind(operation).bind(id).bind(serde_json::to_value(&result).map_err(|_|HubError::Unavailable)?).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,reason) VALUES($1,$2,$3,$4,$5,$6,'Write-only authorization generation; no provider qualification implied')")
            .bind(Uuid::new_v4()).bind(self.installation_id).bind(subject).bind(action).bind(id.to_string()).bind(operation).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        tx.commit().await.map_err(|e| db_failure(e, line!()))?;
        Ok(result)
    }
}
