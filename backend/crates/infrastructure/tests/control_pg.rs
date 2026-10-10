use aihub_application::{FoundationStore, OperationBinding};
use aihub_domain::{
    error::HubError,
    financial::{Currency, Rate},
    prices::{PriceInput, PriceUnit},
};
use aihub_infrastructure::{postgres::PgStore, vault::Vault};
use chrono::{Duration, Utc};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires explicit isolated PostgreSQL17 fixture"]
async fn price_control_is_atomic_idempotent_and_snapshot_bound() {
    let dsn = std::env::var("AIHUB_TEST_DATABASE_URL").unwrap();
    let installation = Uuid::new_v4();
    let vault = Vault::new(vec![7; 32]).unwrap();
    let store = PgStore::connect(&dsn, installation, vault.fingerprint())
        .await
        .unwrap();
    store.migrate().await.unwrap();
    store.initialize("control-fixture").await.unwrap();
    store.ready().await.unwrap();
    let provider: Uuid = sqlx::query_scalar(
        "SELECT id FROM providers WHERE installation_id=$1 AND kind='openai_compatible'",
    )
    .bind(installation)
    .fetch_one(&store.pool)
    .await
    .unwrap();
    let connection = Uuid::new_v4();
    sqlx::query("INSERT INTO connections(id,installation_id,provider_id,display_name,endpoint_policy_ref,billing_mode,status) VALUES($1,$2,$3,'control-fixture','synthetic-fixed','metered','enabled')").bind(connection).bind(installation).bind(provider).execute(&store.pool).await.unwrap();
    let price = PriceInput {
        connection_id: connection,
        model_id: "fixture-model".into(),
        tier: "metered".into(),
        currency: Currency::parse("USD").unwrap(),
        unit: PriceUnit::PerMillionTokens,
        input_uncached: Rate::parse("2").unwrap(),
        input_cached: None,
        output_billable: Rate::parse("8").unwrap(),
        effective_from: Utc::now() - Duration::minutes(1),
        effective_to: None,
        source: "synthetic-fixture".into(),
        request_fee: None,
    };
    let key = Uuid::new_v4();
    let binding = vault.bind(&serde_json::to_value(&price).unwrap()).unwrap();
    let second = PgStore::connect(&dsn, installation, vault.fingerprint())
        .await
        .unwrap();
    let (one, two) = tokio::join!(
        store.create_price("actor-a", key, binding, &price),
        second.create_price("actor-a", key, binding, &price)
    );
    let (one, two) = (one.unwrap(), two.unwrap());
    assert_eq!(one.operation_id, two.operation_id);
    assert_eq!(one.value.id, two.value.id);
    assert_eq!(one.value.price.request_fee, None, "missing fee is unknown");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM price_revisions")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM audit_events WHERE action='price.create'"
        )
        .fetch_one(&store.pool)
        .await
        .unwrap(),
        1
    );
    let changed = PriceInput {
        source: "another-price-source".into(),
        ..price.clone()
    };
    let changed_binding = vault
        .bind(&serde_json::to_value(&changed).unwrap())
        .unwrap();
    assert!(matches!(
        store
            .create_price("actor-a", key, changed_binding, &changed)
            .await,
        Err(HubError::IdempotencyConflict)
    ));
    assert_eq!(
        store
            .operation("actor-a", one.operation_id)
            .await
            .unwrap()
            .resource_id,
        Some(one.value.id)
    );
    assert!(matches!(
        store.operation("actor-b", one.operation_id).await,
        Err(HubError::NotFound)
    ));
    // The same key in another actor's namespace is an independent control operation.
    let other = store
        .create_price("actor-b", key, binding, &price)
        .await
        .unwrap();
    assert_ne!(other.value.id, one.value.id);
    let initial = store.price_page("actor-a", 1, None).await.unwrap();
    let cursor = Uuid::parse_str(initial.next_cursor.as_ref().unwrap()).unwrap();
    assert!(store.price_page("actor-b", 1, Some(cursor)).await.is_err());
    assert!(store.price_page("actor-a", 2, Some(cursor)).await.is_err());
    let third = store
        .create_price("actor-a", Uuid::new_v4(), changed_binding, &changed)
        .await
        .unwrap();
    let next = store.price_page("actor-a", 1, Some(cursor)).await.unwrap();
    assert_eq!(next.items.len(), 1);
    assert!(next.next_cursor.is_none());
    assert_ne!(
        next.items[0].id, third.value.id,
        "new quote cannot enter an old snapshot"
    );
    assert!(
        sqlx::query("UPDATE price_revisions SET request_fee=0")
            .execute(&store.pool)
            .await
            .is_err()
    );
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM operations")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE FUNCTION reject_price_fixture_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='price.create' THEN RAISE EXCEPTION 'fixture audit unavailable'; END IF; RETURN NEW; END; $$; CREATE TRIGGER reject_price_fixture_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_price_fixture_audit();").execute(&store.pool).await.unwrap();
    assert!(
        store
            .create_price("actor-a", Uuid::new_v4(), binding, &price)
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM operations")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        before
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM price_revisions")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        3
    );
}
