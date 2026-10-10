use aihub_application::{FoundationStore, OperationBinding};
use aihub_domain::{
    error::HubError,
    financial::{Currency, Rate},
    prices::{PriceInput, PriceUnit},
    pricing_sources::{PricingDataStatus, PricingMode, PricingSourceInput},
};
use aihub_infrastructure::{postgres::PgStore, vault::Vault};
use chrono::{Duration, Utc};
use uuid::Uuid;

async fn connection(store: &PgStore, provider: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO connections(id,installation_id,provider_id,display_name,endpoint_policy_ref,billing_mode,status) VALUES($1,$2,$3,'same name','controlled','metered','enabled')").bind(id).bind(store.installation_id).bind(provider).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO connection_generations(connection_id,generation,authorization_state,adapter_revision,endpoint_policy_hash,billing_tier) VALUES($1,1,'active','synthetic-pricing-v1',$2,'metered')").bind(id).bind("a".repeat(64)).execute(&store.pool).await.unwrap();
    id
}
async fn quote(store: &PgStore, vault: &Vault, connection: Uuid, currency: &str) -> Uuid {
    let input = PriceInput {
        connection_id: connection,
        model_id: "same-model".into(),
        tier: "metered".into(),
        currency: Currency::parse(currency).unwrap(),
        unit: PriceUnit::PerMillionTokens,
        input_uncached: Rate::parse("2").unwrap(),
        input_cached: None,
        output_billable: Rate::parse("8").unwrap(),
        request_fee: None,
        effective_from: Utc::now() - Duration::hours(1),
        effective_to: None,
        source: "OpenRouter label is not catalog authority".into(),
    };
    store
        .create_price(
            "actor",
            Uuid::new_v4(),
            vault.bind(&serde_json::to_value(&input).unwrap()).unwrap(),
            &input,
        )
        .await
        .unwrap()
        .value
        .id
}
async fn write(
    store: &PgStore,
    vault: &Vault,
    input: &PricingSourceInput,
) -> Result<aihub_domain::pricing_sources::PricingSourceMutation, HubError> {
    store
        .create_pricing_source(
            "actor",
            Uuid::new_v4(),
            vault.bind(&serde_json::to_value(input).unwrap()).unwrap(),
            input,
        )
        .await
}
#[tokio::test]
#[ignore = "requires explicit isolated PostgreSQL17 fixture"]
async fn immutable_source_cas_exact_tuple_expiry_catalog_and_rollback() {
    let dsn = std::env::var("AIHUB_TEST_DATABASE_URL").unwrap();
    let vault = Vault::new(vec![7; 32]).unwrap();
    let installation = Uuid::new_v4();
    let store = PgStore::connect(&dsn, installation, vault.fingerprint())
        .await
        .unwrap();
    store.migrate().await.unwrap();
    store.initialize("pricing-fixture").await.unwrap();
    store.ready().await.unwrap();
    let provider: Uuid = sqlx::query_scalar(
        "SELECT id FROM providers WHERE installation_id=$1 AND kind='openai_compatible'",
    )
    .bind(installation)
    .fetch_one(&store.pool)
    .await
    .unwrap();
    let conn = connection(&store, provider).await;
    let neighbor = connection(&store, provider).await;
    let usd = Currency::parse("USD").unwrap();
    let eur = Currency::parse("EUR").unwrap();
    let initial = store
        .resolve_pricing(conn, "same-model", &usd)
        .await
        .unwrap();
    assert!(initial.source_revision_id.is_none());
    assert!(initial.price_revision_id.is_none());
    assert_eq!(initial.policy_version, 0);
    let price = quote(&store, &vault, conn, "USD").await;
    let input = PricingSourceInput {
        connection_id: conn,
        model_id: "same-model".into(),
        currency: usd.clone(),
        mode: PricingMode::Manual,
        manual_price_revision_id: Some(price),
        expected_version: 0,
        effective_from: Utc::now() - Duration::minutes(10),
        effective_to: None,
    };
    let key = Uuid::new_v4();
    let binding = vault.bind(&serde_json::to_value(&input).unwrap()).unwrap();
    let second = PgStore::connect(&dsn, installation, vault.fingerprint())
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        store.create_pricing_source("actor", key, binding, &input),
        second.create_pricing_source("actor", key, binding, &input)
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_eq!(a.operation_id, b.operation_id);
    assert_eq!(a.value.id, b.value.id);
    assert_eq!(a.value.version, 1);
    let resolved = store
        .resolve_pricing(conn, "same-model", &usd)
        .await
        .unwrap();
    assert_eq!(resolved.source_revision_id, Some(a.value.id));
    assert_eq!(resolved.price_revision_id, Some(price));
    assert!(matches!(
        write(&store, &vault, &input).await,
        Err(HubError::PreconditionFailed)
    ));
    let wrong = PricingSourceInput {
        connection_id: neighbor,
        ..input.clone()
    };
    assert!(matches!(
        write(&store, &vault, &wrong).await,
        Err(HubError::InvalidSemantics(_))
    ));
    let wrong_currency = PricingSourceInput {
        currency: eur.clone(),
        ..input.clone()
    };
    assert!(matches!(
        write(&store, &vault, &wrong_currency).await,
        Err(HubError::InvalidSemantics(_))
    ));
    let neighbor_price = quote(&store, &vault, neighbor, "USD").await;
    let neighbor_input = PricingSourceInput {
        connection_id: neighbor,
        manual_price_revision_id: Some(neighbor_price),
        ..input.clone()
    };
    assert_eq!(
        write(&store, &vault, &neighbor_input)
            .await
            .unwrap()
            .value
            .version,
        1
    );
    let eur_price = quote(&store, &vault, conn, "EUR").await;
    let eur_input = PricingSourceInput {
        currency: eur.clone(),
        manual_price_revision_id: Some(eur_price),
        ..input.clone()
    };
    assert_eq!(
        write(&store, &vault, &eur_input)
            .await
            .unwrap()
            .value
            .version,
        1
    );
    assert_eq!(
        store
            .resolve_pricing(conn, "same-model", &eur)
            .await
            .unwrap()
            .price_revision_id,
        Some(eur_price)
    );
    let expired = PricingSourceInput {
        expected_version: 1,
        effective_from: Utc::now() - Duration::minutes(1),
        effective_to: Some(Utc::now() - Duration::seconds(1)),
        ..input.clone()
    };
    let (one, two) = tokio::join!(
        write(&store, &vault, &expired),
        write(&second, &vault, &expired)
    );
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    let expired_revision = one.or(two).unwrap();
    let expired_resolution = store
        .resolve_pricing(conn, "same-model", &usd)
        .await
        .unwrap();
    assert_eq!(
        expired_resolution.source_revision_id,
        Some(expired_revision.value.id)
    );
    assert!(expired_resolution.price_revision_id.is_none());
    assert!(matches!(
        expired_resolution.data_status,
        PricingDataStatus::Stale
    ));
    assert_eq!(
        resolved.source_revision_id,
        Some(a.value.id),
        "already captured resolution is not rewritten"
    );
    let overlap = PricingSourceInput {
        expected_version: 2,
        ..input.clone()
    };
    assert!(matches!(
        write(&store, &vault, &overlap).await,
        Err(HubError::InvalidSemantics(_))
    ));
    let newer = PricingSourceInput {
        expected_version: 2,
        effective_from: Utc::now() - Duration::milliseconds(1),
        ..input.clone()
    };
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM operations")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE FUNCTION reject_pricing_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='pricing-source.create' THEN RAISE EXCEPTION 'fixture audit unavailable'; END IF; RETURN NEW; END; $$; CREATE TRIGGER reject_pricing_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_pricing_audit();").execute(&store.pool).await.unwrap();
    assert!(write(&store, &vault, &newer).await.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM operations")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        before
    );
    assert_eq!(
        store
            .resolve_pricing(conn, "same-model", &usd)
            .await
            .unwrap()
            .policy_version,
        2
    );
    sqlx::raw_sql(
        "DROP TRIGGER reject_pricing_audit ON audit_events; DROP FUNCTION reject_pricing_audit();",
    )
    .execute(&store.pool)
    .await
    .unwrap();
    // Auto needs qualified native currency and structural catalog provenance, not a quote label.
    let auto = PricingSourceInput {
        mode: PricingMode::ProviderAuto,
        manual_price_revision_id: None,
        ..newer.clone()
    };
    assert!(matches!(
        write(&store, &vault, &auto).await,
        Err(HubError::PreconditionFailed)
    ));
    let op = Uuid::new_v4();
    let probe = Uuid::new_v4();
    let evidence = Uuid::new_v4();
    let qualification = Uuid::new_v4();
    sqlx::query("INSERT INTO operations(id,installation_id,principal_kind,principal_id,idempotency_key,binding_hmac,action,state,expires_at) VALUES($1,$2,'internal','fixture',$1,$3,'catalog-preflight','succeeded',now()+interval '1 hour')").bind(op).bind(installation).bind(vec![7u8;32]).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO probe_snapshots(id,installation_id,operation_id,scope,config_hash,configuration) VALUES($1,$2,$3,'connection_model',$4,'{}')").bind(probe).bind(installation).bind(op).bind("b".repeat(64)).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO verification_evidence(id,installation_id,probe_snapshot_id,state,child_evidence,expires_at) VALUES($1,$2,$3,'verified','[]',now()+interval '1 hour')").bind(evidence).bind(installation).bind(probe).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO runtime_qualifications(id,installation_id,connection_id,generation,provider_model_id,adapter_revision,endpoint_policy_hash,proof_id,capabilities,state,billing_currency,billing_currency_origin) VALUES($1,$2,$3,1,'same-model','synthetic-pricing-v1',$4,$5,'{\"pricing_reader\":true}','active','USD','synthetic-native-currency-witness')").bind(qualification).bind(installation).bind(conn).bind("a".repeat(64)).bind(evidence).execute(&store.pool).await.unwrap();
    assert!(matches!(
        write(&store, &vault, &auto).await,
        Err(HubError::PreconditionFailed)
    ));
    let model = Uuid::new_v4();
    let observed:chrono::DateTime<Utc>=sqlx::query_scalar("INSERT INTO upstream_models(id,connection_id,generation,provider_model_id,metadata,observed_at) VALUES($1,$2,1,'same-model','{}',clock_timestamp()) RETURNING observed_at").bind(model).bind(conn).fetch_one(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO catalog_price_origins(price_revision_id,installation_id,catalog_model_id,observed_at) VALUES($1,$2,$3,$4)").bind(price).bind(installation).bind(model).bind(observed).execute(&store.pool).await.unwrap();
    let automatic = write(&store, &vault, &auto).await.unwrap();
    assert_eq!(automatic.value.version, 3);
    assert!(automatic.value.catalog_observed_at.is_some());
    assert_eq!(
        store
            .resolve_pricing(conn, "same-model", &usd)
            .await
            .unwrap()
            .price_revision_id,
        Some(price)
    );
    sqlx::query("UPDATE runtime_qualifications SET state='invalidated' WHERE id=$1")
        .bind(qualification)
        .execute(&store.pool)
        .await
        .unwrap();
    let unavailable = store
        .resolve_pricing(conn, "same-model", &usd)
        .await
        .unwrap();
    assert_eq!(unavailable.source_revision_id, Some(automatic.value.id));
    assert!(unavailable.price_revision_id.is_none());
    let page = store.pricing_source_page("actor", 100, None).await.unwrap();
    assert!(matches!(
        page.items
            .iter()
            .find(|s| s.id == automatic.value.id)
            .unwrap()
            .data_status,
        PricingDataStatus::Unavailable
    ));
    assert_eq!(
        store
            .create_pricing_source("actor", key, binding, &input)
            .await
            .unwrap()
            .value
            .version,
        1,
        "original readback stays pinned after revisions"
    );
    assert!(
        sqlx::query("UPDATE pricing_source_revisions SET mode='provider_auto' WHERE id=$1")
            .bind(a.value.id)
            .execute(&store.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE price_revisions SET input_uncached=0 WHERE id=$1")
            .bind(price)
            .execute(&store.pool)
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM ledger_entries")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        0
    );
}
