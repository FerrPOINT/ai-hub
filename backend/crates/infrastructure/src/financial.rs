//! One durable admission transaction. Provider transport is deliberately outside it.
use crate::postgres::PgStore;
use aihub_application::FinancialAdmission;
use aihub_domain::{
    admission::{AdmissionIntent, AdmissionReceipt, DispatchClaim, Purpose, PurposeBounds},
    error::HubError,
    financial::{Amount, Category, Currency, Price, Rate},
};
use async_trait::async_trait;
use bigdecimal::BigDecimal;
use sqlx::Row;
use std::{collections::BTreeMap, str::FromStr};
use uuid::Uuid;

pub(crate) fn db_failure(error: sqlx::Error, line: u32) -> HubError {
    // Never log SQL text, bound values or database detail: they may contain private data.
    let code = error
        .as_database_error()
        .and_then(|e| e.code())
        .map(|c| c.into_owned());
    tracing::warn!(db_code=?code,source_line=line,"financial transaction failed");
    HubError::Unavailable
}

fn decimal(amount: Amount) -> Result<BigDecimal, HubError> {
    BigDecimal::from_str(&amount.to_string()).map_err(|_| HubError::Invalid("amount codec"))
}
fn amount(row: &sqlx::postgres::PgRow, name: &str) -> Result<Amount, HubError> {
    let value: BigDecimal = row.try_get(name).map_err(|err| db_failure(err, line!()))?;
    Amount::parse(&value.to_plain_string())
}
fn optional_amount(row: &sqlx::postgres::PgRow, name: &str) -> Result<Option<Amount>, HubError> {
    let value: Option<BigDecimal> = row.try_get(name).map_err(|err| db_failure(err, line!()))?;
    value
        .map(|v| Amount::parse(&v.to_plain_string()))
        .transpose()
}
fn rates(row: &sqlx::postgres::PgRow) -> Result<BTreeMap<Category, Rate>, HubError> {
    let mut rates = BTreeMap::new();
    for (category, column) in [
        (Category::InputUncached, "input_uncached"),
        (Category::InputCached, "input_cached"),
        (Category::CacheWrite, "cache_write"),
        (Category::OutputBillable, "output_billable"),
        (Category::ExplicitOther, "explicit_other"),
    ] {
        let value: Option<BigDecimal> = row
            .try_get(column)
            .map_err(|err| db_failure(err, line!()))?;
        if let Some(value) = value {
            rates.insert(category, Rate::parse(&value.to_plain_string())?);
        }
    }
    Ok(rates)
}

#[async_trait]
impl FinancialAdmission for PgStore {
    async fn settle(
        &self,
        fact: &aihub_domain::settlement::SettlementFact,
    ) -> Result<aihub_domain::settlement::SettlementReceipt, HubError> {
        self.settle_fact(fact).await
    }
    async fn claim_dispatch(
        &self,
        attempt_id: Uuid,
        owner_id: Uuid,
        lease_seconds: i32,
    ) -> Result<DispatchClaim, HubError> {
        if attempt_id.is_nil() || owner_id.is_nil() || !(5..=120).contains(&lease_seconds) {
            return Err(HubError::Invalid("dispatch claim"));
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| db_failure(err, line!()))?;
        let context = sqlx::query("SELECT r.id,r.client_id,r.grant_id,r.principal_id,r.project_binding,r.namespace_binding_id,r.operation_id FROM attempts a JOIN requests r ON r.installation_id=a.installation_id AND r.id=a.request_id WHERE a.installation_id=$1 AND a.id=$2")
            .bind(self.installation_id).bind(attempt_id).fetch_optional(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::NotFound)?;
        // Keep the admission lock order. Revocation and disabling cannot race a new claim.
        sqlx::query("SELECT id FROM clients WHERE installation_id=$1 AND id=$2 AND status='enabled' AND expires_at>now() FOR UPDATE")
            .bind(self.installation_id).bind(context.get::<Uuid,_>("client_id")).fetch_optional(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::Forbidden)?;
        sqlx::query("SELECT id FROM grants WHERE installation_id=$1 AND id=$2 AND client_id=$3 AND principal_id=$4 AND project_binding=$5 AND namespace_binding_id IS NOT DISTINCT FROM $6 AND revoked_at IS NULL AND expires_at>now() FOR UPDATE")
            .bind(self.installation_id).bind(context.get::<Uuid,_>("grant_id")).bind(context.get::<Uuid,_>("client_id")).bind(context.get::<String,_>("principal_id")).bind(context.get::<String,_>("project_binding")).bind(context.get::<Option<Uuid>,_>("namespace_binding_id")).fetch_optional(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::Forbidden)?;
        let request = sqlx::query("SELECT state,cancel_requested FROM requests WHERE installation_id=$1 AND id=$2 FOR UPDATE")
            .bind(self.installation_id).bind(context.get::<Uuid,_>("id")).fetch_one(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        if request.get::<String, _>("state") != "admitted"
            || request.get::<bool, _>("cancel_requested")
        {
            return Err(HubError::PreconditionFailed);
        }
        if let Some(namespace) = context.get::<Option<Uuid>, _>("namespace_binding_id") {
            sqlx::query("SELECT id FROM namespace_bindings WHERE installation_id=$1 AND id=$2 AND state='active' AND tracker_project_id::text=$3 FOR SHARE")
                .bind(self.installation_id).bind(namespace).bind(context.get::<String,_>("project_binding")).fetch_optional(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::Forbidden)?;
        }
        let attempt =
            sqlx::query("SELECT * FROM attempts WHERE installation_id=$1 AND id=$2 FOR UPDATE")
                .bind(self.installation_id)
                .bind(attempt_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|err| db_failure(err, line!()))?;
        if attempt.get::<String, _>("state") != "intended"
            || attempt.get::<Option<Uuid>, _>("dispatch_fence").is_some()
        {
            return Err(HubError::PreconditionFailed);
        }
        let snapshot: serde_json::Value = attempt.get("deployment_snapshot");
        let target = snapshot.get("target").ok_or(HubError::Unavailable)?;
        // Lock each mutable owner through commit. A newer generation or expired proof invalidates dispatch.
        sqlx::query("SELECT q.id FROM runtime_qualifications q JOIN connections c ON c.installation_id=q.installation_id AND c.id=q.connection_id JOIN connection_generations g ON g.connection_id=q.connection_id AND g.generation=q.generation JOIN verification_evidence e ON e.installation_id=q.installation_id AND e.id=q.proof_id WHERE q.installation_id=$1 AND q.id=$2 AND q.connection_id=$3 AND q.generation=$4 AND q.provider_model_id=$5 AND q.state='active' AND c.status='enabled' AND c.generation=q.generation AND g.authorization_state='active' AND g.adapter_revision=q.adapter_revision AND g.endpoint_policy_hash=q.endpoint_policy_hash AND q.adapter_revision=$6 AND q.endpoint_policy_hash=$7 AND q.billing_currency=$8 AND e.state='verified' AND e.expires_at>now() FOR SHARE OF q,c,g,e")
            .bind(self.installation_id).bind(attempt.get::<Uuid,_>("qualification_id")).bind(attempt.get::<Uuid,_>("connection_id")).bind(attempt.get::<i64,_>("generation")).bind(target.get("model_id").and_then(|v|v.as_str()).ok_or(HubError::Unavailable)?).bind(snapshot.get("adapter_revision").and_then(|v|v.as_str()).ok_or(HubError::Unavailable)?).bind(snapshot.get("endpoint_policy_hash").and_then(|v|v.as_str()).ok_or(HubError::Unavailable)?).bind(attempt.get::<String,_>("currency")).fetch_optional(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::PreconditionFailed)?;
        let fence = Uuid::new_v4();
        sqlx::query("UPDATE attempts SET state='dispatched',accepted='unknown',dispatched_at=now(),dispatch_owner_id=$3,dispatch_fence=$4,dispatch_lease_until=now()+make_interval(secs=>$5),version=version+1 WHERE installation_id=$1 AND id=$2")
            .bind(self.installation_id).bind(attempt_id).bind(owner_id).bind(fence).bind(lease_seconds as f64).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        sqlx::query("UPDATE requests SET state='dispatching',version=version+1 WHERE installation_id=$1 AND id=$2")
            .bind(self.installation_id).bind(context.get::<Uuid,_>("id")).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,reason,operation_id,namespace_binding_id) VALUES($1,$2,$3,'request.dispatch',$4,'Durable one-send claim',$5,$6)")
            .bind(Uuid::new_v4()).bind(self.installation_id).bind(context.get::<String,_>("principal_id")).bind(attempt_id.to_string()).bind(context.get::<Uuid,_>("operation_id")).bind(context.get::<Option<Uuid>,_>("namespace_binding_id")).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        tx.commit().await.map_err(|err| db_failure(err, line!()))?;
        Ok(DispatchClaim {
            attempt_id,
            owner_id,
            fence,
            deployment_snapshot: snapshot,
        })
    }

    async fn record_uncertain(&self, claim: &DispatchClaim) -> Result<(), HubError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| db_failure(err, line!()))?;
        let row=sqlx::query("SELECT r.id AS request_id,r.operation_id,r.principal_id,r.namespace_binding_id FROM requests r JOIN attempts a ON a.installation_id=r.installation_id AND a.request_id=r.id WHERE a.installation_id=$1 AND a.id=$2 AND a.dispatch_owner_id=$3 AND a.dispatch_fence=$4 FOR UPDATE OF r")
            .bind(self.installation_id).bind(claim.attempt_id).bind(claim.owner_id).bind(claim.fence).fetch_optional(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::PreconditionFailed)?;
        let changed=sqlx::query("UPDATE attempts SET state='unknown',accepted='unknown',version=version+1 WHERE installation_id=$1 AND id=$2 AND state IN ('dispatched','streaming')")
            .bind(self.installation_id).bind(claim.attempt_id).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.rows_affected();
        if changed == 0 {
            let state: String =
                sqlx::query_scalar("SELECT state FROM attempts WHERE installation_id=$1 AND id=$2")
                    .bind(self.installation_id)
                    .bind(claim.attempt_id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|err| db_failure(err, line!()))?;
            return if state == "unknown" {
                Ok(())
            } else {
                Err(HubError::PreconditionFailed)
            };
        }
        sqlx::query("UPDATE requests SET state='unknown',version=version+1 WHERE installation_id=$1 AND id=$2 AND state IN ('dispatching','streaming','unknown')")
            .bind(self.installation_id).bind(row.get::<Uuid,_>("request_id")).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,namespace_binding_id,reason) VALUES($1,$2,$3,'request.unknown',$4,$5,$6,'Uncertain provider acceptance; reserve held')")
            .bind(Uuid::new_v4()).bind(self.installation_id).bind(row.get::<String,_>("principal_id")).bind(claim.attempt_id.to_string()).bind(row.get::<Uuid,_>("operation_id")).bind(row.get::<Option<Uuid>,_>("namespace_binding_id")).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        tx.commit().await.map_err(|err| db_failure(err, line!()))
    }

    async fn recover_expired_dispatches(&self) -> Result<u64, HubError> {
        // An expired sender is fenced from redispatch; recovery only records uncertainty.
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| db_failure(err, line!()))?;
        let rows=sqlx::query("SELECT r.id,r.operation_id,r.principal_id,r.namespace_binding_id FROM requests r WHERE r.installation_id=$1 AND EXISTS(SELECT 1 FROM attempts a WHERE a.installation_id=r.installation_id AND a.request_id=r.id AND a.state IN ('dispatched','streaming') AND a.dispatch_lease_until<=now()) ORDER BY r.id LIMIT 100 FOR UPDATE OF r SKIP LOCKED")
            .bind(self.installation_id).fetch_all(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        let mut recovered = 0;
        for row in &rows {
            recovered+=sqlx::query("UPDATE attempts SET state='unknown',accepted='unknown',version=version+1 WHERE installation_id=$1 AND request_id=$2 AND state IN ('dispatched','streaming') AND dispatch_lease_until<=now()")
                .bind(self.installation_id).bind(row.get::<Uuid,_>("id")).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.rows_affected();
            sqlx::query("UPDATE requests SET state='unknown',version=version+1 WHERE installation_id=$1 AND id=$2 AND state IN ('dispatching','streaming')")
                .bind(self.installation_id).bind(row.get::<Uuid,_>("id")).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
            sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,namespace_binding_id,reason) VALUES($1,$2,$3,'request.recover',$4,$5,$6,'Expired dispatch lease; reserve held')")
                .bind(Uuid::new_v4()).bind(self.installation_id).bind(row.get::<String,_>("principal_id")).bind(row.get::<Uuid,_>("id").to_string()).bind(row.get::<Uuid,_>("operation_id")).bind(row.get::<Option<Uuid>,_>("namespace_binding_id")).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        }
        tx.commit().await.map_err(|err| db_failure(err, line!()))?;
        Ok(recovered)
    }

    async fn reserve(&self, intent: &AdmissionIntent) -> Result<AdmissionReceipt, HubError> {
        // S2 verification is the first consumer. Public/evaluation admission is exposed by S3/S6.
        if intent.purpose != Purpose::Verification {
            return Err(HubError::Unavailable);
        }
        if intent.client_id.is_nil()
            || intent.grant_id.is_nil()
            || intent.idempotency_key.is_nil()
            || intent.connection_id.is_nil()
            || intent.qualification_id.is_nil()
            || intent.generation < 1
            || intent.model_id.is_empty()
            || intent.model_id.len() > 256
            || intent.tier.is_empty()
            || intent.tier.len() > 120
            || intent.principal_id.is_empty()
            || intent.principal_id.len() > 256
            || intent.profile_revision_id.is_some()
            || intent.probe_snapshot_id.is_none_or(|id| id.is_nil())
            || !intent.upper_usage.complete
            || intent.upper_usage.source.is_empty()
            || intent.upper_usage.source.len() > 256
            || ![
                Category::InputUncached,
                Category::InputCached,
                Category::OutputBillable,
            ]
            .iter()
            .all(|category| intent.upper_usage.categories.contains_key(category))
            || intent
                .upper_usage
                .categories
                .values()
                .any(|tokens| *tokens > i64::MAX as u64)
        {
            return Err(HubError::Invalid("admission context"));
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| db_failure(err, line!()))?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
            .execute(&mut *tx)
            .await
            .map_err(|err| db_failure(err, line!()))?;
        let client=sqlx::query("SELECT * FROM clients WHERE installation_id=$1 AND id=$2 AND status='enabled' AND expires_at>now() FOR UPDATE")
            .bind(self.installation_id).bind(intent.client_id).fetch_optional(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::Forbidden)?;
        let project: String = client.get("project_binding");
        let namespace: Option<Uuid> = client.get("namespace_binding_id");
        let previous=sqlx::query("SELECT r.id,r.state,r.principal_id,r.payload_hmac,r.grant_id,r.wire_protocol,r.streaming,a.id AS attempt_id,a.currency,a.upper_provider_cost FROM requests r JOIN attempts a ON a.installation_id=r.installation_id AND a.request_id=r.id AND a.ordinal=1 WHERE r.installation_id=$1 AND r.client_id=$2 AND r.idempotency_key=$3")
            .bind(self.installation_id).bind(intent.client_id).bind(intent.idempotency_key).fetch_optional(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        if let Some(previous) = previous {
            let hash: Vec<u8> = previous.get("payload_hmac");
            if hash.as_slice() != intent.payload_hmac
                || previous.get::<String, _>("principal_id") != intent.principal_id
                || previous.get::<Uuid, _>("grant_id") != intent.grant_id
                || previous
                    .get::<Option<String>, _>("wire_protocol")
                    .as_deref()
                    != Some(intent.protocol.as_str())
                || previous.get::<Option<bool>, _>("streaming") != Some(intent.streaming)
            {
                return Err(HubError::IdempotencyConflict);
            }
            return Ok(AdmissionReceipt {
                request_id: previous.get("id"),
                attempt_id: previous.get("attempt_id"),
                state: previous.get("state"),
                replay: true,
                upper_provider_cost: optional_amount(&previous, "upper_provider_cost")?,
                currency: Currency::parse(&previous.get::<String, _>("currency"))?,
            });
        }
        if let Some(namespace) = namespace {
            let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM namespace_bindings WHERE installation_id=$1 AND id=$2 AND state='active' AND tracker_project_id::text=$3)")
                .bind(self.installation_id).bind(namespace).bind(&project).fetch_one(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
            if !active {
                return Err(HubError::Forbidden);
            }
        }
        let grant=sqlx::query("SELECT bounds FROM grants WHERE installation_id=$1 AND id=$2 AND client_id=$3 AND principal_id=$4 AND action=$5 AND project_binding=$6 AND namespace_binding_id IS NOT DISTINCT FROM $7 AND revoked_at IS NULL AND expires_at>now() FOR UPDATE")
            .bind(self.installation_id).bind(intent.grant_id).bind(intent.client_id).bind(&intent.principal_id).bind(intent.purpose.action()).bind(&project).bind(namespace).fetch_optional(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::Forbidden)?;
        let bounds: PurposeBounds = serde_json::from_value(grant.get("bounds"))
            .map_err(|_| HubError::Invalid("purpose grant bounds"))?;
        bounds.validate()?;
        if bounds.connection_id != intent.connection_id
            || bounds.generation != intent.generation
            || bounds.model_id != intent.model_id
        {
            return Err(HubError::Forbidden);
        }
        let input = intent
            .upper_usage
            .categories
            .iter()
            .filter(|(category, _)| **category != Category::OutputBillable)
            .try_fold(0_u64, |sum, (_, tokens)| sum.checked_add(*tokens))
            .ok_or(HubError::Invalid("token bound overflow"))?;
        let output = intent
            .upper_usage
            .categories
            .get(&Category::OutputBillable)
            .copied()
            .ok_or(HubError::Invalid("output bound required"))?;
        if input > bounds.max_input_tokens || output > bounds.max_output_tokens {
            return Err(HubError::Forbidden);
        }
        let qualification=sqlx::query("SELECT q.billing_currency,q.adapter_revision,q.endpoint_policy_hash,q.capabilities FROM runtime_qualifications q JOIN connections c ON c.installation_id=q.installation_id AND c.id=q.connection_id JOIN connection_generations g ON g.connection_id=q.connection_id AND g.generation=q.generation JOIN verification_evidence e ON e.installation_id=q.installation_id AND e.id=q.proof_id WHERE q.installation_id=$1 AND q.id=$2 AND q.connection_id=$3 AND q.generation=$4 AND q.provider_model_id=$5 AND q.state='active' AND q.billing_currency IS NOT NULL AND q.billing_currency_origin IS NOT NULL AND c.status='enabled' AND c.generation=q.generation AND g.authorization_state='active' AND g.adapter_revision=q.adapter_revision AND g.endpoint_policy_hash=q.endpoint_policy_hash AND e.state='verified' AND e.expires_at>now()")
            .bind(self.installation_id).bind(intent.qualification_id).bind(intent.connection_id).bind(intent.generation).bind(&intent.model_id).fetch_optional(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::Forbidden)?;
        let currency = Currency::parse(&qualification.get::<String, _>("billing_currency"))?;
        if currency != bounds.currency {
            return Err(HubError::Forbidden);
        }
        let probe=sqlx::query("SELECT configuration,draft_model_id,operation_id FROM probe_snapshots WHERE installation_id=$1 AND id=$2")
            .bind(self.installation_id).bind(intent.probe_snapshot_id).fetch_optional(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::NotFound)?;
        let configuration: serde_json::Value = probe.get("configuration");
        let target = serde_json::json!({"connection_id":intent.connection_id,"generation":intent.generation,"model_id":intent.model_id,"tier":intent.tier,"price_revision_id":intent.price_revision_id,"qualification_id":intent.qualification_id,"upper_usage":intent.upper_usage,"protocol":intent.protocol,"streaming":intent.streaming});
        if !configuration
            .get("targets")
            .and_then(|v| v.as_array())
            .is_some_and(|targets| targets.contains(&target))
        {
            return Err(HubError::PreconditionFailed);
        }
        let profile: Option<Uuid> = probe.get("draft_model_id");
        let (price, upper) = if let Some(price_id) = intent.price_revision_id {
            let row=sqlx::query("SELECT * FROM price_revisions WHERE installation_id=$1 AND id=$2 AND connection_id=$3 AND model_id=$4 AND tier=$5 AND currency=$6 AND effective_from<=now() AND (effective_to IS NULL OR effective_to>now())")
                .bind(self.installation_id).bind(price_id).bind(intent.connection_id).bind(&intent.model_id).bind(&intent.tier).bind(currency.to_string()).fetch_optional(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::PreconditionFailed)?;
            let price = Price {
                currency: currency.clone(),
                rates: rates(&row)?,
                request_fee: optional_amount(&row, "request_fee")?,
            };
            let upper = price.cost(&intent.upper_usage)?;
            (Some(price), upper)
        } else {
            (None, None)
        };
        let budgets=sqlx::query("SELECT * FROM budget_policies WHERE installation_id=$1 AND status='active' AND ((scope_type='installation' AND scope_id=$2) OR (scope_type='client' AND scope_id=$3) OR (scope_type='project' AND namespace_binding_id=$4 AND scope_id=$5) OR (scope_type='profile' AND scope_id=$6)) ORDER BY CASE scope_type WHEN 'installation' THEN 0 WHEN 'project' THEN 1 WHEN 'client' THEN 2 ELSE 3 END,scope_id,currency,id FOR UPDATE")
            .bind(self.installation_id).bind(self.installation_id.to_string()).bind(intent.client_id.to_string()).bind(namespace).bind(&project).bind(profile.map(|id|id.to_string())).fetch_all(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        if budgets
            .iter()
            .any(|budget| budget.get::<String, _>("currency") != currency.to_string())
        {
            return Err(HubError::Forbidden);
        }
        if upper.is_none()
            && (!bounds.cost_unknown_allowed
                || client.get::<String, _>("cost_policy") != "cost_unknown_allowed"
                || bounds.max_total_provider_cost.is_some()
                || !budgets.is_empty())
        {
            return Err(HubError::BudgetExceeded);
        }
        let active:(i64,i64)=sqlx::query_as("SELECT count(*) FILTER(WHERE state IN ('admitted','dispatching','streaming','unknown')),count(*) FILTER(WHERE admitted_at>=date_trunc('minute',now())) FROM requests WHERE installation_id=$1 AND client_id=$2")
            .bind(self.installation_id).bind(intent.client_id).fetch_one(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        if active.0
            >= i64::from(client.get::<i32, _>("max_concurrency"))
                .min(i64::from(bounds.max_concurrency))
            || active.1 >= i64::from(client.get::<i32, _>("max_rpm"))
        {
            return Err(HubError::BudgetExceeded);
        }
        sqlx::query("INSERT INTO grant_accounts(installation_id,grant_id,currency) VALUES($1,$2,$3) ON CONFLICT(grant_id) DO NOTHING").bind(self.installation_id).bind(intent.grant_id).bind(currency.to_string()).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        let account = sqlx::query(
            "SELECT * FROM grant_accounts WHERE installation_id=$1 AND grant_id=$2 FOR UPDATE",
        )
        .bind(self.installation_id)
        .bind(intent.grant_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|err| db_failure(err, line!()))?;
        if account.get::<String, _>("currency") != currency.to_string()
            || account.get::<i64, _>("requests_count") >= bounds.max_requests as i64
        {
            return Err(HubError::BudgetExceeded);
        }
        if let (Some(upper), Some(cap)) = (upper, bounds.max_total_provider_cost) {
            if amount(&account, "charged")?
                .checked_add(amount(&account, "reserved")?)?
                .checked_add(upper)?
                > cap
            {
                return Err(HubError::BudgetExceeded);
            }
        }
        let request_id = Uuid::new_v4();
        let attempt_id = Uuid::new_v4();
        let mut period_ids = Vec::new();
        if let Some(upper) = upper {
            for budget in &budgets {
                let policy: Uuid = budget.get("id");
                let period: String = budget.get("period");
                let grain = match period.as_str() {
                    "utc_day" => "day",
                    "utc_month" => "month",
                    _ => return Err(HubError::Unavailable),
                };
                let row=sqlx::query("INSERT INTO budget_periods(id,installation_id,policy_id,period_start,period_end) VALUES($1,$2,$3,date_trunc($4,now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC',(date_trunc($4,now() AT TIME ZONE 'UTC')+CASE WHEN $4='day' THEN interval '1 day' ELSE interval '1 month' END) AT TIME ZONE 'UTC') ON CONFLICT(policy_id,period_start) DO UPDATE SET policy_id=EXCLUDED.policy_id RETURNING *")
                    .bind(Uuid::new_v4()).bind(self.installation_id).bind(policy).bind(grain).fetch_one(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
                if amount(&row, "charged")?
                    .checked_add(amount(&row, "reserved")?)?
                    .checked_add(upper)?
                    > amount(budget, "hard_limit")?
                {
                    return Err(HubError::BudgetExceeded);
                }
                period_ids.push(row.get::<Uuid, _>("id"));
            }
        }
        sqlx::query("INSERT INTO requests(id,installation_id,client_id,grant_id,principal_id,project_binding,request_kind,probe_snapshot_id,idempotency_key,payload_hmac,state,namespace_binding_id,operation_id,wire_protocol,streaming) VALUES($1,$2,$3,$4,$5,$6,'verification',$7,$8,$9,'admitted',$10,$11,$12,$13)")
            .bind(request_id).bind(self.installation_id).bind(intent.client_id).bind(intent.grant_id).bind(&intent.principal_id).bind(&project).bind(intent.probe_snapshot_id).bind(intent.idempotency_key).bind(intent.payload_hmac.as_slice()).bind(namespace).bind(probe.get::<Uuid,_>("operation_id")).bind(intent.protocol.as_str()).bind(intent.streaming).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        let deployment = serde_json::json!({"target":target,"price":price,"qualified_capabilities":qualification.get::<serde_json::Value,_>("capabilities"),"cost_source_state":if intent.price_revision_id.is_some(){"configured"}else{"unconfigured"},"adapter_revision":qualification.get::<String,_>("adapter_revision"),"endpoint_policy_hash":qualification.get::<String,_>("endpoint_policy_hash")});
        sqlx::query("INSERT INTO attempts(id,installation_id,request_id,ordinal,connection_id,generation,deployment_snapshot,state,accepted,qualification_id,upper_provider_cost,currency) VALUES($1,$2,$3,1,$4,$5,$6,'intended','not_accepted',$7,$8,$9)")
            .bind(attempt_id).bind(self.installation_id).bind(request_id).bind(intent.connection_id).bind(intent.generation).bind(deployment).bind(intent.qualification_id).bind(upper.map(decimal).transpose()?).bind(currency.to_string()).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        if let Some(upper) = upper {
            for period in period_ids {
                sqlx::query("UPDATE budget_periods SET reserved=reserved+$3,version=version+1 WHERE installation_id=$1 AND id=$2").bind(self.installation_id).bind(period).bind(decimal(upper)?).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
                sqlx::query("INSERT INTO reservations(installation_id,attempt_id,budget_period_id,amount,currency,state) VALUES($1,$2,$3,$4,$5,'held')").bind(self.installation_id).bind(attempt_id).bind(period).bind(decimal(upper)?).bind(currency.to_string()).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
            }
            sqlx::query("INSERT INTO ledger_entries(id,installation_id,attempt_id,charge_key,kind,amount,currency,confidence,price_revision_id,source,source_event_id,evidence) VALUES($1,$2,$3,$4,'reserve',$5,$6,'estimated',$7,'admission',$4,$8)")
                .bind(Uuid::new_v4()).bind(self.installation_id).bind(attempt_id).bind(format!("reserve:{attempt_id}")).bind(decimal(upper)?).bind(currency.to_string()).bind(intent.price_revision_id).bind(serde_json::json!({"grant_id":intent.grant_id,"upper_usage":intent.upper_usage})).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        }
        sqlx::query("UPDATE grant_accounts SET requests_count=requests_count+1,reserved=reserved+COALESCE($3,0),version=version+1 WHERE installation_id=$1 AND grant_id=$2").bind(self.installation_id).bind(intent.grant_id).bind(upper.map(decimal).transpose()?).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,namespace_binding_id,reason) VALUES($1,$2,$3,'request.reserve',$4,$5,$6,'Bounded purpose admission')")
            .bind(Uuid::new_v4()).bind(self.installation_id).bind(&intent.principal_id).bind(attempt_id.to_string()).bind(probe.get::<Uuid,_>("operation_id")).bind(namespace).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        tx.commit().await.map_err(|err| db_failure(err, line!()))?;
        Ok(AdmissionReceipt {
            request_id,
            attempt_id,
            state: "admitted".into(),
            replay: false,
            upper_provider_cost: upper,
            currency,
        })
    }
}
