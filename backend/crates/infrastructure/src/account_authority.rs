use crate::{
    account_statement::{
        OPENROUTER_BILLING_TIER, OPENROUTER_STATEMENT_ORIGIN, OpenRouterStatement,
    },
    financial::db_failure,
    postgres::PgStore,
};
use aihub_domain::{
    catalog::{AccountAuthorityView, CatalogObservation},
    error::HubError,
    financial::{Amount, Currency},
};
use chrono::{DateTime, Utc};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;
impl PgStore {
    pub(crate) async fn store_account_statement_locked(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        operation: Uuid,
        connection: Uuid,
        generation: i64,
        endpoint_hash: &str,
        catalog: &CatalogObservation,
        statement: &OpenRouterStatement,
    ) -> Result<Uuid, HubError> {
        if statement.observed_at != catalog.observed_at
            || statement.observed_at > Utc::now() + chrono::Duration::seconds(1)
            || statement.observed_at < Utc::now() - chrono::Duration::seconds(60)
        {
            return Err(HubError::PreconditionFailed);
        }
        let row=sqlx::query("SELECT c.status,p.kind,g.adapter_revision,g.endpoint_policy_hash,g.endpoint_snapshot,g.billing_tier FROM connections c JOIN providers p ON p.installation_id=c.installation_id AND p.id=c.provider_id JOIN connection_generations g ON g.connection_id=c.id AND g.generation=c.generation JOIN credential_versions v ON v.connection_id=c.id AND v.generation=c.generation WHERE c.installation_id=$1 AND c.id=$2 AND c.generation=$3 AND c.status IN ('authorization_unknown','enabled') AND g.authorization_state IN ('prepared','active') AND v.state IN ('prepared','active') FOR UPDATE OF c,g FOR SHARE OF v")
            .bind(self.installation_id).bind(connection).bind(generation).fetch_optional(&mut **tx).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::PreconditionFailed)?;
        let policy: serde_json::Value = row
            .get::<Option<serde_json::Value>, _>("endpoint_snapshot")
            .ok_or(HubError::PreconditionFailed)?;
        if row.get::<String, _>("endpoint_policy_hash") != endpoint_hash
            || !aihub_domain::connections::supports_openrouter_metadata(
                aihub_domain::connections::ProviderKind::parse(&row.get::<String, _>("kind"))?,
                &policy,
            )
        {
            return Err(HubError::PreconditionFailed);
        }
        let expires = statement
            .account
            .expires_at
            .unwrap_or(statement.observed_at + chrono::Duration::hours(1))
            .min(statement.observed_at + chrono::Duration::hours(1));
        let as_of: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| db_failure(e, line!()))?;
        if expires <= as_of {
            return Err(HubError::PreconditionFailed);
        }
        self.store_catalog_locked(tx, connection, generation, endpoint_hash, catalog)
            .await?;
        sqlx::query("UPDATE verification_account_authorities SET state='invalidated' WHERE installation_id=$1 AND connection_id=$2 AND generation=$3 AND state='active'").bind(self.installation_id).bind(connection).bind(generation).execute(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO verification_account_authorities(id,installation_id,connection_id,generation,operation_id,adapter_revision,endpoint_policy_hash,currency,billing_tier,statement_origin,statement_digest,statement_usage,billing_capabilities,observed_at,expires_at,state) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12::text::numeric,'{\"pricing_reader\":true,\"trusted_charge_receipts\":true}',$13,$14,'active')")
            .bind(id).bind(self.installation_id).bind(connection).bind(generation).bind(operation).bind(row.get::<String,_>("adapter_revision")).bind(endpoint_hash).bind(statement.currency.to_string()).bind(OPENROUTER_BILLING_TIER).bind(OPENROUTER_STATEMENT_ORIGIN).bind(&statement.account.digest).bind(statement.usage.to_string()).bind(statement.observed_at).bind(expires).execute(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
        if row.get::<Option<String>, _>("billing_tier").as_deref() != Some(OPENROUTER_BILLING_TIER)
        {
            sqlx::query("UPDATE runtime_qualifications SET state='invalidated',invalidated_reason='account billing tier changed' WHERE installation_id=$1 AND connection_id=$2 AND state='active'").bind(self.installation_id).bind(connection).execute(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
        }
        sqlx::query("UPDATE connection_generations SET billing_tier=$3 WHERE connection_id=$1 AND generation=$2").bind(connection).bind(generation).bind(OPENROUTER_BILLING_TIER).execute(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
        // Quotes are financial metadata. Model availability/capabilities stay unverified.
        for model in &catalog.models {
            let (Some(input), Some(output), Some(fee)) = (
                model.input_uncached.as_ref(),
                model.output_billable.as_ref(),
                model.request_fee.as_ref(),
            ) else {
                continue;
            };
            let model_row:Uuid=sqlx::query_scalar("SELECT id FROM upstream_models WHERE connection_id=$1 AND generation=$2 AND provider_model_id=$3 AND observed_at=$4").bind(connection).bind(generation).bind(&model.metadata.provider_model_id).bind(catalog.observed_at).fetch_one(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
            let price = Uuid::new_v4();
            sqlx::query("INSERT INTO price_revisions(id,installation_id,connection_id,model_id,tier,currency,input_uncached,input_cached,cache_write,output_billable,request_fee,effective_from,source) VALUES($1,$2,$3,$4,$5,$6,$7::text::numeric,$8::text::numeric,$9::text::numeric,$10::text::numeric,$11::text::numeric,$12,'openrouter_catalog_statement_v1')")
                .bind(price).bind(self.installation_id).bind(connection).bind(&model.metadata.provider_model_id).bind(OPENROUTER_BILLING_TIER).bind(statement.currency.to_string()).bind(input.to_string()).bind(model.input_cached.as_ref().map(ToString::to_string)).bind(model.cache_write.as_ref().map(ToString::to_string)).bind(output.to_string()).bind(fee.to_string()).bind(catalog.observed_at).execute(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
            sqlx::query("INSERT INTO catalog_price_origins(price_revision_id,installation_id,catalog_model_id,observed_at) VALUES($1,$2,$3,$4)").bind(price).bind(self.installation_id).bind(model_row).bind(catalog.observed_at).execute(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
        }
        Ok(id)
    }
    pub(crate) async fn read_account_authority(
        &self,
        connection: Uuid,
    ) -> Result<AccountAuthorityView, HubError> {
        let conn=sqlx::query("SELECT c.generation,c.status,g.adapter_revision,g.endpoint_policy_hash,g.billing_tier,g.authorization_state FROM connections c JOIN connection_generations g ON g.connection_id=c.id AND g.generation=c.generation WHERE c.installation_id=$1 AND c.id=$2").bind(self.installation_id).bind(connection).fetch_optional(&self.pool).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::NotFound)?;
        let generation: i64 = conn.get("generation");
        let row=sqlx::query("SELECT *,expires_at>clock_timestamp() AS live FROM verification_account_authorities WHERE installation_id=$1 AND connection_id=$2 AND generation=$3 ORDER BY observed_at DESC,id DESC LIMIT 1").bind(self.installation_id).bind(connection).bind(generation).fetch_optional(&self.pool).await.map_err(|e|db_failure(e,line!()))?;
        let Some(row) = row else {
            return Ok(AccountAuthorityView {
                connection_id: connection,
                generation,
                status: "unqualified".into(),
                currency: None,
                statement_usage: None,
                observed_at: None,
                expires_at: None,
            });
        };
        let status = if !row.get::<bool, _>("live") {
            "expired"
        } else if row.get::<String, _>("state") != "active"
            || !matches!(
                conn.get::<String, _>("status").as_str(),
                "enabled" | "authorization_unknown"
            )
            || !matches!(
                conn.get::<String, _>("authorization_state").as_str(),
                "prepared" | "active"
            )
            || row.get::<String, _>("adapter_revision") != conn.get::<String, _>("adapter_revision")
            || row.get::<String, _>("endpoint_policy_hash")
                != conn.get::<String, _>("endpoint_policy_hash")
            || Some(row.get::<String, _>("billing_tier"))
                != conn.get::<Option<String>, _>("billing_tier")
        {
            "invalidated"
        } else {
            "active"
        };
        let amount: bigdecimal::BigDecimal = row.get("statement_usage");
        Ok(AccountAuthorityView {
            connection_id: connection,
            generation,
            status: status.into(),
            currency: Some(Currency::parse(&row.get::<String, _>("currency"))?),
            statement_usage: Some(Amount::parse(&amount.normalized().to_plain_string())?),
            observed_at: Some(row.get("observed_at")),
            expires_at: Some(row.get("expires_at")),
        })
    }
}
