use aihub_application::{FoundationStore, OperationBinding};
use aihub_domain::{
    connections::{BillingMode, ConnectionInput, EndpointPolicyInput, ProviderKind},
    error::HubError,
};
use aihub_infrastructure::{postgres::PgStore, vault::Vault};
use uuid::Uuid;
#[tokio::test]
#[ignore = "requires explicit isolated PostgreSQL17 fixture"]
async fn connection_endpoint_cas_generation_label_and_atomic_audit() {
    let dsn = std::env::var("AIHUB_TEST_DATABASE_URL").unwrap();
    let vault = Vault::new(vec![7; 32]).unwrap();
    let installation = Uuid::new_v4();
    let store = PgStore::connect(&dsn, installation, vault.fingerprint())
        .await
        .unwrap();
    store.migrate().await.unwrap();
    store.initialize("connection-fixture").await.unwrap();
    store.ready().await.unwrap();
    let endpoint = EndpointPolicyInput {
        policy_ref: "fixture-v1".into(),
        provider_kind: ProviderKind::OpenaiCompatible,
        base_url: "https://api.example.test/v1/".into(),
        allow_loopback: false,
    };
    store
        .configure_endpoints(&[endpoint.clone()])
        .await
        .unwrap();
    store
        .configure_endpoints(&[endpoint.clone()])
        .await
        .unwrap();
    assert!(matches!(
        store
            .configure_endpoints(&[EndpointPolicyInput {
                base_url: "https://other.example.test/v1/".into(),
                ..endpoint.clone()
            }])
            .await,
        Err(HubError::IdempotencyConflict)
    ));
    let second_endpoint = EndpointPolicyInput {
        policy_ref: "fixture-v2".into(),
        base_url: "https://other.example.test/v1/".into(),
        ..endpoint.clone()
    };
    store.configure_endpoints(&[second_endpoint]).await.unwrap();
    let input = ConnectionInput {
        display_name: "same name".into(),
        endpoint_policy_ref: endpoint.policy_ref.clone(),
        billing_mode: BillingMode::Metered,
    };
    let key = Uuid::new_v4();
    let hash = vault.bind(&serde_json::to_value(&input).unwrap()).unwrap();
    let second = PgStore::connect(&dsn, installation, vault.fingerprint())
        .await
        .unwrap();
    let (one, two) = tokio::join!(
        FoundationStore::save_connection(
            &store,
            "actor",
            key,
            hash,
            ProviderKind::OpenaiCompatible,
            None,
            &input
        ),
        FoundationStore::save_connection(
            &second,
            "actor",
            key,
            hash,
            ProviderKind::OpenaiCompatible,
            None,
            &input
        )
    );
    let (one, two) = (one.unwrap(), two.unwrap());
    assert_eq!(one.operation_id, two.operation_id);
    assert_eq!(one.value.id, two.value.id);
    assert_eq!(one.value.status, "authorization_unknown");
    assert!(!one.value.has_credentials);
    assert!(one.value.quota.is_none());
    assert!(matches!(
        FoundationStore::save_connection(
            &store,
            "actor",
            Uuid::new_v4(),
            hash,
            ProviderKind::Zai,
            None,
            &input
        )
        .await,
        Err(HubError::InvalidSemantics(_))
    ));
    let peer = FoundationStore::save_connection(
        &store,
        "actor",
        Uuid::new_v4(),
        hash,
        ProviderKind::OpenaiCompatible,
        None,
        &input,
    )
    .await
    .unwrap();
    assert_ne!(one.value.id, peer.value.id, "display name is not identity");
    sqlx::query("UPDATE connections SET status='enabled' WHERE id=$1")
        .bind(one.value.id)
        .execute(&store.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE connection_generations SET authorization_state='active',billing_tier='metered' WHERE connection_id=$1 AND generation=1").bind(one.value.id).execute(&store.pool).await.unwrap();
    let rename = ConnectionInput {
        display_name: "renamed".into(),
        ..input.clone()
    };
    let rename_key = Uuid::new_v4();
    let rename_hash = vault.bind(&serde_json::to_value(&rename).unwrap()).unwrap();
    let renamed = FoundationStore::save_connection(
        &store,
        "actor",
        rename_key,
        rename_hash,
        ProviderKind::OpenaiCompatible,
        Some((one.value.id, 1)),
        &rename,
    )
    .await
    .unwrap();
    assert_eq!(renamed.value.generation, 1);
    assert_eq!(renamed.value.version, 2);
    assert_eq!(renamed.value.status, "enabled");
    assert!(matches!(
        FoundationStore::save_connection(
            &store,
            "actor",
            Uuid::new_v4(),
            rename_hash,
            ProviderKind::OpenaiCompatible,
            Some((one.value.id, 1)),
            &rename
        )
        .await,
        Err(HubError::PreconditionFailed)
    ));
    let changed = ConnectionInput {
        endpoint_policy_ref: "fixture-v2".into(),
        ..rename.clone()
    };
    let changed_hash = vault
        .bind(&serde_json::to_value(&changed).unwrap())
        .unwrap();
    let (a, b) = tokio::join!(
        FoundationStore::save_connection(
            &store,
            "actor",
            Uuid::new_v4(),
            changed_hash,
            ProviderKind::OpenaiCompatible,
            Some((one.value.id, 2)),
            &changed
        ),
        FoundationStore::save_connection(
            &second,
            "actor",
            Uuid::new_v4(),
            changed_hash,
            ProviderKind::OpenaiCompatible,
            Some((one.value.id, 2)),
            &changed
        )
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let updated = a.or(b).unwrap();
    assert_eq!(updated.value.generation, 2);
    assert_eq!(updated.value.status, "authorization_unknown");
    assert!(!updated.value.has_credentials);
    assert_eq!(
        FoundationStore::save_connection(
            &store,
            "actor",
            rename_key,
            rename_hash,
            ProviderKind::OpenaiCompatible,
            Some((one.value.id, 1)),
            &rename
        )
        .await
        .unwrap()
        .value
        .version,
        2,
        "original readback remains pinned"
    );
    assert!(sqlx::query("UPDATE connection_generations SET endpoint_policy_hash=$1 WHERE connection_id=$2 AND generation=1").bind("x".repeat(64)).bind(one.value.id).execute(&store.pool).await.is_err());
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM operations")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE FUNCTION reject_connection_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='connection.create' THEN RAISE EXCEPTION 'fixture audit unavailable'; END IF; RETURN NEW; END; $$; CREATE TRIGGER reject_connection_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_connection_audit();").execute(&store.pool).await.unwrap();
    assert!(
        FoundationStore::save_connection(
            &store,
            "actor",
            Uuid::new_v4(),
            hash,
            ProviderKind::OpenaiCompatible,
            None,
            &input
        )
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
        FoundationStore::connection_page(&store, "actor", 100, None)
            .await
            .unwrap()
            .items
            .len(),
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM credential_versions")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        0
    );
}
