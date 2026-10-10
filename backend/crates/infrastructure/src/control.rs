use crate::{financial::db_failure, postgres::PgStore};
use aihub_domain::{
    error::HubError,
    financial::{Amount, Currency, Rate},
    prices::{PriceInput, PriceMutation, PriceRevision, PriceUnit},
    records::Page,
};
use bigdecimal::BigDecimal;
use sqlx::Row;
use std::str::FromStr;
use uuid::Uuid;

fn decimal(value: impl ToString) -> Result<BigDecimal, HubError> {
    BigDecimal::from_str(&value.to_string()).map_err(|_| HubError::Invalid("price decimal"))
}
fn quote(row: &sqlx::postgres::PgRow) -> Result<PriceRevision, HubError> {
    let input: BigDecimal = row.get("input_uncached");
    let output: BigDecimal = row.get("output_billable");
    let cached: Option<BigDecimal> = row.get("input_cached");
    let fee: Option<BigDecimal> = row.get("request_fee");
    Ok(PriceRevision {
        id: row.get("id"),
        created_at: row.get("created_at"),
        price: PriceInput {
            connection_id: row.get("connection_id"),
            model_id: row.get("model_id"),
            tier: row.get("tier"),
            currency: Currency::parse(&row.get::<String, _>("currency"))?,
            unit: PriceUnit::PerMillionTokens,
            input_uncached: Rate::parse(&input.normalized().to_plain_string())?,
            input_cached: cached
                .map(|v| Rate::parse(&v.normalized().to_plain_string()))
                .transpose()?,
            output_billable: Rate::parse(&output.normalized().to_plain_string())?,
            request_fee: fee
                .map(|v| Amount::parse(&v.normalized().to_plain_string()))
                .transpose()?,
            effective_from: row.get("effective_from"),
            effective_to: row.get("effective_to"),
            source: row.get("source"),
        },
    })
}

impl PgStore {
    pub(crate) async fn begin_control_operation(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        subject: &str,
        key: Uuid,
        binding: [u8; 32],
        action: &str,
    ) -> Result<(Uuid, Option<serde_json::Value>), HubError> {
        if key.is_nil() || subject.is_empty() {
            return Err(HubError::Invalid("control operation"));
        }
        let closed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM control_key_fences WHERE installation_id=$1 AND principal_kind='human' AND principal_id=$2 AND idempotency_key=$3)").bind(self.installation_id).bind(subject).bind(key).fetch_one(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
        if closed {
            return Err(HubError::IdempotencyConflict);
        }
        let operation = Uuid::new_v4();
        let row=sqlx::query("INSERT INTO operations(id,installation_id,principal_kind,principal_id,idempotency_key,binding_hmac,action,state,expires_at) VALUES($1,$2,'human',$3,$4,$5,$6,'pending',now()+interval '30 days') ON CONFLICT(installation_id,principal_kind,principal_id,idempotency_key) DO UPDATE SET id=operations.id RETURNING *,expires_at>now() AS replay_valid")
            .bind(operation).bind(self.installation_id).bind(subject).bind(key).bind(binding.as_slice()).bind(action).fetch_one(&mut **tx).await.map_err(|err|db_failure(err,line!()))?;
        if row.get::<Vec<u8>, _>("binding_hmac").as_slice() != binding
            || row.get::<String, _>("action") != action
            || !row.get::<bool, _>("replay_valid")
        {
            return Err(HubError::IdempotencyConflict);
        }
        let stored: Uuid = row.get("id");
        if stored != operation {
            let safe: Option<serde_json::Value> = row.get("safe_result");
            return Ok((stored, Some(safe.ok_or(HubError::Unavailable)?)));
        }
        Ok((operation, None))
    }
    pub(crate) async fn price_page_internal(
        &self,
        subject: &str,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<Page<PriceRevision>, HubError> {
        let items = if cursor.is_none() {
            let rows=sqlx::query("SELECT * FROM price_revisions WHERE installation_id=$1 ORDER BY created_at DESC,id LIMIT 10001")
                .bind(self.installation_id).fetch_all(&self.pool).await.map_err(|err|db_failure(err,line!()))?;
            rows.iter().map(quote).collect::<Result<Vec<_>, _>>()?
        } else {
            vec![]
        };
        self.snapshot_page(subject, &format!("prices:{limit}"), limit, cursor, items)
            .await
    }
    pub(crate) async fn create_price_internal(
        &self,
        subject: &str,
        key: Uuid,
        binding: [u8; 32],
        price: &PriceInput,
    ) -> Result<PriceMutation, HubError> {
        price.validate()?;
        if key.is_nil() || subject.is_empty() {
            return Err(HubError::Invalid("price operation"));
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| db_failure(err, line!()))?;
        let (operation, replay) = self
            .begin_control_operation(&mut tx, subject, key, binding, "price.create")
            .await?;
        if let Some(replay) = replay {
            return serde_json::from_value(replay).map_err(|_| HubError::Unavailable);
        }
        sqlx::query("SELECT id FROM connections WHERE installation_id=$1 AND id=$2 AND status<>'archived' FOR SHARE")
            .bind(self.installation_id).bind(price.connection_id).fetch_optional(&mut *tx).await.map_err(|err|db_failure(err,line!()))?.ok_or(HubError::NotFound)?;
        let id = Uuid::new_v4();
        let row=sqlx::query("INSERT INTO price_revisions(id,installation_id,connection_id,model_id,tier,currency,input_uncached,input_cached,output_billable,request_fee,effective_from,effective_to,source) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) RETURNING *")
            .bind(id).bind(self.installation_id).bind(price.connection_id).bind(&price.model_id).bind(&price.tier).bind(price.currency.to_string()).bind(decimal(price.input_uncached)?).bind(price.input_cached.map(decimal).transpose()?).bind(decimal(price.output_billable)?).bind(price.request_fee.map(decimal).transpose()?).bind(price.effective_from).bind(price.effective_to).bind(&price.source).fetch_one(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        let result = PriceMutation {
            operation_id: operation,
            value: quote(&row)?,
        };
        sqlx::query("UPDATE operations SET state='succeeded',resource_id=$3,safe_result=$4,updated_at=now(),version=version+1 WHERE installation_id=$1 AND id=$2")
            .bind(self.installation_id).bind(operation).bind(id).bind(serde_json::to_value(&result).map_err(|_|HubError::Unavailable)?).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,reason) VALUES($1,$2,$3,'price.create',$4,$5,'Immutable quote; no provider I/O')")
            .bind(Uuid::new_v4()).bind(self.installation_id).bind(subject).bind(id.to_string()).bind(operation).execute(&mut *tx).await.map_err(|err|db_failure(err,line!()))?;
        tx.commit().await.map_err(|err| db_failure(err, line!()))?;
        Ok(result)
    }
}
