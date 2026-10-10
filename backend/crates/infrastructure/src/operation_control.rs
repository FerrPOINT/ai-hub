//! Actor-owned recovery for write-only control bodies. No provider capability.
use crate::{financial::db_failure, postgres::PgStore};
use aihub_application::FoundationStore;
use aihub_domain::{
    connections::EndpointPolicyInput,
    error::HubError,
    records::{Operation, OperationLookup},
};
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;
fn lookup(row: &PgRow) -> OperationLookup {
    OperationLookup {
        idempotency_key: row.get("idempotency_key"),
        action: row.get("action"),
        operation: Operation {
            id: row.get("id"),
            status: row.get("state"),
            resource_id: row.get("resource_id"),
            safe_error: row.get("safe_error"),
            version: row.get("version"),
        },
    }
}
impl PgStore {
    pub(crate) async fn operation_key_internal(
        &self,
        subject: &str,
        key: Uuid,
    ) -> Result<OperationLookup, HubError> {
        let row=sqlx::query("SELECT id,idempotency_key,action,state,resource_id,version,safe_result->>'safe_error' AS safe_error FROM operations WHERE installation_id=$1 AND principal_kind='human' AND principal_id=$2 AND idempotency_key=$3")
            .bind(self.installation_id).bind(subject).bind(key).fetch_optional(&self.pool).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::NotFound)?;
        Ok(lookup(&row))
    }
    pub(crate) async fn close_unstarted_internal(
        &self,
        subject: &str,
        key: Uuid,
        binding: [u8; 32],
    ) -> Result<OperationLookup, HubError> {
        self.ready().await?;
        if subject.is_empty() || key.is_nil() {
            return Err(HubError::Invalid("operation close"));
        }
        let id = Uuid::new_v4();
        let closed = Operation {
            id,
            status: "cancelled".into(),
            resource_id: None,
            safe_error: Some("Запрос закрыт до начала выполнения".into()),
            version: 1,
        };
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| db_failure(e, line!()))?;
        // Unique-key insertion waits for any original transaction. Existing intent is only read back.
        let row=sqlx::query("INSERT INTO operations(id,installation_id,principal_kind,principal_id,idempotency_key,binding_hmac,action,state,safe_result,expires_at) VALUES($1,$2,'human',$3,$4,$5,'operation.close-unstarted','cancelled',$6,clock_timestamp()+interval '30 days') ON CONFLICT(installation_id,principal_kind,principal_id,idempotency_key) DO UPDATE SET id=operations.id RETURNING *,safe_result->>'safe_error' AS safe_error")
            .bind(id).bind(self.installation_id).bind(subject).bind(key).bind(binding.as_slice()).bind(serde_json::to_value(closed).map_err(|_|HubError::Unavailable)?).fetch_one(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        if row.get::<Uuid, _>("id") == id {
            sqlx::query("INSERT INTO control_key_fences(installation_id,principal_kind,principal_id,idempotency_key,operation_id) VALUES($1,'human',$2,$3,$4)")
                .bind(self.installation_id).bind(subject).bind(key).bind(id).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
            sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,reason) VALUES($1,$2,$3,'operation.close-unstarted',$4,$5,'Запрет позднего выполнения исходного запроса')")
                .bind(Uuid::new_v4()).bind(self.installation_id).bind(subject).bind(key.to_string()).bind(id).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        }
        let result = lookup(&row);
        tx.commit().await.map_err(|e| db_failure(e, line!()))?;
        Ok(result)
    }
    pub(crate) async fn endpoint_policy_page_internal(
        &self,
        subject: &str,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<aihub_domain::records::Page<EndpointPolicyInput>, HubError> {
        let items = if cursor.is_none() {
            let rows=sqlx::query("SELECT policy FROM endpoint_policies WHERE installation_id=$1 ORDER BY policy_ref LIMIT 10001").bind(self.installation_id).fetch_all(&self.pool).await.map_err(|e|db_failure(e,line!()))?;
            rows.into_iter()
                .map(|row| {
                    serde_json::from_value::<EndpointPolicyInput>(row.get("policy"))
                        .map_err(|_| HubError::Unavailable)
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            vec![]
        };
        self.snapshot_page(
            subject,
            &format!("endpoint-policies:{limit}"),
            limit,
            cursor,
            items,
        )
        .await
    }
}
