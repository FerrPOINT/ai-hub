use crate::{financial::db_failure, postgres::PgStore};
use aihub_application::BudgetFilter;
use aihub_domain::{
    NamespaceRef,
    budgets::{Budget, BudgetInput, BudgetMutation, BudgetPeriod, BudgetScope},
    error::HubError,
    financial::{Adjustment, Amount, Currency},
    records::Page,
};
use bigdecimal::BigDecimal;
use sqlx::{Postgres, Row, Transaction};
use std::str::FromStr;
use uuid::Uuid;

// Namespace for client scope comes from the client, never a caller-supplied display name.
const BUDGET_VIEW: &str = "SELECT b.*,n.registry_instance_id,n.namespace_id,COALESCE(p.charged,0::numeric) AS current_charged,COALESCE(p.reserved,0::numeric) AS current_reserved,COALESCE(p.period_start,date_trunc(CASE b.period WHEN 'utc_day' THEN 'day' ELSE 'month' END,now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC') AS current_start,COALESCE(p.period_end,(date_trunc(CASE b.period WHEN 'utc_day' THEN 'day' ELSE 'month' END,now() AT TIME ZONE 'UTC')+CASE b.period WHEN 'utc_day' THEN interval '1 day' ELSE interval '1 month' END) AT TIME ZONE 'UTC') AS current_end FROM budget_policies b LEFT JOIN clients c ON b.scope_type='client' AND c.installation_id=b.installation_id AND c.id::text=b.scope_id LEFT JOIN namespace_bindings n ON n.installation_id=b.installation_id AND n.id=COALESCE(b.namespace_binding_id,c.namespace_binding_id) LEFT JOIN budget_periods p ON p.installation_id=b.installation_id AND p.policy_id=b.id AND p.period_start<=now() AND p.period_end>now()";
const AUTHORIZED: &str = " AND EXISTS(SELECT 1 FROM grants g WHERE g.installation_id=b.installation_id AND g.principal_id=$2 AND g.action='metadata.read' AND g.revoked_at IS NULL AND g.expires_at>now() AND ((n.id IS NOT NULL AND g.namespace_binding_id=n.id AND g.project_binding=n.tracker_project_id::text) OR (n.id IS NULL AND g.namespace_binding_id IS NULL AND g.project_binding='installation')))";

fn amount(row: &sqlx::postgres::PgRow, column: &str) -> Result<Amount, HubError> {
    let value: BigDecimal = row.get(column);
    Amount::parse(&value.normalized().to_plain_string())
}
fn budget(row: &sqlx::postgres::PgRow) -> Result<Budget, HubError> {
    let namespace =
        row.get::<Option<Uuid>, _>("registry_instance_id")
            .map(|registry_instance_id| NamespaceRef {
                registry_instance_id,
                namespace_id: row.get("namespace_id"),
            });
    let scope_type = match row.get::<String, _>("scope_type").as_str() {
        "installation" => BudgetScope::Installation,
        "project" => BudgetScope::Project,
        "client" => BudgetScope::Client,
        "profile" => BudgetScope::Profile,
        _ => return Err(HubError::Unavailable),
    };
    let period = match row.get::<String, _>("period").as_str() {
        "utc_day" => BudgetPeriod::UtcDay,
        "utc_month" => BudgetPeriod::UtcMonth,
        _ => return Err(HubError::Unavailable),
    };
    let scope_id = if scope_type == BudgetScope::Project {
        namespace
            .as_ref()
            .ok_or(HubError::Unavailable)?
            .namespace_id
    } else {
        Uuid::parse_str(&row.get::<String, _>("scope_id")).map_err(|_| HubError::Unavailable)?
    };
    let charged = amount(row, "current_charged")?;
    let reserved = amount(row, "current_reserved")?;
    let cap = amount(row, "hard_limit")?;
    let remaining = Adjustment::from_units(
        cap.units()
            .checked_sub(charged.units())
            .and_then(|v| v.checked_sub(reserved.units()))
            .ok_or(HubError::Invalid("budget total overflow"))?,
    )?;
    Ok(Budget {
        id: row.get("id"),
        policy: BudgetInput {
            scope_type,
            scope_id,
            currency: Currency::parse(&row.get::<String, _>("currency"))?,
            period,
            hard_limit: cap,
            warning_thresholds: serde_json::from_value(row.get("thresholds"))
                .map_err(|_| HubError::Unavailable)?,
            namespace: if scope_type == BudgetScope::Project {
                namespace.clone()
            } else {
                None
            },
        },
        period_start: row.get("current_start"),
        period_end: row.get("current_end"),
        charged,
        reserved,
        remaining,
        version: row.get("version"),
        namespace,
    })
}

impl PgStore {
    async fn resolve_budget_scope(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        subject: &str,
        policy: &BudgetInput,
    ) -> Result<(String, Option<Uuid>, Option<Uuid>), HubError> {
        policy.validate(self.installation_id)?;
        let (scope, stored_namespace, effective_namespace) = match policy.scope_type {
            BudgetScope::Installation => (self.installation_id.to_string(), None, None),
            BudgetScope::Project => {
                let namespace = policy
                    .namespace
                    .as_ref()
                    .ok_or(HubError::Invalid("project Namespace required"))?;
                let row=sqlx::query("SELECT id,tracker_project_id FROM namespace_bindings WHERE installation_id=$1 AND registry_instance_id=$2 AND namespace_id=$3 FOR SHARE")
                    .bind(self.installation_id).bind(namespace.registry_instance_id).bind(namespace.namespace_id).fetch_optional(&mut **tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::Forbidden)?;
                let binding: Uuid = row.get("id");
                (
                    row.get::<Uuid, _>("tracker_project_id").to_string(),
                    Some(binding),
                    Some(binding),
                )
            }
            BudgetScope::Client => {
                let row=sqlx::query("SELECT namespace_binding_id FROM clients WHERE installation_id=$1 AND id=$2 FOR SHARE")
                    .bind(self.installation_id).bind(policy.scope_id).fetch_optional(&mut **tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::NotFound)?;
                (
                    policy.scope_id.to_string(),
                    None,
                    row.get::<Option<Uuid>, _>("namespace_binding_id"),
                )
            }
            // The profile owner table is introduced by S2b; no invented UUID authority.
            BudgetScope::Profile => return Err(HubError::Unavailable),
        };
        sqlx::query("SELECT g.id FROM grants g LEFT JOIN namespace_bindings n ON n.installation_id=g.installation_id AND n.id=$3 WHERE g.installation_id=$1 AND g.principal_id=$2 AND g.action='metadata.read' AND g.revoked_at IS NULL AND g.expires_at>now() AND (($3::uuid IS NULL AND g.namespace_binding_id IS NULL AND g.project_binding='installation') OR (g.namespace_binding_id=n.id AND g.project_binding=n.tracker_project_id::text)) FOR SHARE OF g")
            .bind(self.installation_id).bind(subject).bind(effective_namespace).fetch_optional(&mut **tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::Forbidden)?;
        Ok((scope, stored_namespace, effective_namespace))
    }

    pub(crate) async fn budget_page_internal(
        &self,
        subject: &str,
        filter: &BudgetFilter,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<Page<Budget>, HubError> {
        let query_identity = format!(
            "budgets:{limit}:{}:{}",
            filter
                .namespace
                .as_ref()
                .map(|n| format!("{}/{}", n.registry_instance_id, n.namespace_id))
                .unwrap_or_default(),
            filter.unbound_only
        );
        let initial = if cursor.is_none() {
            let query = format!(
                "{BUDGET_VIEW} WHERE b.installation_id=$1{AUTHORIZED} AND ($3::uuid IS NULL OR (n.registry_instance_id=$3 AND n.namespace_id=$4)) AND (NOT $5 OR n.id IS NULL) ORDER BY b.id LIMIT 10001"
            );
            let rows = sqlx::query(&query)
                .bind(self.installation_id)
                .bind(subject)
                .bind(filter.namespace.as_ref().map(|n| n.registry_instance_id))
                .bind(filter.namespace.as_ref().map(|n| n.namespace_id))
                .bind(filter.unbound_only)
                .fetch_all(&self.pool)
                .await
                .map_err(|err| db_failure(err, line!()))?;
            rows.iter().map(budget).collect::<Result<Vec<_>, _>>()?
        } else {
            vec![]
        };
        let page = self
            .snapshot_page(subject, &query_identity, limit, cursor, initial)
            .await?;
        let ids: Vec<Uuid> = page.items.iter().map(|b| b.id).collect();
        let query = format!(
            "SELECT count(*) FROM ({BUDGET_VIEW} WHERE b.installation_id=$1{AUTHORIZED} AND b.id=ANY($3)) AS permitted"
        );
        let allowed: i64 = sqlx::query_scalar(&query)
            .bind(self.installation_id)
            .bind(subject)
            .bind(ids)
            .fetch_one(&self.pool)
            .await
            .map_err(|err| db_failure(err, line!()))?;
        if allowed != page.items.len() as i64 {
            return Err(HubError::Forbidden);
        }
        Ok(page)
    }

    pub(crate) async fn write_budget_internal(
        &self,
        subject: &str,
        key: Uuid,
        binding: [u8; 32],
        update: Option<(Uuid, i64)>,
        policy: &BudgetInput,
    ) -> Result<BudgetMutation, HubError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| db_failure(err, line!()))?;
        let (scope, namespace, effective_namespace) =
            self.resolve_budget_scope(&mut tx, subject, policy).await?;
        let action = if update.is_some() {
            "budget.update"
        } else {
            "budget.create"
        };
        let (operation, replay) = self
            .begin_control_operation(&mut tx, subject, key, binding, action)
            .await?;
        if let Some(replay) = replay {
            return serde_json::from_value(replay).map_err(|_| HubError::Unavailable);
        }
        if let Some(namespace) = effective_namespace {
            sqlx::query("SELECT id FROM namespace_bindings WHERE installation_id=$1 AND id=$2 AND state='active' FOR SHARE")
                .bind(self.installation_id).bind(namespace).fetch_optional(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::PreconditionFailed)?;
        }
        let id = if let Some((id, version)) = update {
            let existing = sqlx::query(
                "SELECT * FROM budget_policies WHERE installation_id=$1 AND id=$2 FOR UPDATE",
            )
            .bind(self.installation_id)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| db_failure(err, line!()))?
            .ok_or(HubError::NotFound)?;
            if existing.get::<String, _>("scope_type") != policy.scope_type.as_str()
                || existing.get::<String, _>("scope_id") != scope
                || existing.get::<Option<Uuid>, _>("namespace_binding_id") != namespace
            {
                return Err(HubError::Forbidden);
            }
            if existing.get::<i64, _>("version") != version {
                return Err(HubError::PreconditionFailed);
            }
            if existing.get::<String, _>("currency") != policy.currency.to_string()
                || existing.get::<String, _>("period") != policy.period.as_str()
            {
                return Err(HubError::Invalid("budget identity is immutable"));
            }
            sqlx::query("UPDATE budget_policies SET hard_limit=$3,thresholds=$4,version=version+1 WHERE installation_id=$1 AND id=$2")
                .bind(self.installation_id).bind(id).bind(BigDecimal::from_str(&policy.hard_limit.to_string()).map_err(|_|HubError::Invalid("budget amount"))?).bind(serde_json::json!(policy.warning_thresholds)).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
            id
        } else {
            let id = Uuid::new_v4();
            let inserted=sqlx::query("INSERT INTO budget_policies(id,installation_id,scope_type,scope_id,namespace_binding_id,currency,period,hard_limit,thresholds,status) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,'active') ON CONFLICT ON CONSTRAINT budget_scope_currency_period DO NOTHING")
                .bind(id).bind(self.installation_id).bind(policy.scope_type.as_str()).bind(scope).bind(namespace).bind(policy.currency.to_string()).bind(policy.period.as_str()).bind(BigDecimal::from_str(&policy.hard_limit.to_string()).map_err(|_|HubError::Invalid("budget amount"))?).bind(serde_json::json!(policy.warning_thresholds)).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.rows_affected();
            if inserted == 0 {
                return Err(HubError::IdempotencyConflict);
            }
            id
        };
        let query = format!("{BUDGET_VIEW} WHERE b.installation_id=$1 AND b.id=$2");
        let row = sqlx::query(&query)
            .bind(self.installation_id)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|err| db_failure(err, line!()))?;
        let result = BudgetMutation {
            operation_id: operation,
            value: budget(&row)?,
        };
        sqlx::query("UPDATE operations SET state='succeeded',resource_id=$3,safe_result=$4,updated_at=now(),version=version+1 WHERE installation_id=$1 AND id=$2")
            .bind(self.installation_id).bind(operation).bind(id).bind(serde_json::to_value(&result).map_err(|_|HubError::Unavailable)?).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,namespace_binding_id,reason) VALUES($1,$2,$3,$4,$5,$6,$7,'Bounded monetary policy; existing expenses preserved')")
            .bind(Uuid::new_v4()).bind(self.installation_id).bind(subject).bind(action).bind(id.to_string()).bind(operation).bind(effective_namespace).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        tx.commit().await.map_err(|err| db_failure(err, line!()))?;
        Ok(result)
    }
}
