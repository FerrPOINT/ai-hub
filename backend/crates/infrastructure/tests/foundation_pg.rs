use aihub_application::FoundationStore;
use aihub_infrastructure::postgres::PgStore;
use sqlx::Row;
use uuid::Uuid;

/// Run only in the repository-owned disposable PostgreSQL harness. No skip-as-pass.
#[tokio::test]
#[ignore = "requires explicit isolated PostgreSQL 17 fixture"]
async fn actual_foundation_constraints_and_atomic_initialize() {
    let dsn = std::env::var("AIHUB_TEST_DATABASE_URL").expect("explicit fixture DSN required");
    let installation = Uuid::new_v4();
    let store = PgStore::connect(&dsn, installation, vec![7; 32])
        .await
        .unwrap();
    store.migrate().await.unwrap();
    assert!(store.ready().await.is_err());
    sqlx::raw_sql("CREATE FUNCTION reject_fixture_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture audit failure'; END; $$; CREATE TRIGGER reject_fixture_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_fixture_audit();").execute(&store.pool).await.unwrap();
    assert!(store.initialize("must-roll-back").await.is_err());
    for table in ["installations", "providers", "operations", "audit_events"] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&store.pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "partial bootstrap must roll back {table}");
    }
    sqlx::raw_sql(
        "DROP TRIGGER reject_fixture_audit ON audit_events; DROP FUNCTION reject_fixture_audit();",
    )
    .execute(&store.pool)
    .await
    .unwrap();
    let (first, second) = tokio::join!(
        store.initialize("disposable-foundation-test"),
        store.initialize("disposable-foundation-test")
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    store.ready().await.unwrap();
    assert!(store.initialize("replacement-denied").await.is_err());
    let foreign = PgStore::connect(&dsn, Uuid::new_v4(), vec![7; 32])
        .await
        .unwrap();
    assert!(foreign.ready().await.is_err());
    let wrong_key = PgStore::connect(&dsn, installation, vec![8; 32])
        .await
        .unwrap();
    assert!(wrong_key.ready().await.is_err());
    let audit = sqlx::query("SELECT id,operation_id FROM audit_events")
        .fetch_all(&store.pool)
        .await
        .unwrap();
    assert_eq!(audit.len(), 1);
    let id: Uuid = audit[0].get("id");
    assert!(
        sqlx::query("UPDATE audit_events SET reason='tampered' WHERE id=$1")
            .bind(id)
            .execute(&store.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM audit_events WHERE id=$1")
            .bind(id)
            .execute(&store.pool)
            .await
            .is_err()
    );
    let providers: i64 = sqlx::query_scalar("SELECT count(*) FROM providers")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(providers, 4);
    // No permission/budget/key is seeded merely because installation exists.
    let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM grants")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(grants, 0);
    let other = Uuid::new_v4();
    assert!(sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id) VALUES ($1,$2,'test','test','safe',$3)").bind(Uuid::new_v4()).bind(installation).bind(other).execute(&store.pool).await.is_err());
    assert!(sqlx::query("INSERT INTO namespace_bindings(id,installation_id,registry_instance_id,namespace_id,tracker_instance_id,tracker_project_id,state,generation,observed_at,label,tracker_project_key) VALUES ($1,$2,$3,$4,$5,$6,'active',1,now(),'same-name','key')").bind(Uuid::new_v4()).bind(installation).bind(Uuid::nil()).bind(Uuid::new_v4()).bind(Uuid::new_v4()).bind(Uuid::new_v4()).execute(&store.pool).await.is_err());
    let project = Uuid::new_v4();
    let registry = Uuid::new_v4();
    let mut authorized = vec![];
    for index in 0..3 {
        let binding = Uuid::new_v4();
        let namespace = Uuid::new_v4();
        sqlx::query("INSERT INTO namespace_bindings(id,installation_id,registry_instance_id,namespace_id,tracker_instance_id,tracker_project_id,state,generation,observed_at,label,tracker_project_key) VALUES($1,$2,$3,$4,$5,$6,'active',1,now(),'same-name','key')")
            .bind(binding).bind(installation).bind(if index==2 {Uuid::new_v4()} else {registry}).bind(namespace).bind(Uuid::new_v4()).bind(project).execute(&store.pool).await.unwrap();
        if index < 2 {
            authorized.push((binding, namespace));
            sqlx::query("INSERT INTO grants(id,installation_id,principal_id,project_binding,action,bounds,expires_at,namespace_binding_id) VALUES($1,$2,'actor',$3,'metadata.read','{}',now()+interval '1 hour',$4)")
                .bind(Uuid::new_v4()).bind(installation).bind(project.to_string()).bind(binding).execute(&store.pool).await.unwrap();
        }
    }
    let first = store.namespace_page("actor", 1, None).await.unwrap();
    assert_eq!(first.items.len(), 1);
    let cursor = Uuid::parse_str(first.next_cursor.as_ref().unwrap()).unwrap();
    let second_id = authorized
        .iter()
        .find(|(_, n)| *n != first.items[0].namespace.namespace_id)
        .unwrap()
        .0;
    sqlx::query("UPDATE namespace_bindings SET label='changed' WHERE id=$1")
        .bind(second_id)
        .execute(&store.pool)
        .await
        .unwrap();
    let second = store
        .namespace_page("actor", 1, Some(cursor))
        .await
        .unwrap();
    assert_eq!(second.items.len(), 1);
    assert_eq!(
        second.items[0].label, "same-name",
        "snapshot freezes metadata"
    );
    assert!(second.next_cursor.is_none());
    assert_ne!(first.items[0].namespace, second.items[0].namespace);
    assert!(
        store
            .namespace_page("foreign", 1, Some(cursor))
            .await
            .is_err()
    );
    assert!(
        store
            .namespace_page("actor", 2, Some(cursor))
            .await
            .is_err()
    );
    assert!(
        store
            .audit_page("actor", None, 1, Some(cursor))
            .await
            .is_err()
    );
    sqlx::query("UPDATE grants SET revoked_at=now() WHERE namespace_binding_id=$1")
        .bind(second_id)
        .execute(&store.pool)
        .await
        .unwrap();
    assert!(
        store
            .namespace_page("actor", 1, Some(cursor))
            .await
            .is_err(),
        "cursor does not preserve revoked authority"
    );
}
