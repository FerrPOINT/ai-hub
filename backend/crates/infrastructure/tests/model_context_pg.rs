use aihub_application::{FoundationStore, OperationBinding};
use aihub_domain::{
    error::HubError,
    model_context::{ModelContextInput, ModelContextSnapshot},
};
use aihub_infrastructure::{postgres::PgStore, vault::Vault};
use uuid::Uuid;
#[tokio::test]
#[ignore = "requires explicit isolated PostgreSQL17 fixture"]
async fn model_context_exact_cas_replay_immutable_history_and_atomic_audit() {
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
    store.initialize("model-context-fixture").await.unwrap();
    let provider: Uuid = sqlx::query_scalar(
        "SELECT id FROM providers WHERE installation_id=$1 AND kind='openai_compatible'",
    )
    .bind(installation)
    .fetch_one(&store.pool)
    .await
    .unwrap();
    let conn = Uuid::new_v4();
    sqlx::query("INSERT INTO connections(id,installation_id,provider_id,display_name,endpoint_policy_ref,billing_mode,status) VALUES($1,$2,$3,'context','synthetic','unknown','authorization_unknown')").bind(conn).bind(installation).bind(provider).execute(&store.pool).await.unwrap();
    let first = ModelContextInput {
        model_id: "vendor/CaseSensitive-model".into(),
        context_window_tokens: 4096,
    };
    let key = Uuid::new_v4();
    let binding = vault
        .bind(&serde_json::json!({"key":key,"input":first}))
        .unwrap();
    let (one, two) = tokio::join!(
        store.save_model_context("actor", key, binding, conn, 0, &first),
        store.save_model_context("actor", key, binding, conn, 0, &first)
    );
    let one = one.unwrap();
    assert_eq!(one.operation_id, two.unwrap().operation_id);
    assert_eq!(one.preference.version, 1);
    assert!(matches!(
        store
            .save_model_context("actor", Uuid::new_v4(), [8; 32], conn, 0, &first)
            .await,
        Err(HubError::PreconditionFailed)
    ));
    let second = ModelContextInput {
        model_id: first.model_id.clone(),
        context_window_tokens: 8192,
    };
    let saved = store
        .save_model_context("actor", Uuid::new_v4(), [9; 32], conn, 1, &second)
        .await
        .unwrap();
    assert_eq!(saved.preference.version, 2);
    assert_eq!(
        store
            .save_model_context("actor", key, binding, conn, 0, &first)
            .await
            .unwrap()
            .preference
            .context_window_tokens,
        4096,
        "original successful reply is immutable"
    );
    let exact = store
        .model_context_page("actor", conn, Some(&first.model_id), 50, None)
        .await
        .unwrap();
    assert_eq!(exact.items[0].context_window_tokens, 8192);
    assert!(
        store
            .model_context_page("actor", conn, Some("vendor/casesensitive-model"), 50, None)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let long = ModelContextInput {
        model_id: format!("vendor/{}", "x".repeat(200)),
        context_window_tokens: u32::MAX,
    };
    store
        .save_model_context("actor", Uuid::new_v4(), [10; 32], conn, 0, &long)
        .await
        .unwrap();
    let page = store
        .model_context_page("actor", conn, None, 1, None)
        .await
        .unwrap();
    let cursor = Uuid::parse_str(page.next_cursor.as_ref().unwrap()).unwrap();
    assert!(
        store
            .model_context_page("other", conn, None, 1, Some(cursor))
            .await
            .is_err()
    );
    assert!(
        store
            .model_context_page("actor", conn, Some(&first.model_id), 1, Some(cursor))
            .await
            .is_err()
    );
    let snapshot:ModelContextSnapshot=serde_json::from_value(sqlx::query_scalar::<_,serde_json::Value>("SELECT jsonb_build_object('revision_id',current_revision_id,'version',version,'context_window_tokens',8192) FROM model_context_preferences WHERE connection_id=$1 AND model_id=$2").bind(conn).bind(&first.model_id).fetch_one(&store.pool).await.unwrap()).unwrap();
    assert_eq!(snapshot.version, 2);
    assert!(
        sqlx::query("UPDATE model_context_revisions SET context_window_tokens=1 WHERE id=$1")
            .bind(snapshot.revision_id)
            .execute(&store.pool)
            .await
            .is_err()
    );
    sqlx::raw_sql("CREATE FUNCTION reject_model_context_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='model-context.write' THEN RAISE EXCEPTION 'fixture audit unavailable'; END IF; RETURN NEW; END; $$; CREATE TRIGGER reject_model_context_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_model_context_audit();").execute(&store.pool).await.unwrap();
    assert!(
        store
            .save_model_context("actor", Uuid::new_v4(), [11; 32], conn, 2, &first)
            .await
            .is_err()
    );
    assert_eq!(
        store
            .model_context_page("actor", conn, Some(&first.model_id), 50, None)
            .await
            .unwrap()
            .items[0]
            .version,
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM model_context_revisions WHERE connection_id=$1 AND model_id=$2"
        )
        .bind(conn)
        .bind(&first.model_id)
        .fetch_one(&store.pool)
        .await
        .unwrap(),
        2
    );
}
