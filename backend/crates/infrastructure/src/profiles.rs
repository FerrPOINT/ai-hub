use crate::{financial::db_failure, postgres::PgStore};
use aihub_domain::{
    error::HubError,
    profiles::{DraftMutation, Profile, ProfileInput},
    records::Page,
};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;
fn profile(row: &sqlx::postgres::PgRow) -> Result<Profile, HubError> {
    Ok(Profile {
        id: row.get("id"),
        draft: serde_json::from_value(row.get("draft")).map_err(|_| HubError::Unavailable)?,
        draft_version: row.get("draft_version"),
        active_revision_id: row.get("active_revision_id"),
        status: row.get("status"),
    })
}
impl PgStore {
    pub(crate) async fn profile_page_internal(
        &self,
        subject: &str,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<Page<Profile>, HubError> {
        let items = if cursor.is_none() {
            let rows=sqlx::query("SELECT * FROM virtual_models WHERE installation_id=$1 ORDER BY slug,id LIMIT 10001").bind(self.installation_id).fetch_all(&self.pool).await.map_err(|e|db_failure(e,line!()))?;
            rows.iter().map(profile).collect::<Result<Vec<_>, _>>()?
        } else {
            vec![]
        };
        self.snapshot_page(subject, &format!("profiles:{limit}"), limit, cursor, items)
            .await
    }
    pub(crate) async fn read_profile_internal(&self, id: Uuid) -> Result<Profile, HubError> {
        let row = sqlx::query("SELECT * FROM virtual_models WHERE installation_id=$1 AND id=$2")
            .bind(self.installation_id)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| db_failure(e, line!()))?
            .ok_or(HubError::NotFound)?;
        profile(&row)
    }
    pub(crate) async fn save_profile_internal(
        &self,
        subject: &str,
        key: Uuid,
        binding: [u8; 32],
        update: Option<(Uuid, i64)>,
        input: &ProfileInput,
    ) -> Result<DraftMutation, HubError> {
        input.validate()?;
        if update.is_some_and(|(id, v)| id.is_nil() || v < 1 || v == i64::MAX) {
            return Err(HubError::Invalid("profile CAS"));
        }
        let action = if update.is_some() {
            "profile.draft.update"
        } else {
            "profile.draft.create"
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
        let configuration =
            serde_json::to_value(input).map_err(|_| HubError::Invalid("profile configuration"))?;
        let hash = hex::encode(Sha256::digest(
            serde_json::to_vec(&configuration).map_err(|_| HubError::Unavailable)?,
        ));
        let (id, version) = if let Some((id, expected)) = update {
            let row = sqlx::query(
                "SELECT * FROM virtual_models WHERE installation_id=$1 AND id=$2 FOR UPDATE",
            )
            .bind(self.installation_id)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| db_failure(e, line!()))?
            .ok_or(HubError::NotFound)?;
            if row.get::<i64, _>("draft_version") != expected
                || row.get::<String, _>("status") == "archived"
            {
                return Err(HubError::PreconditionFailed);
            }
            if row.get::<String, _>("slug") != input.slug {
                return Err(HubError::InvalidSemantics("profile slug is immutable"));
            }
            sqlx::query("UPDATE virtual_models SET display_name=$3,draft=$4,draft_version=draft_version+1 WHERE installation_id=$1 AND id=$2").bind(self.installation_id).bind(id).bind(&input.display_name).bind(&configuration).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
            (id, expected + 1)
        } else {
            let id = Uuid::new_v4();
            let exists=sqlx::query("INSERT INTO virtual_models(id,installation_id,slug,display_name,draft,draft_version,status) VALUES($1,$2,$3,$4,$5,1,'draft') ON CONFLICT(installation_id,slug) DO NOTHING RETURNING id").bind(id).bind(self.installation_id).bind(&input.slug).bind(&input.display_name).bind(&configuration).fetch_optional(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
            if exists.is_none() {
                return Err(HubError::AlreadyExists);
            }
            (id, 1)
        };
        sqlx::query("INSERT INTO profile_draft_revisions(installation_id,virtual_model_id,version,configuration,config_hash,operation_id) VALUES($1,$2,$3,$4,$5,$6)").bind(self.installation_id).bind(id).bind(version).bind(&configuration).bind(hash).bind(operation).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        for (ordinal, target) in input.deployments.iter().enumerate() {
            let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM connections c JOIN connection_generations g ON g.connection_id=c.id WHERE c.installation_id=$1 AND c.id=$2 AND g.generation=$3)").bind(self.installation_id).bind(target.connection_id).bind(target.generation).fetch_one(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
            if !exists {
                return Err(HubError::InvalidSemantics(
                    "deployment connection/generation is not owned",
                ));
            }
            sqlx::query("INSERT INTO profile_draft_targets(installation_id,virtual_model_id,version,ordinal,connection_id,generation,model_id) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(self.installation_id).bind(id).bind(version).bind((ordinal+1) as i32).bind(target.connection_id).bind(target.generation).bind(&target.model_id).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        }
        let row = sqlx::query("SELECT * FROM virtual_models WHERE installation_id=$1 AND id=$2")
            .bind(self.installation_id)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let result = DraftMutation {
            operation_id: operation,
            profile: profile(&row)?,
        };
        sqlx::query("UPDATE operations SET state='succeeded',resource_id=$3,safe_result=$4,version=version+1,updated_at=clock_timestamp() WHERE installation_id=$1 AND id=$2").bind(self.installation_id).bind(operation).bind(id).bind(serde_json::to_value(&result).map_err(|_|HubError::Unavailable)?).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,reason) VALUES($1,$2,$3,$4,$5,$6,'Сохранён draft; публикация и provider I/O не выполнялись')").bind(Uuid::new_v4()).bind(self.installation_id).bind(subject).bind(action).bind(id.to_string()).bind(operation).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        tx.commit().await.map_err(|e| db_failure(e, line!()))?;
        Ok(result)
    }
}
