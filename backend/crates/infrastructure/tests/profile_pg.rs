use aihub_application::{FoundationStore, OperationBinding};
use aihub_domain::{
    connections::{BillingMode, ConnectionInput, EndpointPolicyInput, ProviderKind},
    error::HubError,
    profiles::{ProfileInput, ProfileMode},
};
use aihub_infrastructure::{postgres::PgStore, vault::Vault};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires explicit isolated PostgreSQL17 fixture"]
async fn profile_draft_ownership_cas_replay_history_and_atomic_audit() {
    let installation = Uuid::new_v4();
    let vault = Vault::new(vec![7; 32]).unwrap();
    let store = PgStore::connect(
        &std::env::var("AIHUB_TEST_DATABASE_URL").unwrap(),
        installation,
        vault.fingerprint(),
    )
    .await
    .unwrap();
    store.migrate().await.unwrap();
    store.initialize("profile-fixture").await.unwrap();
    store
        .configure_endpoints(&[EndpointPolicyInput {
            policy_ref: "profile-fixture".into(),
            provider_kind: ProviderKind::OpenaiCompatible,
            base_url: "https://api.example.test/v1/".into(),
            allow_loopback: false,
        }])
        .await
        .unwrap();
    let connection = store
        .save_connection(
            "actor",
            Uuid::new_v4(),
            [1; 32],
            ProviderKind::OpenaiCompatible,
            None,
            &ConnectionInput {
                display_name: "Unqualified connection".into(),
                endpoint_policy_ref: "profile-fixture".into(),
                billing_mode: BillingMode::Unknown,
            },
        )
        .await
        .unwrap()
        .value;
    let input:ProfileInput=serde_json::from_value(serde_json::json!({"slug":"dev-profile","display_name":"Черновик","mode":"development","deployments":[{"connection_id":connection.id,"generation":connection.generation,"model_id":"vendor/CaseSensitive"},{"connection_id":connection.id,"generation":connection.generation,"model_id":"vendor/casesensitive"}],"parameters":{"temperature":0.1,"top_p":0.8,"reasoning_effort":"low"},"input_limit":2048,"output_limit":512,"context_limit":4096,"required_capabilities":["text","stream"],"timeout_seconds":120,"max_attempts":3,"allowed_overrides":["temperature"]})).unwrap();
    let key = Uuid::new_v4();
    let binding = vault.bind(&serde_json::to_value(&input).unwrap()).unwrap();
    let (one, two) = tokio::join!(
        store.save_profile("actor", key, binding, None, &input),
        store.save_profile("actor", key, binding, None, &input)
    );
    let one = one.unwrap();
    assert_eq!(one.operation_id, two.unwrap().operation_id);
    let id = one.profile.id;
    assert_eq!(one.profile.status, "draft");
    assert_eq!(one.profile.draft_version, 1);
    assert!(one.profile.active_revision_id.is_none());
    assert_eq!(
        serde_json::to_value(&one.profile.draft).unwrap(),
        serde_json::to_value(&input).unwrap()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM requests")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        0,
        "draft needs no inference intent or provider proof"
    );
    assert!(matches!(
        store
            .save_profile("actor", Uuid::new_v4(), [2; 32], None, &input)
            .await,
        Err(HubError::AlreadyExists)
    ));
    assert!(matches!(
        store
            .save_profile("actor", key, [2; 32], None, &input)
            .await,
        Err(HubError::IdempotencyConflict)
    ));
    let mut updated = input.clone();
    updated.display_name = "Новая версия".into();
    updated.deployments.reverse();
    updated.parameters.temperature = Some(0.7);
    let (a, b) = tokio::join!(
        store.save_profile("actor", Uuid::new_v4(), [3; 32], Some((id, 1)), &updated),
        store.save_profile("actor", Uuid::new_v4(), [4; 32], Some((id, 1)), &updated)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(matches!(
        a.as_ref().err().or(b.as_ref().err()),
        Some(HubError::PreconditionFailed)
    ));
    let current = store.read_profile(id).await.unwrap();
    assert_eq!(current.draft_version, 2);
    assert!(current.active_revision_id.is_none());
    let replay = store
        .save_profile("actor", key, binding, None, &input)
        .await
        .unwrap();
    assert_eq!(replay.profile.draft_version, 1);
    assert_eq!(replay.profile.draft.display_name, "Черновик");
    let historical: serde_json::Value = sqlx::query_scalar(
        "SELECT configuration FROM profile_draft_revisions WHERE virtual_model_id=$1 AND version=1",
    )
    .bind(id)
    .fetch_one(&store.pool)
    .await
    .unwrap();
    assert_eq!(
        historical["deployments"][0]["model_id"],
        "vendor/CaseSensitive"
    );
    let ordered:Vec<String>=sqlx::query_scalar("SELECT model_id FROM profile_draft_targets WHERE virtual_model_id=$1 AND version=2 ORDER BY ordinal").bind(id).fetch_all(&store.pool).await.unwrap();
    assert_eq!(
        ordered,
        vec!["vendor/casesensitive", "vendor/CaseSensitive"]
    );
    let mut renamed = updated.clone();
    renamed.slug = "other-slug".into();
    assert!(matches!(
        store
            .save_profile("actor", Uuid::new_v4(), [5; 32], Some((id, 2)), &renamed)
            .await,
        Err(HubError::InvalidSemantics(_))
    ));
    let mut denied = input.clone();
    denied.slug = input.slug.clone();
    denied.deployments[0].connection_id = connection.id;
    denied.deployments[0].generation += 20;
    assert!(matches!(
        store
            .save_profile("actor", Uuid::new_v4(), [7; 32], Some((id, 2)), &denied)
            .await,
        Err(HubError::InvalidSemantics(_))
    ));
    // A pinned neighboring draft is valid without credentials or catalog advertisement.
    let mut pinned = input.clone();
    pinned.slug = "pinned-profile".into();
    pinned.mode = ProfileMode::PinnedTest;
    pinned.max_attempts = 1;
    pinned.deployments.truncate(1);
    pinned.deployments[0].model_id = format!("vendor/{}", "x".repeat(200));
    store
        .save_profile("actor", Uuid::new_v4(), [8; 32], None, &pinned)
        .await
        .unwrap();
    let page = store.profile_page("actor", 1, None).await.unwrap();
    let cursor = Uuid::parse_str(page.next_cursor.as_ref().unwrap()).unwrap();
    assert!(store.profile_page("other", 1, Some(cursor)).await.is_err());
    assert!(store.profile_page("actor", 2, Some(cursor)).await.is_err());
    let mut later = pinned.clone();
    later.slug = "after-snapshot".into();
    store
        .save_profile("actor", Uuid::new_v4(), [9; 32], None, &later)
        .await
        .unwrap();
    let rest = store.profile_page("actor", 1, Some(cursor)).await.unwrap();
    assert_eq!(rest.items.len(), 1);
    assert!(rest.next_cursor.is_none());
    assert_ne!(rest.items[0].draft.slug, "after-snapshot");
    for query in [
        "UPDATE profile_draft_revisions SET config_hash=repeat('0',64) WHERE virtual_model_id=$1",
        "DELETE FROM profile_draft_targets WHERE virtual_model_id=$1",
        "UPDATE virtual_models SET draft_version=999 WHERE id=$1",
    ] {
        assert!(
            sqlx::query(query)
                .bind(id)
                .execute(&store.pool)
                .await
                .is_err()
        );
    }
    let fence = Uuid::new_v4();
    store
        .close_unstarted_operation("actor", fence, [10; 32])
        .await
        .unwrap();
    assert!(matches!(
        store
            .save_profile("actor", fence, [10; 32], Some((id, 2)), &updated)
            .await,
        Err(HubError::IdempotencyConflict)
    ));
    sqlx::raw_sql("CREATE FUNCTION reject_profile_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='profile.draft.update' THEN RAISE EXCEPTION 'fixture audit unavailable'; END IF; RETURN NEW; END; $$; CREATE TRIGGER reject_profile_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_profile_audit();").execute(&store.pool).await.unwrap();
    let failure_key = Uuid::new_v4();
    assert!(matches!(
        store
            .save_profile("actor", failure_key, [11; 32], Some((id, 2)), &updated)
            .await,
        Err(HubError::Unavailable)
    ));
    assert_eq!(store.read_profile(id).await.unwrap().draft_version, 2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM profile_draft_revisions WHERE virtual_model_id=$1"
        )
        .bind(id)
        .fetch_one(&store.pool)
        .await
        .unwrap(),
        2
    );
    assert!(matches!(
        store.operation_key("actor", failure_key).await,
        Err(HubError::NotFound)
    ));
    sqlx::raw_sql(
        "DROP TRIGGER reject_profile_audit ON audit_events; DROP FUNCTION reject_profile_audit();",
    )
    .execute(&store.pool)
    .await
    .unwrap();
    sqlx::query("UPDATE virtual_models SET status='disabled' WHERE id=$1")
        .bind(id)
        .execute(&store.pool)
        .await
        .unwrap();
    assert_eq!(
        store
            .save_profile("actor", Uuid::new_v4(), [12; 32], Some((id, 2)), &updated)
            .await
            .unwrap()
            .profile
            .status,
        "disabled"
    );
    sqlx::query("UPDATE virtual_models SET status='archived' WHERE id=$1")
        .bind(id)
        .execute(&store.pool)
        .await
        .unwrap();
    assert!(matches!(
        store
            .save_profile("actor", Uuid::new_v4(), [13; 32], Some((id, 3)), &updated)
            .await,
        Err(HubError::PreconditionFailed)
    ));
    // Insert a foreign installation last: installation readiness rejects multi-installation databases.
    let foreign = Uuid::new_v4();
    let foreign_provider = Uuid::new_v4();
    let foreign_connection = Uuid::new_v4();
    sqlx::query("INSERT INTO installations(id,stable_key,status,vault_key_fingerprint) VALUES($1,'foreign','active',$2)").bind(foreign).bind(vault.fingerprint()).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO providers(id,installation_id,kind,display_name) VALUES($1,$2,'openai_compatible','foreign')").bind(foreign_provider).bind(foreign).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO connections(id,installation_id,provider_id,display_name,endpoint_policy_ref,billing_mode,status) VALUES($1,$2,$3,'foreign','fixture','unknown','authorization_unknown')").bind(foreign_connection).bind(foreign).bind(foreign_provider).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO connection_generations(connection_id,generation,authorization_state,adapter_revision,endpoint_policy_hash) VALUES($1,1,'absent','fixture-v1',$2)").bind(foreign_connection).bind("f".repeat(64)).execute(&store.pool).await.unwrap();
    let mut denied = input.clone();
    denied.slug = "foreign-draft".into();
    denied.deployments[0].connection_id = foreign_connection;
    assert!(matches!(
        store
            .save_profile("actor", Uuid::new_v4(), [6; 32], None, &denied)
            .await,
        Err(HubError::InvalidSemantics(_))
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM virtual_models WHERE slug='foreign-draft'"
        )
        .fetch_one(&store.pool)
        .await
        .unwrap(),
        0
    );
    assert!(matches!(
        store.ready().await,
        Err(HubError::InstallationMismatch)
    ));
}
