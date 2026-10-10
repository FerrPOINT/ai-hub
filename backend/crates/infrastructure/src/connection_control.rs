use crate::{financial::db_failure, postgres::PgStore};
use aihub_domain::{
    connections::{
        BillingMode, Connection, ConnectionInput, ConnectionMutation, EndpointPolicyInput,
        ProviderKind,
    },
    error::HubError,
    records::Page,
};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;
const VIEW: &str = "SELECT c.*,p.kind,g.endpoint_snapshot,EXISTS(SELECT 1 FROM credential_versions v WHERE v.connection_id=c.id AND v.generation=c.generation AND v.state IN ('prepared','active')) AS has_credentials FROM connections c JOIN providers p ON p.installation_id=c.installation_id AND p.id=c.provider_id LEFT JOIN connection_generations g ON g.connection_id=c.id AND g.generation=c.generation";
fn connection(row: &sqlx::postgres::PgRow) -> Result<Connection, HubError> {
    let billing_mode = match row.get::<String, _>("billing_mode").as_str() {
        "metered" => BillingMode::Metered,
        "subscription" => BillingMode::Subscription,
        "local" => BillingMode::Local,
        "unknown" => BillingMode::Unknown,
        _ => return Err(HubError::Unavailable),
    };
    let provider_kind = ProviderKind::parse(&row.get::<String, _>("kind"))?;
    let snapshot = row.get::<Option<serde_json::Value>, _>("endpoint_snapshot");
    Ok(Connection {
        id: row.get("id"),
        provider_kind,
        display_name: row.get("display_name"),
        generation: row.get("generation"),
        status: row.get("status"),
        has_credentials: row.get("has_credentials"),
        catalog_refresh_supported: snapshot.as_ref().is_some_and(|p| {
            aihub_domain::connections::supports_openrouter_metadata(provider_kind, p)
        }),
        quota: None,
        version: row.get("version"),
        settings: ConnectionInput {
            display_name: row.get("display_name"),
            endpoint_policy_ref: row.get("endpoint_policy_ref"),
            billing_mode,
        },
    })
}
impl PgStore {
    pub(crate) async fn disable_connection_internal(
        &self,
        subject: &str,
        key: Uuid,
        binding: [u8; 32],
        id: Uuid,
        expected: i64,
    ) -> Result<aihub_domain::records::Operation, HubError> {
        if id.is_nil() || expected < 1 {
            return Err(HubError::Invalid("connection disable"));
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let (operation, replay) = self
            .begin_control_operation(&mut tx, subject, key, binding, "connection.disable")
            .await?;
        if let Some(replay) = replay {
            return serde_json::from_value(replay).map_err(|_| HubError::Unavailable);
        }
        let row = sqlx::query(
            "SELECT status,version FROM connections WHERE installation_id=$1 AND id=$2 FOR UPDATE",
        )
        .bind(self.installation_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| db_failure(e, line!()))?
        .ok_or(HubError::NotFound)?;
        if row.get::<i64, _>("version") != expected {
            return Err(HubError::PreconditionFailed);
        }
        if row.get::<String, _>("status") != "disabled" {
            sqlx::query("UPDATE connections SET status='disabled',version=version+1 WHERE installation_id=$1 AND id=$2").bind(self.installation_id).bind(id).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        }
        let result = aihub_domain::records::Operation {
            id: operation,
            status: "succeeded".into(),
            resource_id: Some(id),
            safe_error: None,
            version: 2,
        };
        sqlx::query("UPDATE operations SET state='succeeded',resource_id=$3,safe_result=$4,version=version+1,updated_at=clock_timestamp() WHERE installation_id=$1 AND id=$2").bind(self.installation_id).bind(operation).bind(id).bind(serde_json::to_value(&result).map_err(|_|HubError::Unavailable)?).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,reason) VALUES($1,$2,$3,'connection.disable',$4,$5,'Отключён новый admission и dispatch; история сохранена')").bind(Uuid::new_v4()).bind(self.installation_id).bind(subject).bind(id.to_string()).bind(operation).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        tx.commit().await.map_err(|e| db_failure(e, line!()))?;
        Ok(result)
    }
    pub async fn configure_endpoints(
        &self,
        policies: &[EndpointPolicyInput],
    ) -> Result<(), HubError> {
        if policies.is_empty() || policies.len() > 100 {
            return Err(HubError::Invalid("endpoint policy count"));
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| db_failure(e, line!()))?;
        for policy in policies {
            policy.validate()?;
            let value =
                serde_json::to_value(policy).map_err(|_| HubError::Invalid("endpoint policy"))?;
            let hash = hex::encode(Sha256::digest(
                serde_json::to_vec(&value).map_err(|_| HubError::Invalid("endpoint policy"))?,
            ));
            sqlx::query("INSERT INTO endpoint_policies(installation_id,policy_ref,provider_kind,policy,policy_hash) VALUES($1,$2,$3,$4,$5) ON CONFLICT(installation_id,policy_ref) DO NOTHING")
                .bind(self.installation_id).bind(&policy.policy_ref).bind(policy.provider_kind.as_str()).bind(value).bind(&hash).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
            let saved:String=sqlx::query_scalar("SELECT policy_hash FROM endpoint_policies WHERE installation_id=$1 AND policy_ref=$2 FOR SHARE").bind(self.installation_id).bind(&policy.policy_ref).fetch_one(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
            if saved != hash {
                return Err(HubError::IdempotencyConflict);
            }
        }
        tx.commit().await.map_err(|e| db_failure(e, line!()))
    }
    pub(crate) async fn connection_page(
        &self,
        subject: &str,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<Page<Connection>, HubError> {
        let items = if cursor.is_none() {
            let rows = sqlx::query(&format!(
                "{VIEW} WHERE c.installation_id=$1 ORDER BY c.id LIMIT 10001"
            ))
            .bind(self.installation_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| db_failure(e, line!()))?;
            rows.iter().map(connection).collect::<Result<Vec<_>, _>>()?
        } else {
            vec![]
        };
        self.snapshot_page(
            subject,
            &format!("connections:{limit}"),
            limit,
            cursor,
            items,
        )
        .await
    }
    pub(crate) async fn read_connection(&self, id: Uuid) -> Result<Connection, HubError> {
        let row = sqlx::query(&format!("{VIEW} WHERE c.installation_id=$1 AND c.id=$2"))
            .bind(self.installation_id)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| db_failure(e, line!()))?
            .ok_or(HubError::NotFound)?;
        connection(&row)
    }
    pub(crate) async fn save_connection(
        &self,
        subject: &str,
        key: Uuid,
        binding: [u8; 32],
        kind: ProviderKind,
        update: Option<(Uuid, i64)>,
        input: &ConnectionInput,
    ) -> Result<ConnectionMutation, HubError> {
        input.validate()?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let action = if update.is_some() {
            "connection.update"
        } else {
            "connection.create"
        };
        let (operation, replay) = self
            .begin_control_operation(&mut tx, subject, key, binding, action)
            .await?;
        if let Some(replay) = replay {
            return serde_json::from_value(replay).map_err(|_| HubError::Unavailable);
        }
        let policy=sqlx::query("SELECT policy,policy_hash FROM endpoint_policies WHERE installation_id=$1 AND policy_ref=$2 AND provider_kind=$3 FOR SHARE")
            .bind(self.installation_id).bind(&input.endpoint_policy_ref).bind(kind.as_str()).fetch_optional(&mut *tx).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::InvalidSemantics("unknown endpoint policy or provider mismatch"))?;
        let endpoint: serde_json::Value = policy.get("policy");
        let hash: String = policy.get("policy_hash");
        let provider: Uuid =
            sqlx::query_scalar("SELECT id FROM providers WHERE installation_id=$1 AND kind=$2")
                .bind(self.installation_id)
                .bind(kind.as_str())
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| db_failure(e, line!()))?;
        let (id, generation, new_generation) = if let Some((id, version)) = update {
            let existing = sqlx::query(
                "SELECT * FROM connections WHERE installation_id=$1 AND id=$2 FOR UPDATE",
            )
            .bind(self.installation_id)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| db_failure(e, line!()))?
            .ok_or(HubError::NotFound)?;
            if existing.get::<Uuid, _>("provider_id") != provider {
                return Err(HubError::InvalidSemantics("provider is immutable"));
            }
            if existing.get::<i64, _>("version") != version {
                return Err(HubError::PreconditionFailed);
            }
            let material = existing.get::<String, _>("endpoint_policy_ref")
                != input.endpoint_policy_ref
                || existing.get::<String, _>("billing_mode") != input.billing_mode.as_str();
            let generation = if material {
                existing
                    .get::<i64, _>("generation")
                    .checked_add(1)
                    .ok_or(HubError::InvalidSemantics("generation exhausted"))?
            } else {
                existing.get("generation")
            };
            sqlx::query("UPDATE connections SET display_name=$3,endpoint_policy_ref=$4,billing_mode=$5,generation=$6,version=version+1,status=CASE WHEN $7 AND status<>'disabled' THEN 'authorization_unknown' ELSE status END WHERE installation_id=$1 AND id=$2")
                .bind(self.installation_id).bind(id).bind(&input.display_name).bind(&input.endpoint_policy_ref).bind(input.billing_mode.as_str()).bind(generation).bind(material).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
            // Settings change never silently carries authorization into a new endpoint/generation.
            if material {
                sqlx::query("UPDATE runtime_qualifications SET state='invalidated',invalidated_reason='Connection settings generation changed' WHERE installation_id=$1 AND connection_id=$2 AND state='active'").bind(self.installation_id).bind(id).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
            }
            (id, generation, material)
        } else {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO connections(id,installation_id,provider_id,display_name,endpoint_policy_ref,billing_mode,status) VALUES($1,$2,$3,$4,$5,$6,'authorization_unknown')")
                .bind(id).bind(self.installation_id).bind(provider).bind(&input.display_name).bind(&input.endpoint_policy_ref).bind(input.billing_mode.as_str()).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
            (id, 1, true)
        };
        if new_generation {
            sqlx::query("INSERT INTO connection_generations(connection_id,generation,authorization_state,adapter_revision,endpoint_policy_hash,endpoint_snapshot) VALUES($1,$2,'absent',$3,$4,$5)")
            .bind(id).bind(generation).bind(format!("{}-v1",kind.as_str())).bind(hash).bind(endpoint).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        }
        let row = sqlx::query(&format!("{VIEW} WHERE c.installation_id=$1 AND c.id=$2"))
            .bind(self.installation_id)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let result = ConnectionMutation {
            operation_id: operation,
            value: connection(&row)?,
        };
        self.finish_connection_operation(&mut tx, subject, action, &result)
            .await?;
        tx.commit().await.map_err(|e| db_failure(e, line!()))?;
        Ok(result)
    }
    async fn finish_connection_operation(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        subject: &str,
        action: &str,
        result: &ConnectionMutation,
    ) -> Result<(), HubError> {
        sqlx::query("UPDATE operations SET state='succeeded',resource_id=$3,safe_result=$4,version=version+1,updated_at=now() WHERE installation_id=$1 AND id=$2")
            .bind(self.installation_id).bind(result.operation_id).bind(result.value.id).bind(serde_json::to_value(result).map_err(|_|HubError::Unavailable)?).execute(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
        sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,reason) VALUES($1,$2,$3,$4,$5,$6,'Connection control; no provider I/O')")
            .bind(Uuid::new_v4()).bind(self.installation_id).bind(subject).bind(action).bind(result.value.id.to_string()).bind(result.operation_id).execute(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
        Ok(())
    }
}
