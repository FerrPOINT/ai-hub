//! Atomic terminal facts, effective expense, reservations and audit.
use crate::{financial::db_failure, postgres::PgStore};
use aihub_domain::{
    error::HubError,
    financial::{Acceptance, Adjustment, Amount, Price},
    settlement::{SettlementFact, SettlementReceipt},
};
use bigdecimal::BigDecimal;
use sqlx::Row;
use std::str::FromStr;
use uuid::Uuid;

fn decimal(value: impl ToString) -> Result<BigDecimal, HubError> {
    BigDecimal::from_str(&value.to_string()).map_err(|_| HubError::Invalid("settlement decimal"))
}
fn row_amount(row: &sqlx::postgres::PgRow, column: &str) -> Result<Option<Amount>, HubError> {
    let value: Option<BigDecimal> = row.try_get(column).map_err(|_| HubError::Unavailable)?;
    value
        .map(|v| Amount::parse(&v.to_plain_string()))
        .transpose()
}

impl PgStore {
    pub(crate) async fn settle_fact(
        &self,
        fact: &SettlementFact,
    ) -> Result<SettlementReceipt, HubError> {
        if fact.attempt_id.is_nil()
            || fact.owner_id.is_nil()
            || fact.fence.is_nil()
            || fact.source.is_empty()
            || fact.source.len() > 256
            || fact.source_event_id.is_empty()
            || fact.source_event_id.len() > 256
        {
            return Err(HubError::Invalid("settlement identity"));
        }
        let json = serde_json::to_value(fact).map_err(|_| HubError::Invalid("settlement fact"))?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| db_failure(err, line!()))?;
        let context=sqlx::query("SELECT r.id,r.client_id,r.grant_id,r.principal_id,r.namespace_binding_id,r.operation_id FROM attempts a JOIN requests r ON r.installation_id=a.installation_id AND r.id=a.request_id WHERE a.installation_id=$1 AND a.id=$2")
            .bind(self.installation_id).bind(fact.attempt_id).fetch_optional(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::NotFound)?;
        // Disabled/revoked clients still owe previously accepted provider expenses.
        sqlx::query("SELECT id FROM clients WHERE installation_id=$1 AND id=$2 FOR UPDATE")
            .bind(self.installation_id)
            .bind(context.get::<Uuid, _>("client_id"))
            .fetch_one(&mut *tx)
            .await
            .map_err(|err| db_failure(err, line!()))?;
        sqlx::query("SELECT id FROM grants WHERE installation_id=$1 AND id=$2 FOR UPDATE")
            .bind(self.installation_id)
            .bind(context.get::<Uuid, _>("grant_id"))
            .fetch_one(&mut *tx)
            .await
            .map_err(|err| db_failure(err, line!()))?;
        sqlx::query("SELECT id FROM requests WHERE installation_id=$1 AND id=$2 FOR UPDATE")
            .bind(self.installation_id)
            .bind(context.get::<Uuid, _>("id"))
            .fetch_one(&mut *tx)
            .await
            .map_err(|err| db_failure(err, line!()))?;
        let attempt =
            sqlx::query("SELECT *,dispatch_lease_until>now() AS lease_valid FROM attempts WHERE installation_id=$1 AND id=$2 FOR UPDATE")
                .bind(self.installation_id)
                .bind(fact.attempt_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|err|db_failure(err,line!()))?;
        if attempt.get::<Option<Uuid>, _>("dispatch_owner_id") != Some(fact.owner_id)
            || attempt.get::<Option<Uuid>, _>("dispatch_fence") != Some(fact.fence)
        {
            return Err(HubError::PreconditionFailed);
        }
        let snapshot: serde_json::Value = attempt.get("deployment_snapshot");
        if snapshot.get("adapter_revision").and_then(|v| v.as_str()) != Some(fact.source.as_str()) {
            return Err(HubError::Forbidden);
        }
        // Provider event IDs are scoped to the exact account, not a display name or adapter alone.
        let source_key = format!(
            "{}:{}",
            fact.source,
            attempt.get::<Uuid, _>("connection_id")
        );
        if source_key.len() > 256 {
            return Err(HubError::Invalid("settlement source"));
        }
        let previous_fact=sqlx::query("SELECT * FROM settlement_facts WHERE installation_id=$1 AND source=$2 AND source_event_id=$3")
            .bind(self.installation_id).bind(&source_key).bind(&fact.source_event_id).fetch_optional(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        if let Some(previous) = previous_fact {
            if previous.get::<serde_json::Value, _>("fact") != json {
                return Err(HubError::IdempotencyConflict);
            }
            return Ok(SettlementReceipt {
                ledger_id: previous.get("ledger_id"),
                amount: row_amount(&previous, "result_amount")?,
                confidence: previous.get("result_confidence"),
                duplicate: true,
            });
        }
        if fact.acceptance != Acceptance::Unknown
            && fact.receipt.is_none()
            && !attempt.get::<bool, _>("lease_valid")
        {
            return Err(HubError::PreconditionFailed);
        }
        let currency: String = attempt.get("currency");
        let (cost, confidence) = if fact.acceptance == Acceptance::Unknown {
            (None, "unknown")
        } else if let Some(receipt) = &fact.receipt {
            if snapshot
                .pointer("/qualified_capabilities/trusted_charge_receipts")
                .and_then(|v| v.as_bool())
                != Some(true)
                || receipt.currency.to_string() != currency
                || receipt.external_id != fact.source_event_id
                || receipt.digest.len() != 64
                || !receipt.digest.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(HubError::Forbidden);
            }
            (Some(receipt.amount), "confirmed")
        } else if let Some(usage) = &fact.usage {
            let price: Option<Price> = serde_json::from_value(
                snapshot
                    .get("price")
                    .cloned()
                    .ok_or(HubError::Unavailable)?,
            )
            .map_err(|_| HubError::Unavailable)?;
            let cost = price.map(|p| p.cost(usage)).transpose()?.flatten();
            (
                cost,
                if cost.is_some() {
                    "estimated"
                } else {
                    "unknown"
                },
            )
        } else {
            (None, "unknown")
        };
        let previous = sqlx::query(
            "SELECT * FROM attempt_expenses WHERE installation_id=$1 AND attempt_id=$2 FOR UPDATE",
        )
        .bind(self.installation_id)
        .bind(fact.attempt_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| db_failure(err, line!()))?;
        let prior_cost = previous
            .as_ref()
            .map(|row| row_amount(row, "amount"))
            .transpose()?
            .flatten();
        if let Some(previous) = &previous {
            // Later unknown/estimate facts cannot erase an already known or confirmed expense.
            if (prior_cost.is_some() && cost.is_none())
                || (previous.get::<String, _>("confidence") == "confirmed"
                    && confidence != "confirmed")
            {
                return Err(HubError::PreconditionFailed);
            }
        }
        let delta = Adjustment::from_units(
            cost.map(|a| a.units())
                .unwrap_or(0)
                .checked_sub(prior_cost.map(|a| a.units()).unwrap_or(0))
                .ok_or(HubError::Invalid("settlement overflow"))?,
        )?;
        let ledger_id = Uuid::new_v4();
        let price_id = snapshot
            .pointer("/target/price_revision_id")
            .and_then(|v| v.as_str())
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|_| HubError::Unavailable)?;
        let previous_id = previous.as_ref().map(|p| p.get::<Uuid, _>("ledger_id"));
        // Repeated unknown observations are facts but have no additional charge.
        let effective_ledger = if cost.is_none() && previous_id.is_some() {
            previous_id.unwrap()
        } else {
            let kind = if previous_id.is_some() {
                "correction"
            } else {
                "charge"
            };
            sqlx::query("INSERT INTO ledger_entries(id,installation_id,attempt_id,charge_key,kind,amount,currency,confidence,original_entry_id,price_revision_id,source,source_event_id,evidence) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)")
                .bind(ledger_id).bind(self.installation_id).bind(fact.attempt_id).bind(format!("expense:{}",fact.attempt_id)).bind(kind).bind(if cost.is_some(){Some(decimal(delta)?)}else{None}).bind(&currency).bind(confidence).bind(previous_id).bind(price_id).bind(&source_key).bind(&fact.source_event_id).bind(&json).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
            ledger_id
        };
        if let Some(usage) = &fact.usage {
            // Validate category/provenance even when only a receipt establishes the cost.
            if usage.source.is_empty()
                || usage.source.len() > 256
                || usage.categories.values().any(|n| *n > i64::MAX as u64)
            {
                return Err(HubError::Invalid("usage fact"));
            }
            sqlx::query("INSERT INTO usage_facts(id,installation_id,attempt_id,source,source_event_id,categories,provenance) VALUES($1,$2,$3,$4,$5,$6,$7)")
                .bind(Uuid::new_v4()).bind(self.installation_id).bind(fact.attempt_id).bind(&source_key).bind(&fact.source_event_id).bind(serde_json::to_value(&usage.categories).map_err(|_|HubError::Invalid("usage"))?).bind(serde_json::json!({"source":usage.source,"complete":usage.complete})).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        }
        let periods=sqlx::query("SELECT p.id,r.amount,r.state FROM reservations r JOIN budget_periods p ON p.installation_id=r.installation_id AND p.id=r.budget_period_id JOIN budget_policies b ON b.installation_id=p.installation_id AND b.id=p.policy_id WHERE r.installation_id=$1 AND r.attempt_id=$2 ORDER BY CASE b.scope_type WHEN 'installation' THEN 0 WHEN 'project' THEN 1 WHEN 'client' THEN 2 ELSE 3 END,b.scope_id,b.currency,b.id FOR UPDATE OF p,r")
            .bind(self.installation_id).bind(fact.attempt_id).fetch_all(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        let known = cost.is_some();
        for period in periods {
            let release = known && period.get::<String, _>("state") == "held";
            let reserved = if release {
                row_amount(&period, "amount")?.ok_or(HubError::Unavailable)?
            } else {
                Amount::from_units(0)?
            };
            sqlx::query("UPDATE budget_periods SET charged=charged+$3,reserved=reserved-$4,version=version+1 WHERE installation_id=$1 AND id=$2")
                .bind(self.installation_id).bind(period.get::<Uuid,_>("id")).bind(decimal(delta)?).bind(decimal(reserved)?).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
            if release {
                sqlx::query("UPDATE reservations SET state='settled' WHERE installation_id=$1 AND attempt_id=$2 AND budget_period_id=$3")
                    .bind(self.installation_id).bind(fact.attempt_id).bind(period.get::<Uuid,_>("id")).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
            }
        }
        let release = known && prior_cost.is_none();
        let upper = if release {
            row_amount(&attempt, "upper_provider_cost")?.unwrap_or(Amount::from_units(0)?)
        } else {
            Amount::from_units(0)?
        };
        sqlx::query("UPDATE grant_accounts SET charged=charged+$3,reserved=reserved-$4,version=version+1 WHERE installation_id=$1 AND grant_id=$2")
            .bind(self.installation_id).bind(context.get::<Uuid,_>("grant_id")).bind(decimal(delta)?).bind(decimal(upper)?).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        if release && row_amount(&attempt, "upper_provider_cost")?.is_some() {
            sqlx::query("INSERT INTO ledger_entries(id,installation_id,attempt_id,charge_key,kind,amount,currency,confidence,source,source_event_id,evidence) VALUES($1,$2,$3,$4,'release',$5,$6,'estimated','settlement-reserve',$3::text,$7)")
                .bind(Uuid::new_v4()).bind(self.installation_id).bind(fact.attempt_id).bind(format!("reserve:{}",fact.attempt_id)).bind(decimal(upper)?).bind(&currency).bind(serde_json::json!({"settlement_ledger_id":effective_ledger})).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        }
        sqlx::query("INSERT INTO attempt_expenses(installation_id,attempt_id,ledger_id,amount,currency,confidence) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(attempt_id) DO UPDATE SET ledger_id=EXCLUDED.ledger_id,amount=EXCLUDED.amount,confidence=EXCLUDED.confidence,version=attempt_expenses.version+1")
            .bind(self.installation_id).bind(fact.attempt_id).bind(effective_ledger).bind(cost.map(decimal).transpose()?).bind(&currency).bind(confidence).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        sqlx::query("INSERT INTO settlement_facts(id,installation_id,attempt_id,source,source_event_id,fact,ledger_id,result_amount,result_confidence) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)")
            .bind(Uuid::new_v4()).bind(self.installation_id).bind(fact.attempt_id).bind(&source_key).bind(&fact.source_event_id).bind(&json).bind(effective_ledger).bind(cost.map(decimal).transpose()?).bind(confidence).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        let state = if known {
            fact.terminal.as_str()
        } else {
            "unknown"
        };
        let accepted = match fact.acceptance {
            Acceptance::Unknown => "unknown",
            Acceptance::Accepted => "accepted",
            Acceptance::NotAccepted => "not_accepted",
        };
        sqlx::query("UPDATE attempts SET state=$3,accepted=$4,finished_at=CASE WHEN $3='unknown' THEN NULL ELSE now() END,version=version+1 WHERE installation_id=$1 AND id=$2")
            .bind(self.installation_id).bind(fact.attempt_id).bind(state).bind(accepted).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        sqlx::query("UPDATE requests SET state=$3,finished_at=CASE WHEN $3='unknown' THEN NULL ELSE now() END,version=version+1 WHERE installation_id=$1 AND id=$2")
            .bind(self.installation_id).bind(context.get::<Uuid,_>("id")).bind(state).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,namespace_binding_id,reason,operation_id) VALUES($1,$2,$3,'request.settle',$4,$5,'Atomic expense and terminal fact',$6)")
            .bind(Uuid::new_v4()).bind(self.installation_id).bind(context.get::<String,_>("principal_id")).bind(fact.attempt_id.to_string()).bind(context.get::<Option<Uuid>,_>("namespace_binding_id")).bind(context.get::<Uuid,_>("operation_id")).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        tx.commit().await.map_err(|err| db_failure(err, line!()))?;
        Ok(SettlementReceipt {
            ledger_id: effective_ledger,
            amount: cost,
            confidence: confidence.into(),
            duplicate: false,
        })
    }
}
