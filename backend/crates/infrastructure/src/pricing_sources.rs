//! Immutable source configuration. Quote labels never qualify catalog provenance.
use crate::{financial::db_failure, postgres::PgStore};
use aihub_domain::{
    error::HubError,
    financial::Currency,
    pricing_sources::{
        PricingDataStatus, PricingMode, PricingResolution, PricingSourceInput,
        PricingSourceMutation, PricingSourceRevision,
    },
    records::Page,
};
use chrono::{DateTime, Utc};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

const SOURCE_VIEW: &str = "SELECT r.*,p.connection_id,p.model_id,p.currency,a.observed_at AS visible_catalog_observed_at,CASE WHEN r.effective_to<=clock_timestamp() THEN 'stale' WHEN c.status<>'enabled' OR g.billing_tier IS NULL THEN 'unavailable' WHEN r.mode='provider_auto' THEN CASE WHEN a.id IS NULL THEN 'unavailable' ELSE 'complete' END WHEN EXISTS(SELECT 1 FROM price_revisions q WHERE q.installation_id=r.installation_id AND q.id=r.manual_price_revision_id AND q.connection_id=p.connection_id AND q.model_id=p.model_id AND q.currency=p.currency AND q.tier=g.billing_tier AND q.effective_from<=r.effective_from AND (q.effective_to IS NULL OR (r.effective_to IS NOT NULL AND q.effective_to>=r.effective_to))) THEN 'complete' ELSE 'unavailable' END AS data_status FROM pricing_source_revisions r JOIN pricing_source_policies p ON p.installation_id=r.installation_id AND p.id=r.policy_id JOIN connections c ON c.installation_id=p.installation_id AND c.id=p.connection_id LEFT JOIN connection_generations g ON g.connection_id=c.id AND g.generation=c.generation LEFT JOIN LATERAL(SELECT v.id,v.observed_at FROM eligible_catalog_prices v WHERE v.installation_id=p.installation_id AND v.connection_id=p.connection_id AND v.model_id=p.model_id AND v.currency=p.currency AND v.tier=g.billing_tier AND v.effective_from<=clock_timestamp() AND (v.effective_to IS NULL OR v.effective_to>clock_timestamp()) AND v.observed_at<=clock_timestamp() AND v.observed_at>clock_timestamp()-interval '24 hours' ORDER BY v.observed_at DESC,v.id LIMIT 1)a ON r.mode='provider_auto'";
fn revision(row: &sqlx::postgres::PgRow) -> Result<PricingSourceRevision, HubError> {
    Ok(PricingSourceRevision {
        id: row.get("id"),
        version: row.get("version"),
        actor_subject: row.get("actor_subject"),
        created_at: row.get("created_at"),
        catalog_observed_at: row.get("visible_catalog_observed_at"),
        data_status: match row.get::<String, _>("data_status").as_str() {
            "complete" => PricingDataStatus::Complete,
            "stale" => PricingDataStatus::Stale,
            _ => PricingDataStatus::Unavailable,
        },
        config: PricingSourceInput {
            connection_id: row.get("connection_id"),
            model_id: row.get("model_id"),
            currency: Currency::parse(&row.get::<String, _>("currency"))?,
            mode: match row.get::<String, _>("mode").as_str() {
                "manual" => PricingMode::Manual,
                "provider_auto" => PricingMode::ProviderAuto,
                _ => return Err(HubError::Unavailable),
            },
            manual_price_revision_id: row.get("manual_price_revision_id"),
            expected_version: row.get("expected_version"),
            effective_from: row.get("effective_from"),
            effective_to: row.get("effective_to"),
        },
    })
}
impl PgStore {
    async fn qualified_catalog_price(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        connection: Uuid,
        model: &str,
        currency: &Currency,
        tier: &str,
        as_of: DateTime<Utc>,
    ) -> Result<Option<(Uuid, DateTime<Utc>)>, HubError> {
        sqlx::query_as("SELECT id,observed_at FROM eligible_catalog_prices WHERE installation_id=$1 AND connection_id=$2 AND model_id=$3 AND currency=$4 AND tier=$5 AND effective_from<=$6 AND (effective_to IS NULL OR effective_to>$6) AND observed_at<=$6 AND observed_at>$6-interval '24 hours' ORDER BY observed_at DESC,id LIMIT 1")
            .bind(self.installation_id).bind(connection).bind(model).bind(currency.to_string()).bind(tier).bind(as_of).fetch_optional(&mut **tx).await.map_err(|e|db_failure(e,line!()))
    }
    pub(crate) async fn source_page_internal(
        &self,
        subject: &str,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<Page<PricingSourceRevision>, HubError> {
        let items = if cursor.is_none() {
            let rows=sqlx::query(&format!("{SOURCE_VIEW} WHERE r.installation_id=$1 ORDER BY r.created_at DESC,r.id LIMIT 10001")).bind(self.installation_id).fetch_all(&self.pool).await.map_err(|e|db_failure(e,line!()))?;
            rows.iter().map(revision).collect::<Result<Vec<_>, _>>()?
        } else {
            vec![]
        };
        self.snapshot_page(
            subject,
            &format!("pricing-sources:{limit}"),
            limit,
            cursor,
            items,
        )
        .await
    }
    pub(crate) async fn create_source_internal(
        &self,
        subject: &str,
        key: Uuid,
        binding: [u8; 32],
        input: &PricingSourceInput,
    ) -> Result<PricingSourceMutation, HubError> {
        input.validate()?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let (operation, replay) = self
            .begin_control_operation(&mut tx, subject, key, binding, "pricing-source.create")
            .await?;
        if let Some(replay) = replay {
            return serde_json::from_value(replay).map_err(|_| HubError::Unavailable);
        }
        let conn=sqlx::query("SELECT g.billing_tier FROM connections c JOIN connection_generations g ON g.connection_id=c.id AND g.generation=c.generation WHERE c.installation_id=$1 AND c.id=$2 AND (c.status='enabled' OR (c.status='authorization_unknown' AND EXISTS(SELECT 1 FROM verification_account_authorities a WHERE a.installation_id=c.installation_id AND a.connection_id=c.id AND a.generation=c.generation AND a.state='active' AND a.adapter_revision=g.adapter_revision AND a.endpoint_policy_hash=g.endpoint_policy_hash AND a.billing_tier=g.billing_tier AND a.observed_at<=clock_timestamp() AND a.expires_at>clock_timestamp()))) FOR SHARE OF c,g")
            .bind(self.installation_id).bind(input.connection_id).fetch_optional(&mut *tx).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::PreconditionFailed)?;
        let tier: String = conn
            .get::<Option<String>, _>("billing_tier")
            .ok_or(HubError::PreconditionFailed)?;
        // Covers the absent-policy case without creating an unconfigured placeholder row.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(jsonb_build_array($1::text,$2::text,$3::text)::text,310817))")
            .bind(input.connection_id.to_string()).bind(&input.model_id).bind(input.currency.to_string()).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        let as_of: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let catalog = match input.mode {
            PricingMode::Manual => {
                let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM price_revisions WHERE installation_id=$1 AND id=$2 AND connection_id=$3 AND model_id=$4 AND currency=$5 AND tier=$6 AND effective_from<=$7 AND (effective_to IS NULL OR ($8::timestamptz IS NOT NULL AND effective_to>=$8)))")
                    .bind(self.installation_id).bind(input.manual_price_revision_id).bind(input.connection_id).bind(&input.model_id).bind(input.currency.to_string()).bind(&tier).bind(input.effective_from).bind(input.effective_to).fetch_one(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
                if !valid {
                    return Err(HubError::InvalidSemantics(
                        "manual quote tuple/tier/window mismatch",
                    ));
                }
                None
            }
            PricingMode::ProviderAuto => Some(
                self.qualified_catalog_price(
                    &mut tx,
                    input.connection_id,
                    &input.model_id,
                    &input.currency,
                    &tier,
                    as_of,
                )
                .await?
                .ok_or(HubError::PreconditionFailed)?
                .1,
            ),
        };
        sqlx::query("INSERT INTO pricing_source_policies(id,installation_id,connection_id,model_id,currency) VALUES($1,$2,$3,$4,$5) ON CONFLICT(installation_id,connection_id,model_id,currency) DO NOTHING")
            .bind(Uuid::new_v4()).bind(self.installation_id).bind(input.connection_id).bind(&input.model_id).bind(input.currency.to_string()).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        let policy=sqlx::query("SELECT id,version FROM pricing_source_policies WHERE installation_id=$1 AND connection_id=$2 AND model_id=$3 AND currency=$4 FOR UPDATE")
            .bind(self.installation_id).bind(input.connection_id).bind(&input.model_id).bind(input.currency.to_string()).fetch_one(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        let version: i64 = policy.get("version");
        if version != input.expected_version {
            return Err(HubError::PreconditionFailed);
        }
        let version = version.checked_add(1).ok_or(HubError::InvalidSemantics(
            "pricing source version exhausted",
        ))?;
        let id = Uuid::new_v4();
        let inserted=sqlx::query("INSERT INTO pricing_source_revisions(id,installation_id,policy_id,mode,manual_price_revision_id,expected_version,version,effective_from,effective_to,actor_subject,catalog_observed_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) ON CONFLICT(policy_id,effective_from) DO NOTHING")
            .bind(id).bind(self.installation_id).bind(policy.get::<Uuid,_>("id")).bind(input.mode.as_str()).bind(input.manual_price_revision_id).bind(input.expected_version).bind(version).bind(input.effective_from).bind(input.effective_to).bind(subject).bind(catalog).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?.rows_affected();
        if inserted == 0 {
            return Err(HubError::InvalidSemantics(
                "source interval start already occupied",
            ));
        }
        sqlx::query(
            "UPDATE pricing_source_policies SET version=$3 WHERE installation_id=$1 AND id=$2",
        )
        .bind(self.installation_id)
        .bind(policy.get::<Uuid, _>("id"))
        .bind(version)
        .execute(&mut *tx)
        .await
        .map_err(|e| db_failure(e, line!()))?;
        let row = sqlx::query(&format!(
            "{SOURCE_VIEW} WHERE r.installation_id=$1 AND r.id=$2"
        ))
        .bind(self.installation_id)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| db_failure(e, line!()))?;
        let mut value = revision(&row)?;
        value.catalog_observed_at = catalog;
        let result = PricingSourceMutation {
            operation_id: operation,
            value,
        };
        sqlx::query("UPDATE operations SET state='succeeded',resource_id=$3,safe_result=$4,version=version+1,updated_at=now() WHERE installation_id=$1 AND id=$2")
            .bind(self.installation_id).bind(operation).bind(id).bind(serde_json::to_value(&result).map_err(|_|HubError::Unavailable)?).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,reason) VALUES($1,$2,$3,'pricing-source.create',$4,$5,'Immutable source revision; old request snapshots unchanged')")
            .bind(Uuid::new_v4()).bind(self.installation_id).bind(subject).bind(id.to_string()).bind(operation).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        tx.commit().await.map_err(|e| db_failure(e, line!()))?;
        Ok(result)
    }
    pub async fn resolve_pricing(
        &self,
        connection: Uuid,
        model: &str,
        currency: &Currency,
    ) -> Result<PricingResolution, HubError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let as_of: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let value = self
            .resolve_pricing_locked(&mut tx, connection, model, currency, as_of)
            .await?;
        tx.commit().await.map_err(|e| db_failure(e, line!()))?;
        Ok(value)
    }
    pub(crate) async fn resolve_pricing_locked(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        connection: Uuid,
        model: &str,
        currency: &Currency,
        as_of: DateTime<Utc>,
    ) -> Result<PricingResolution, HubError> {
        let tier:Option<String>=sqlx::query_scalar("SELECT g.billing_tier FROM connections c JOIN connection_generations g ON g.connection_id=c.id AND g.generation=c.generation WHERE c.installation_id=$1 AND c.id=$2 AND (c.status='enabled' OR (c.status='authorization_unknown' AND EXISTS(SELECT 1 FROM verification_account_authorities a WHERE a.installation_id=c.installation_id AND a.connection_id=c.id AND a.generation=c.generation AND a.state='active' AND a.adapter_revision=g.adapter_revision AND a.endpoint_policy_hash=g.endpoint_policy_hash AND a.billing_tier=g.billing_tier AND a.observed_at<=clock_timestamp() AND a.expires_at>clock_timestamp()))) FOR SHARE OF c,g")
            .bind(self.installation_id).bind(connection).fetch_optional(&mut **tx).await.map_err(|e|db_failure(e,line!()))?.flatten();
        sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended(jsonb_build_array($1::text,$2::text,$3::text)::text,310817))")
            .bind(connection.to_string()).bind(model).bind(currency.to_string()).execute(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
        let policy=sqlx::query("SELECT id,version FROM pricing_source_policies WHERE installation_id=$1 AND connection_id=$2 AND model_id=$3 AND currency=$4 FOR SHARE")
            .bind(self.installation_id).bind(connection).bind(model).bind(currency.to_string()).fetch_optional(&mut **tx).await.map_err(|e|db_failure(e,line!()))?;
        let mut value = PricingResolution {
            source_revision_id: None,
            policy_version: policy.as_ref().map(|p| p.get("version")).unwrap_or(0),
            price_revision_id: None,
            tier: tier.clone(),
            as_of,
            data_status: PricingDataStatus::Unavailable,
        };
        let Some(policy) = policy else {
            return Ok(value);
        };
        let Some(row)=sqlx::query("SELECT * FROM pricing_source_revisions WHERE installation_id=$1 AND policy_id=$2 AND effective_from<=$3 ORDER BY effective_from DESC LIMIT 1")
            .bind(self.installation_id).bind(policy.get::<Uuid,_>("id")).bind(as_of).fetch_optional(&mut **tx).await.map_err(|e|db_failure(e,line!()))? else {return Ok(value)};
        value.source_revision_id = Some(row.get("id"));
        if row
            .get::<Option<DateTime<Utc>>, _>("effective_to")
            .is_some_and(|end| end <= as_of)
        {
            value.data_status = PricingDataStatus::Stale;
            return Ok(value);
        }
        let Some(tier) = tier else { return Ok(value) };
        value.price_revision_id = if row.get::<String, _>("mode") == "manual" {
            sqlx::query_scalar("SELECT id FROM price_revisions WHERE installation_id=$1 AND id=$2 AND connection_id=$3 AND model_id=$4 AND currency=$5 AND tier=$6 AND effective_from<=$7 AND (effective_to IS NULL OR effective_to>$7)")
                .bind(self.installation_id).bind(row.get::<Option<Uuid>,_>("manual_price_revision_id")).bind(connection).bind(model).bind(currency.to_string()).bind(&tier).bind(as_of).fetch_optional(&mut **tx).await.map_err(|e|db_failure(e,line!()))?
        } else {
            self.qualified_catalog_price(tx, connection, model, currency, &tier, as_of)
                .await?
                .map(|v| v.0)
        };
        if value.price_revision_id.is_some() {
            value.data_status = PricingDataStatus::Complete
        }
        Ok(value)
    }
}
