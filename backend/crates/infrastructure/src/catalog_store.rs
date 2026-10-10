use crate::{financial::db_failure, postgres::PgStore};
use aihub_domain::{
    catalog::{CatalogObservation, CatalogPage, ModelMetadata},
    error::HubError,
};
use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;
impl PgStore {
    /// Internal qualified reader output. This method cannot authorize an inference call.
    pub async fn store_catalog(
        &self,
        connection: Uuid,
        expected_generation: i64,
        endpoint_hash: &str,
        observation: &CatalogObservation,
    ) -> Result<Uuid, HubError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let id = self
            .store_catalog_locked(
                &mut tx,
                connection,
                expected_generation,
                endpoint_hash,
                observation,
            )
            .await?;
        tx.commit().await.map_err(|e| db_failure(e, line!()))?;
        Ok(id)
    }
    pub(crate) async fn store_catalog_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        connection: Uuid,
        expected_generation: i64,
        endpoint_hash: &str,
        observation: &CatalogObservation,
    ) -> Result<Uuid, HubError> {
        if connection.is_nil()
            || expected_generation < 1
            || observation.models.len() > 1000
            || observation.digest.len() != 64
            || !observation.digest.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(HubError::Invalid("catalog observation"));
        }
        let row=sqlx::query("SELECT c.generation,g.endpoint_policy_hash FROM connections c JOIN connection_generations g ON g.connection_id=c.id AND g.generation=c.generation WHERE c.installation_id=$1 AND c.id=$2 FOR UPDATE OF c,g")
            .bind(self.installation_id).bind(connection).fetch_optional(&mut **tx).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::NotFound)?;
        if row.get::<i64, _>("generation") != expected_generation
            || row.get::<String, _>("endpoint_policy_hash") != endpoint_hash
        {
            return Err(HubError::PreconditionFailed);
        }
        let clock: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| db_failure(e, line!()))?;
        if observation.observed_at > clock
            || observation.observed_at < clock - chrono::Duration::hours(24)
        {
            return Err(HubError::Invalid("catalog observation time"));
        }
        let mut models = Vec::with_capacity(observation.models.len());
        let mut identities = std::collections::BTreeSet::new();
        for model in &observation.models {
            let metadata = &model.metadata;
            if metadata.provider_model_id.is_empty()
                || metadata.provider_model_id.len() > 256
                || !identities.insert(&metadata.provider_model_id)
                || metadata.evidence_status != "unverified"
                || metadata.observed_at != observation.observed_at
                || metadata.capabilities.iter().any(|c| {
                    !matches!(
                        c.as_str(),
                        "text"
                            | "stream"
                            | "function_tools"
                            | "json_schema"
                            | "responses"
                            | "cancel"
                    )
                })
            {
                return Err(HubError::Invalid("catalog model metadata"));
            }
            models.push(metadata.clone());
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO catalog_snapshots(id,installation_id,connection_id,generation,source_digest,models,observed_at) VALUES($1,$2,$3,$4,$5,$6,$7)")
            .bind(id).bind(self.installation_id).bind(connection).bind(expected_generation).bind(&observation.digest).bind(serde_json::to_value(models).map_err(|_|HubError::Invalid("catalog models"))?).bind(observation.observed_at).execute(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
        for model in &observation.models {
            sqlx::query("INSERT INTO upstream_models(id,connection_id,generation,provider_model_id,input_limit,output_limit,metadata,observed_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(connection_id,generation,provider_model_id) DO UPDATE SET input_limit=EXCLUDED.input_limit,output_limit=EXCLUDED.output_limit,metadata=EXCLUDED.metadata,observed_at=EXCLUDED.observed_at")
                .bind(Uuid::new_v4()).bind(connection).bind(expected_generation).bind(&model.metadata.provider_model_id).bind(model.metadata.input_limit).bind(model.metadata.output_limit).bind(serde_json::to_value(&model.metadata).map_err(|_|HubError::Invalid("model metadata"))?).bind(observation.observed_at).execute(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
        }
        // Old rows are historical; the current immutable snapshot controls membership.
        sqlx::query("UPDATE connection_generations SET catalog_snapshot_id=$3 WHERE connection_id=$1 AND generation=$2").bind(connection).bind(expected_generation).bind(id).execute(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
        Ok(id)
    }
    pub(crate) async fn catalog_page(
        &self,
        subject: &str,
        connection: Uuid,
        query: &str,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<CatalogPage, HubError> {
        if query.len() > 256 || connection.is_nil() {
            return Err(HubError::Invalid("catalog filter"));
        }
        let row=sqlx::query("SELECT c.generation,s.observed_at,s.models FROM connections c JOIN connection_generations g ON g.connection_id=c.id AND g.generation=c.generation LEFT JOIN catalog_snapshots s ON s.installation_id=c.installation_id AND s.id=g.catalog_snapshot_id WHERE c.installation_id=$1 AND c.id=$2")
            .bind(self.installation_id).bind(connection).fetch_optional(&self.pool).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::NotFound)?;
        let generation: i64 = row.get("generation");
        let observed: Option<DateTime<Utc>> = row.get("observed_at");
        let as_of: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let models = if cursor.is_none() {
            let models: Vec<ModelMetadata> = serde_json::from_value(
                row.get::<Option<serde_json::Value>, _>("models")
                    .unwrap_or_else(|| serde_json::json!([])),
            )
            .map_err(|_| HubError::Unavailable)?;
            let query = query.to_lowercase();
            models
                .into_iter()
                .filter(|m| m.provider_model_id.to_lowercase().contains(&query))
                .collect()
        } else {
            vec![]
        };
        let page = self
            .snapshot_page(
                subject,
                &format!("catalog:{connection}:{generation}:{limit}:{query}"),
                limit,
                cursor,
                models,
            )
            .await?;
        let captured = page.items.first().map(|m| m.observed_at).or(observed);
        Ok(CatalogPage {
            connection_id: connection,
            generation,
            models: page.items,
            next_cursor: page.next_cursor,
            as_of: captured.unwrap_or(as_of),
            data_status: match captured {
                None => "partial",
                Some(time) if time < as_of - chrono::Duration::hours(24) => "stale",
                _ => "complete",
            }
            .into(),
        })
    }
}
