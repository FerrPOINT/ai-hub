use crate::{financial::db_failure, postgres::PgStore};
use aihub_domain::{
    error::HubError,
    model_context::{
        ModelContextInput, ModelContextMutation, ModelContextPreference, ModelContextSnapshot,
    },
    records::Page,
};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;
fn preference(row: &sqlx::postgres::PgRow) -> Result<ModelContextPreference, HubError> {
    Ok(ModelContextPreference {
        connection_id: row.get("connection_id"),
        model_id: row.get("model_id"),
        version: row.get("version"),
        context_window_tokens: row
            .get::<i64, _>("context_window_tokens")
            .try_into()
            .map_err(|_| HubError::Unavailable)?,
        updated_at: row.get("created_at"),
    })
}
impl PgStore {
    pub(crate) async fn qualified_model_context_locked(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        connection: Uuid,
        model: &str,
        qualification: &sqlx::postgres::PgRow,
    ) -> Result<ModelContextSnapshot, HubError> {
        let current = self
            .model_context_snapshot_locked(tx, connection, model)
            .await?;
        if qualification.get::<Option<i64>, _>("model_context_version") != Some(current.version)
            || qualification.get::<Option<Uuid>, _>("model_context_revision_id")
                != current.revision_id
        {
            return Err(HubError::PreconditionFailed);
        }
        Ok(current)
    }
    pub(crate) async fn model_context_page_internal(
        &self,
        subject: &str,
        connection: Uuid,
        model: Option<&str>,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<Page<ModelContextPreference>, HubError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM connections WHERE installation_id=$1 AND id=$2)",
        )
        .bind(self.installation_id)
        .bind(connection)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| db_failure(e, line!()))?;
        if !exists {
            return Err(HubError::NotFound);
        }
        if let Some(model) = model {
            aihub_domain::model_context::validate_model_id(model)?;
            if cursor.is_some() {
                return Err(HubError::Invalid("exact context cursor"));
            }
        }
        let items = if cursor.is_none() {
            let rows=sqlx::query("SELECT r.* FROM model_context_preferences p JOIN model_context_revisions r ON r.id=p.current_revision_id WHERE p.installation_id=$1 AND p.connection_id=$2 AND ($3::text IS NULL OR p.model_id=$3) ORDER BY p.model_id LIMIT 10001")
                .bind(self.installation_id).bind(connection).bind(model).fetch_all(&self.pool).await.map_err(|e|db_failure(e,line!()))?;
            rows.iter().map(preference).collect::<Result<Vec<_>, _>>()?
        } else {
            vec![]
        };
        if model.is_some() {
            return Ok(Page {
                items,
                next_cursor: None,
            });
        }
        self.snapshot_page(
            subject,
            &format!("model-context:{connection}:{limit}"),
            limit,
            cursor,
            items,
        )
        .await
    }
    pub(crate) async fn save_model_context_internal(
        &self,
        subject: &str,
        key: Uuid,
        binding: [u8; 32],
        connection: Uuid,
        expected: i64,
        input: &ModelContextInput,
    ) -> Result<ModelContextMutation, HubError> {
        input.validate()?;
        if connection.is_nil() || expected < 0 || expected == i64::MAX {
            return Err(HubError::Invalid("model context CAS"));
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let (operation, replay) = self
            .begin_control_operation(&mut tx, subject, key, binding, "model-context.write")
            .await?;
        if let Some(replay) = replay {
            return serde_json::from_value(replay).map_err(|_| HubError::Unavailable);
        }
        // ponytail: one connection lock serializes config; per-model locks if measured contention warrants it.
        let row = sqlx::query(
            "SELECT status FROM connections WHERE installation_id=$1 AND id=$2 FOR UPDATE",
        )
        .bind(self.installation_id)
        .bind(connection)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| db_failure(e, line!()))?
        .ok_or(HubError::NotFound)?;
        if row.get::<String, _>("status") == "archived" {
            return Err(HubError::PreconditionFailed);
        }
        let current = self
            .model_context_snapshot_locked(&mut tx, connection, &input.model_id)
            .await?;
        if current.version != expected {
            return Err(HubError::PreconditionFailed);
        }
        let revision = Uuid::new_v4();
        let version = expected + 1;
        let row=sqlx::query("INSERT INTO model_context_revisions(id,installation_id,connection_id,model_id,version,context_window_tokens,operation_id) VALUES($1,$2,$3,$4,$5,$6,$7) RETURNING *")
            .bind(revision).bind(self.installation_id).bind(connection).bind(&input.model_id).bind(version).bind(i64::from(input.context_window_tokens)).bind(operation).fetch_one(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        sqlx::query("INSERT INTO model_context_preferences(installation_id,connection_id,model_id,version,current_revision_id) VALUES($1,$2,$3,$4,$5) ON CONFLICT(connection_id,model_id) DO UPDATE SET version=EXCLUDED.version,current_revision_id=EXCLUDED.current_revision_id")
            .bind(self.installation_id).bind(connection).bind(&input.model_id).bind(version).bind(revision).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        sqlx::query("UPDATE runtime_qualifications SET state='invalidated',invalidated_reason='model_context_changed' WHERE installation_id=$1 AND connection_id=$2 AND provider_model_id=$3 AND state='active'")
            .bind(self.installation_id).bind(connection).bind(&input.model_id).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        let result = ModelContextMutation {
            operation_id: operation,
            preference: preference(&row)?,
        };
        sqlx::query("UPDATE operations SET state='succeeded',resource_id=$3,safe_result=$4,version=version+1,updated_at=clock_timestamp() WHERE installation_id=$1 AND id=$2")
            .bind(self.installation_id).bind(operation).bind(connection).bind(serde_json::to_value(&result).map_err(|_|HubError::Unavailable)?).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,reason) VALUES($1,$2,$3,'model-context.write',$4,$5,'Изменён сохранённый контекст модели; прежний proof недействителен')")
            .bind(Uuid::new_v4()).bind(self.installation_id).bind(subject).bind(format!("{connection}/{}",input.model_id)).bind(operation).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        tx.commit().await.map_err(|e| db_failure(e, line!()))?;
        Ok(result)
    }
    /// Caller holds the connection lock through commit, including the absence case.
    pub(crate) async fn model_context_snapshot_locked(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        connection: Uuid,
        model: &str,
    ) -> Result<ModelContextSnapshot, HubError> {
        let row=sqlx::query("SELECT p.current_revision_id,p.version,r.context_window_tokens FROM model_context_preferences p JOIN model_context_revisions r ON r.id=p.current_revision_id WHERE p.installation_id=$1 AND p.connection_id=$2 AND p.model_id=$3")
            .bind(self.installation_id).bind(connection).bind(model).fetch_optional(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
        match row {
            None => Ok(ModelContextSnapshot {
                revision_id: None,
                version: 0,
                context_window_tokens: None,
            }),
            Some(row) => Ok(ModelContextSnapshot {
                revision_id: Some(row.get("current_revision_id")),
                version: row.get("version"),
                context_window_tokens: Some(
                    row.get::<i64, _>("context_window_tokens")
                        .try_into()
                        .map_err(|_| HubError::Unavailable)?,
                ),
            }),
        }
    }
}
