use aihub_application::FoundationStore;
use aihub_domain::error::HubError;
use aihub_infrastructure::{openrouter_metadata::decode_catalog, postgres::PgStore};
use uuid::Uuid;
#[tokio::test]
#[ignore = "requires explicit isolated PostgreSQL17 fixture"]
async fn generation_bound_catalog_snapshots_search_cursor_and_immutability() {
    let dsn = std::env::var("AIHUB_TEST_DATABASE_URL").unwrap();
    let installation = Uuid::new_v4();
    let store = PgStore::connect(&dsn, installation, vec![7; 32])
        .await
        .unwrap();
    store.migrate().await.unwrap();
    store.initialize("catalog-fixture").await.unwrap();
    store.ready().await.unwrap();
    let provider: Uuid = sqlx::query_scalar(
        "SELECT id FROM providers WHERE installation_id=$1 AND kind='openai_compatible'",
    )
    .bind(installation)
    .fetch_one(&store.pool)
    .await
    .unwrap();
    let conn = Uuid::new_v4();
    let hash = "a".repeat(64);
    sqlx::query("INSERT INTO connections(id,installation_id,provider_id,display_name,endpoint_policy_ref,billing_mode,status) VALUES($1,$2,$3,'catalog-fixture','synthetic','metered','authorization_unknown')").bind(conn).bind(installation).bind(provider).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO connection_generations(connection_id,generation,authorization_state,adapter_revision,endpoint_policy_hash) VALUES($1,1,'prepared','openrouter-metadata-v1',$2)").bind(conn).bind(&hash).execute(&store.pool).await.unwrap();
    let empty = FoundationStore::catalog_page(&store, "actor", conn, "", 100, None)
        .await
        .unwrap();
    assert_eq!(empty.data_status, "partial");
    assert!(empty.models.is_empty());
    let observation=decode_catalog(br#"{"data":[{"id":"same-a","context_length":4096},{"id":"same-b","context_length":8192}]}"#,chrono::Utc::now()).unwrap();
    let snapshot = store
        .store_catalog(conn, 1, &hash, &observation)
        .await
        .unwrap();
    let first = FoundationStore::catalog_page(&store, "actor", conn, "same", 1, None)
        .await
        .unwrap();
    assert_eq!(first.models.len(), 1);
    let cursor = Uuid::parse_str(first.next_cursor.as_ref().unwrap()).unwrap();
    assert_eq!(
        FoundationStore::catalog_page(&store, "actor", conn, "SAME-B", 100, None)
            .await
            .unwrap()
            .models[0]
            .provider_model_id,
        "same-b"
    );
    assert!(
        FoundationStore::catalog_page(&store, "other", conn, "same", 1, Some(cursor))
            .await
            .is_err()
    );
    assert!(
        FoundationStore::catalog_page(&store, "actor", conn, "different", 1, Some(cursor))
            .await
            .is_err()
    );
    let refresh = decode_catalog(
        br#"{"data":[{"id":"new-model","context_length":16384}]}"#,
        chrono::Utc::now(),
    )
    .unwrap();
    store.store_catalog(conn, 1, &hash, &refresh).await.unwrap();
    let old_page = FoundationStore::catalog_page(&store, "actor", conn, "same", 1, Some(cursor))
        .await
        .unwrap();
    assert_eq!(old_page.models[0].provider_model_id, "same-b");
    assert_eq!(old_page.as_of, observation.observed_at);
    let current = FoundationStore::catalog_page(&store, "actor", conn, "", 100, None)
        .await
        .unwrap();
    assert_eq!(current.models.len(), 1);
    assert_eq!(current.models[0].provider_model_id, "new-model");
    assert_eq!(current.models[0].evidence_status, "unverified");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM upstream_models WHERE connection_id=$1")
            .bind(conn)
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        3,
        "removed catalog membership does not delete historical model rows"
    );
    assert!(
        sqlx::query("UPDATE catalog_snapshots SET models='[]' WHERE id=$1")
            .bind(snapshot)
            .execute(&store.pool)
            .await
            .is_err()
    );
    assert!(matches!(
        store
            .store_catalog(conn, 1, &"b".repeat(64), &refresh)
            .await,
        Err(HubError::PreconditionFailed)
    ));
    sqlx::query("INSERT INTO connection_generations(connection_id,generation,authorization_state,adapter_revision,endpoint_policy_hash) VALUES($1,2,'prepared','openrouter-metadata-v1',$2)").bind(conn).bind(&hash).execute(&store.pool).await.unwrap();
    sqlx::query("UPDATE connections SET generation=2 WHERE id=$1")
        .bind(conn)
        .execute(&store.pool)
        .await
        .unwrap();
    assert!(matches!(
        store.store_catalog(conn, 1, &hash, &refresh).await,
        Err(HubError::PreconditionFailed)
    ));
    let new_generation = FoundationStore::catalog_page(&store, "actor", conn, "", 100, None)
        .await
        .unwrap();
    assert_eq!(new_generation.generation, 2);
    assert!(new_generation.models.is_empty());
    assert_eq!(new_generation.data_status, "partial");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM runtime_qualifications")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM ledger_entries")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        0
    );
}
