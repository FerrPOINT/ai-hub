use aihub_application::FoundationStore;
use aihub_domain::{
    NamespaceRef,
    error::HubError,
    records::{AuditEvent, NamespaceBinding, Operation, Page},
};
use async_trait::async_trait;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use uuid::Uuid;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

pub struct PgStore {
    pub pool: PgPool,
    pub installation_id: Uuid,
    pub vault_fingerprint: Vec<u8>,
}

impl PgStore {
    pub(crate) async fn snapshot_page<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
        subject: &str,
        query_identity: &str,
        limit: i64,
        cursor: Option<Uuid>,
        initial: Vec<T>,
    ) -> Result<Page<T>, HubError> {
        let mut tx = self.pool.begin().await.map_err(|_| HubError::Unavailable)?;
        let (snapshot_id, items, start) = if let Some(cursor) = cursor {
            let row: Option<(Uuid, serde_json::Value, i32)> = sqlx::query_as("SELECT s.id,s.items,c.position FROM read_cursors c JOIN read_snapshots s ON s.id=c.snapshot_id WHERE c.id=$1 AND s.installation_id=$2 AND s.subject=$3 AND s.query_identity=$4 AND s.expires_at>now()")
                .bind(cursor).bind(self.installation_id).bind(subject).bind(query_identity).fetch_optional(&mut *tx).await.map_err(|_| HubError::Unavailable)?;
            let (id, json, start) =
                row.ok_or(HubError::Invalid("cursor недействителен или истёк"))?;
            let items: Vec<T> = serde_json::from_value(json).map_err(|_| HubError::Unavailable)?;
            (id, items, start as usize)
        } else {
            if initial.len() > 10000 {
                return Err(HubError::Invalid(
                    "projection exceeds 10000 rows; narrow the filter",
                ));
            }
            let id = Uuid::new_v4();
            if initial.len() > limit as usize {
                let json = serde_json::to_value(&initial).map_err(|_| HubError::Unavailable)?;
                sqlx::query("INSERT INTO read_snapshots(id,installation_id,subject,query_identity,items) VALUES($1,$2,$3,$4,$5)")
                    .bind(id).bind(self.installation_id).bind(subject).bind(query_identity).bind(json).execute(&mut *tx).await.map_err(|_| HubError::Unavailable)?;
            }
            (id, initial, 0)
        };
        if start > items.len() {
            return Err(HubError::Invalid("cursor"));
        }
        let end = (start + limit as usize).min(items.len());
        let next_cursor = if end < items.len() {
            let id: Uuid = sqlx::query_scalar("INSERT INTO read_cursors(id,snapshot_id,position) VALUES($1,$2,$3) ON CONFLICT(snapshot_id,position) DO UPDATE SET position=EXCLUDED.position RETURNING id")
                .bind(Uuid::new_v4()).bind(snapshot_id).bind(end as i32).fetch_one(&mut *tx).await.map_err(|_| HubError::Unavailable)?;
            Some(id.to_string())
        } else {
            None
        };
        let items = items.into_iter().skip(start).take(end - start).collect();
        tx.commit().await.map_err(|_| HubError::Unavailable)?;
        Ok(Page { items, next_cursor })
    }
    pub async fn connect(
        dsn: &str,
        installation_id: Uuid,
        vault_fingerprint: Vec<u8>,
    ) -> Result<Self, HubError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(5))
            .connect(dsn)
            .await
            .map_err(|_| HubError::Unavailable)?;
        let role: (bool, bool, bool, String, String) = sqlx::query_as("SELECT rolsuper,rolcreatedb,rolcreaterole,current_database(),current_user FROM pg_roles WHERE rolname=current_user").fetch_one(&pool).await.map_err(|_| HubError::Unavailable)?;
        if role.0
            || role.1
            || role.2
            || !role.3.starts_with("aihub_")
            || !role.4.starts_with("aihub_")
        {
            return Err(HubError::InstallationMismatch);
        }
        Ok(Self {
            pool,
            installation_id,
            vault_fingerprint,
        })
    }

    pub async fn migrate(&self) -> Result<(), HubError> {
        MIGRATOR
            .run(&self.pool)
            .await
            .map_err(|_| HubError::InstallationMismatch)
    }

    /// Explicit bootstrap only; restart never calls this method or migration DDL.
    pub async fn initialize(&self, stable_key: &str) -> Result<(), HubError> {
        if stable_key.is_empty() || stable_key.len() > 120 {
            return Err(HubError::Invalid("stable key"));
        }
        let mut tx = self.pool.begin().await.map_err(|_| HubError::Unavailable)?;
        sqlx::query("SELECT pg_advisory_xact_lock(31881249)")
            .execute(&mut *tx)
            .await
            .map_err(|_| HubError::Unavailable)?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM installations")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| HubError::InstallationMismatch)?;
        if count != 0 {
            return Err(HubError::InstallationMismatch);
        }
        sqlx::query("INSERT INTO installations(id,stable_key,status,vault_key_fingerprint) VALUES ($1,$2,'active',$3)").bind(self.installation_id).bind(stable_key).bind(&self.vault_fingerprint).execute(&mut *tx).await.map_err(|_| HubError::InstallationMismatch)?;
        for (kind, name) in [
            ("openai_compatible", "Совместимый API / OpenRouter"),
            ("ollama", "Ollama"),
            ("zai", "Z.AI"),
            ("chatgpt_managed", "ChatGPT managed"),
        ] {
            sqlx::query(
                "INSERT INTO providers(id,installation_id,kind,display_name) VALUES ($1,$2,$3,$4)",
            )
            .bind(Uuid::new_v4())
            .bind(self.installation_id)
            .bind(kind)
            .bind(name)
            .execute(&mut *tx)
            .await
            .map_err(|_| HubError::Unavailable)?;
        }
        let op = Uuid::new_v4();
        sqlx::query("INSERT INTO operations(id,installation_id,principal_kind,principal_id,idempotency_key,binding_hmac,action,resource_id,state,expires_at) VALUES ($1,$2,'internal','installation-operator',$1,$3,'initialize',$2,'succeeded',now()+interval '30 days')").bind(op).bind(self.installation_id).bind(&self.vault_fingerprint).execute(&mut *tx).await.map_err(|_| HubError::Unavailable)?;
        sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,reason) VALUES ($1,$2,'installation-operator','initialize',$3,$4,'Явная первая установка')").bind(Uuid::new_v4()).bind(self.installation_id).bind(self.installation_id.to_string()).bind(op).execute(&mut *tx).await.map_err(|_| HubError::Unavailable)?;
        tx.commit().await.map_err(|_| HubError::Unavailable)
    }
}

#[async_trait]
impl FoundationStore for PgStore {
    async fn price_page(
        &self,
        subject: &str,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<Page<aihub_domain::prices::PriceRevision>, HubError> {
        self.price_page_internal(subject, limit, cursor).await
    }
    async fn create_price(
        &self,
        subject: &str,
        key: Uuid,
        binding: [u8; 32],
        price: &aihub_domain::prices::PriceInput,
    ) -> Result<aihub_domain::prices::PriceMutation, HubError> {
        self.create_price_internal(subject, key, binding, price)
            .await
    }
    async fn namespace_page(
        &self,
        subject: &str,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<Page<NamespaceBinding>, HubError> {
        let initial = if cursor.is_none() {
            self.namespaces(subject, 10001).await?
        } else {
            vec![]
        };
        let page = self
            .snapshot_page(
                subject,
                &format!("namespaces:{limit}"),
                limit,
                cursor,
                initial,
            )
            .await?;
        for binding in &page.items {
            self.require_namespace(subject, &binding.namespace, "metadata.read")
                .await?;
        }
        Ok(page)
    }
    async fn audit_page(
        &self,
        subject: &str,
        namespace: Option<&NamespaceRef>,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<Page<AuditEvent>, HubError> {
        let initial = if cursor.is_none() {
            self.audit(subject, namespace, 10001).await?
        } else {
            vec![]
        };
        let identity = format!(
            "audit:{limit}:{}",
            namespace
                .map(|n| format!("{}/{}", n.registry_instance_id, n.namespace_id))
                .unwrap_or_else(|| "all".into())
        );
        let page = self
            .snapshot_page(subject, &identity, limit, cursor, initial)
            .await?;
        // Recheck current grants; an old authorised snapshot never survives revocation.
        let ids: Vec<Uuid> = page.items.iter().map(|e| e.id).collect();
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_events a LEFT JOIN namespace_bindings n ON n.installation_id=a.installation_id AND n.id=a.namespace_binding_id WHERE a.installation_id=$1 AND a.id=ANY($3) AND ((a.actor=$2 AND a.namespace_binding_id IS NULL) OR EXISTS(SELECT 1 FROM grants g WHERE g.installation_id=a.installation_id AND g.principal_id=$2 AND g.action='audit.read' AND ((a.namespace_binding_id IS NULL AND g.namespace_binding_id IS NULL AND g.project_binding='installation') OR (g.namespace_binding_id=n.id AND g.project_binding=n.tracker_project_id::text)) AND g.revoked_at IS NULL AND g.expires_at>now()))")
            .bind(self.installation_id).bind(subject).bind(&ids).fetch_one(&self.pool).await.map_err(|_| HubError::Unavailable)?;
        if count != page.items.len() as i64 {
            return Err(HubError::Forbidden);
        }
        Ok(page)
    }
    async fn ready(&self) -> Result<(), HubError> {
        let installations: Vec<(Uuid, Vec<u8>, String)> =
            sqlx::query_as("SELECT id,vault_key_fingerprint,status FROM installations")
                .fetch_all(&self.pool)
                .await
                .map_err(|_| HubError::Unavailable)?;
        if installations.len() != 1
            || installations[0].0 != self.installation_id
            || installations[0].1 != self.vault_fingerprint
            || installations[0].2 != "active"
        {
            return Err(HubError::InstallationMismatch);
        }
        let applied: Vec<(i64, Vec<u8>, bool)> = sqlx::query_as(
            "SELECT version,checksum,success FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| HubError::InstallationMismatch)?;
        let expected: Vec<_> = MIGRATOR
            .iter()
            .filter(|m| !m.migration_type.is_down_migration())
            .collect();
        if applied.len() != expected.len()
            || applied
                .iter()
                .zip(expected)
                .any(|(a, e)| a.0 != e.version || a.1 != e.checksum.as_ref() || !a.2)
        {
            return Err(HubError::InstallationMismatch);
        }
        Ok(())
    }

    async fn project_grants(&self, subject: &str, action: &str) -> Result<Vec<String>, HubError> {
        sqlx::query_scalar("SELECT DISTINCT project_binding FROM grants WHERE installation_id=$1 AND principal_id=$2 AND action=$3 AND revoked_at IS NULL AND expires_at>now() ORDER BY project_binding LIMIT 101").bind(self.installation_id).bind(subject).bind(action).fetch_all(&self.pool).await.map_err(|_| HubError::Unavailable)
    }

    async fn namespaces(
        &self,
        subject: &str,
        limit: i64,
    ) -> Result<Vec<NamespaceBinding>, HubError> {
        let rows = sqlx::query("SELECT n.* FROM namespace_bindings n WHERE n.installation_id=$1 AND EXISTS (SELECT 1 FROM grants g WHERE g.installation_id=n.installation_id AND g.principal_id=$2 AND g.action='metadata.read' AND g.namespace_binding_id=n.id AND g.project_binding=n.tracker_project_id::text AND g.revoked_at IS NULL AND g.expires_at>now()) ORDER BY n.registry_instance_id,n.namespace_id LIMIT $3").bind(self.installation_id).bind(subject).bind(limit).fetch_all(&self.pool).await.map_err(|_| HubError::Unavailable)?;
        rows.into_iter()
            .map(|r| {
                Ok(NamespaceBinding {
                    namespace: NamespaceRef {
                        registry_instance_id: r
                            .try_get("registry_instance_id")
                            .map_err(|_| HubError::Unavailable)?,
                        namespace_id: r
                            .try_get("namespace_id")
                            .map_err(|_| HubError::Unavailable)?,
                    },
                    tracker_instance_id: r
                        .try_get("tracker_instance_id")
                        .map_err(|_| HubError::Unavailable)?,
                    tracker_project_id: r
                        .try_get("tracker_project_id")
                        .map_err(|_| HubError::Unavailable)?,
                    state: r.try_get("state").map_err(|_| HubError::Unavailable)?,
                    generation: r.try_get("generation").map_err(|_| HubError::Unavailable)?,
                    observed_at: r
                        .try_get("observed_at")
                        .map_err(|_| HubError::Unavailable)?,
                    label: r.try_get("label").map_err(|_| HubError::Unavailable)?,
                    tracker_project_key: r
                        .try_get("tracker_project_key")
                        .map_err(|_| HubError::Unavailable)?,
                })
            })
            .collect()
    }

    async fn require_namespace(
        &self,
        subject: &str,
        namespace: &NamespaceRef,
        action: &str,
    ) -> Result<(), HubError> {
        let allowed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM namespace_bindings n JOIN grants g ON g.installation_id=n.installation_id AND g.namespace_binding_id=n.id AND g.project_binding=n.tracker_project_id::text WHERE n.installation_id=$1 AND n.registry_instance_id=$2 AND n.namespace_id=$3 AND g.principal_id=$4 AND g.action=$5 AND g.revoked_at IS NULL AND g.expires_at>now())").bind(self.installation_id).bind(namespace.registry_instance_id).bind(namespace.namespace_id).bind(subject).bind(action).fetch_one(&self.pool).await.map_err(|_| HubError::Unavailable)?;
        if allowed {
            Ok(())
        } else {
            Err(HubError::Forbidden)
        }
    }

    async fn audit(
        &self,
        subject: &str,
        namespace: Option<&NamespaceRef>,
        limit: i64,
    ) -> Result<Vec<AuditEvent>, HubError> {
        let rows = sqlx::query("SELECT a.*,n.registry_instance_id,n.namespace_id FROM audit_events a LEFT JOIN namespace_bindings n ON n.installation_id=a.installation_id AND n.id=a.namespace_binding_id WHERE a.installation_id=$1 AND ($3::uuid IS NULL OR (n.registry_instance_id=$3 AND n.namespace_id=$4)) AND ((a.actor=$2 AND a.namespace_binding_id IS NULL) OR EXISTS(SELECT 1 FROM grants g WHERE g.installation_id=a.installation_id AND g.principal_id=$2 AND g.action='audit.read' AND ((a.namespace_binding_id IS NULL AND g.namespace_binding_id IS NULL AND g.project_binding='installation') OR (g.namespace_binding_id=n.id AND g.project_binding=n.tracker_project_id::text)) AND g.revoked_at IS NULL AND g.expires_at>now())) ORDER BY a.happened_at DESC,a.id LIMIT $5").bind(self.installation_id).bind(subject).bind(namespace.map(|n|n.registry_instance_id)).bind(namespace.map(|n|n.namespace_id)).bind(limit).fetch_all(&self.pool).await.map_err(|_| HubError::Unavailable)?;
        rows.into_iter()
            .map(|r| {
                let registry: Option<Uuid> = r
                    .try_get("registry_instance_id")
                    .map_err(|_| HubError::Unavailable)?;
                Ok(AuditEvent {
                    id: r.try_get("id").map_err(|_| HubError::Unavailable)?,
                    actor: r.try_get("actor").map_err(|_| HubError::Unavailable)?,
                    action: r.try_get("action").map_err(|_| HubError::Unavailable)?,
                    object_id: r.try_get("object_id").map_err(|_| HubError::Unavailable)?,
                    revision: r.try_get("revision").map_err(|_| HubError::Unavailable)?,
                    operation_id: r
                        .try_get("operation_id")
                        .map_err(|_| HubError::Unavailable)?,
                    reason: r.try_get("reason").map_err(|_| HubError::Unavailable)?,
                    happened_at: r
                        .try_get("happened_at")
                        .map_err(|_| HubError::Unavailable)?,
                    namespace: registry.map(|registry_instance_id| NamespaceRef {
                        registry_instance_id,
                        namespace_id: r.get("namespace_id"),
                    }),
                })
            })
            .collect()
    }

    async fn operation(&self, subject: &str, id: Uuid) -> Result<Operation, HubError> {
        let row = sqlx::query("SELECT id,state,resource_id,version FROM operations WHERE installation_id=$1 AND id=$2 AND principal_kind='human' AND principal_id=$3").bind(self.installation_id).bind(id).bind(subject).fetch_optional(&self.pool).await.map_err(|_| HubError::Unavailable)?.ok_or(HubError::NotFound)?;
        Ok(Operation {
            id: row.get("id"),
            status: row.get("state"),
            resource_id: row.get("resource_id"),
            safe_error: None,
            version: row.get("version"),
        })
    }
}
