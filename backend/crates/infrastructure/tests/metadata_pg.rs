use aihub_application::{CentralAuthentication, Foundation, FoundationStore};
use aihub_domain::{
    access::{HumanAuthentication, HumanPrincipal},
    connections::{
        BillingMode, ConnectionInput, CredentialInput, CredentialType, EndpointPolicyInput,
        ProviderKind,
    },
    error::HubError,
};
use aihub_infrastructure::{
    metadata_refresh::{MetadataClaim, MetadataRefresh, MetadataStart},
    openrouter_metadata::{decode_account, decode_catalog},
    postgres::PgStore,
    vault::Vault,
};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;
use zeroize::Zeroizing;
struct UnusedAuth;
#[async_trait]
impl CentralAuthentication for UnusedAuth {
    async fn authenticate(&self, _: &str) -> Result<HumanPrincipal, HubError> {
        Err(HubError::Unauthenticated)
    }
}
fn claim(start: MetadataStart) -> MetadataClaim {
    match start {
        MetadataStart::Claim(c) => c,
        _ => panic!("expected fresh claim"),
    }
}
fn status(start: MetadataStart) -> String {
    match start {
        MetadataStart::Readback(o) => o.status,
        _ => panic!("duplicate must not obtain a dispatch claim"),
    }
}
#[tokio::test]
#[ignore = "requires explicit isolated PostgreSQL17 fixture"]
async fn durable_metadata_single_claim_generation_fence_atomic_audit_and_recovery() {
    let installation = Uuid::new_v4();
    let vault = Arc::new(Vault::new(vec![7; 32]).unwrap());
    let store = Arc::new(
        PgStore::connect(
            &std::env::var("AIHUB_TEST_DATABASE_URL").unwrap(),
            installation,
            vault.fingerprint(),
        )
        .await
        .unwrap(),
    );
    store.migrate().await.unwrap();
    store.initialize("metadata-fixture").await.unwrap();
    store
        .configure_endpoints(&[EndpointPolicyInput {
            policy_ref: "openrouter-fixture".into(),
            provider_kind: ProviderKind::OpenaiCompatible,
            base_url: "https://openrouter.ai/api/v1/".into(),
            allow_loopback: false,
        }])
        .await
        .unwrap();
    let foundation = Foundation {
        installation_id: installation,
        auth: Arc::new(UnusedAuth),
        store: store.clone(),
        bindings: vault.clone(),
        secrets: vault.clone(),
    };
    let principal = HumanPrincipal {
        subject: "metadata-actor".into(),
        authentication: HumanAuthentication::BrowserSession,
    };
    let connection = foundation
        .save_connection(
            &principal,
            Uuid::new_v4(),
            ProviderKind::OpenaiCompatible,
            None,
            &ConnectionInput {
                display_name: "metadata fixture".into(),
                endpoint_policy_ref: "openrouter-fixture".into(),
                billing_mode: BillingMode::Metered,
            },
        )
        .await
        .unwrap()
        .value;
    foundation
        .write_credential(
            &principal,
            Uuid::new_v4(),
            connection.id,
            &CredentialInput {
                secret: Zeroizing::new("metadata-secret-canary".into()),
                credential_type: CredentialType::ApiKey,
                expected_generation: 1,
            },
        )
        .await
        .unwrap();
    let service = MetadataRefresh::new(store.clone(), vault.clone()).unwrap();
    assert!(
        FoundationStore::read_connection(&*store, connection.id)
            .await
            .unwrap()
            .catalog_refresh_supported
    );
    let key = Uuid::new_v4();
    let (one, two) = tokio::join!(
        service.begin(&principal.subject, key, connection.id, 2),
        service.begin(&principal.subject, key, connection.id, 2)
    );
    let (fresh, replay) = match (one.unwrap(), two.unwrap()) {
        (MetadataStart::Claim(c), MetadataStart::Readback(o))
        | (MetadataStart::Readback(o), MetadataStart::Claim(c)) => (c, o),
        _ => panic!("only one claim is permitted"),
    };
    assert_eq!(fresh.operation.id, replay.id);
    assert_eq!(replay.status, "pending");
    assert!(matches!(
        service
            .begin(&principal.subject, key, connection.id, 3)
            .await,
        Err(HubError::IdempotencyConflict)
    ));
    let catalog = decode_catalog(
        br#"{"data":[{"id":"qualified-name-is-still-unverified","context_length":4096}]}"#,
        chrono::Utc::now(),
    )
    .unwrap();
    let account=decode_account(br#"{"data":{"label":"private-provider-label-canary","creator_user_id":"private-account-id-canary","limit":10.000000000000000001,"limit_remaining":9,"usage":1,"is_management_key":false,"is_free_tier":false}}"#).unwrap();
    sqlx::raw_sql("CREATE FUNCTION reject_metadata_finish() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='catalog.refresh.finish' THEN RAISE EXCEPTION 'fixture audit unavailable'; END IF; RETURN NEW; END; $$; CREATE TRIGGER reject_metadata_finish BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_metadata_finish();").execute(&store.pool).await.unwrap();
    assert!(service.complete(&fresh, &catalog, &account).await.is_err());
    assert_eq!(
        status(
            service
                .begin(&principal.subject, key, connection.id, 2)
                .await
                .unwrap()
        ),
        "pending"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM catalog_snapshots")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        0,
        "audit failure rolls back catalog and pointer"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM metadata_account_observations")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        0
    );
    sqlx::raw_sql("DROP TRIGGER reject_metadata_finish ON audit_events; DROP FUNCTION reject_metadata_finish();").execute(&store.pool).await.unwrap();
    assert_eq!(
        service
            .complete(&fresh, &catalog, &account)
            .await
            .unwrap()
            .status,
        "succeeded"
    );
    assert!(service.complete(&fresh, &catalog, &account).await.is_err());
    assert_eq!(
        status(
            service
                .begin(&principal.subject, key, connection.id, 2)
                .await
                .unwrap()
        ),
        "succeeded"
    );
    let safe:String=sqlx::query_scalar("SELECT (SELECT jsonb_agg(to_jsonb(o))::text FROM operations o)||(SELECT jsonb_agg(to_jsonb(a))::text FROM audit_events a)||(SELECT jsonb_agg(to_jsonb(m))::text FROM metadata_account_observations m)").fetch_one(&store.pool).await.unwrap();
    for value in [
        "metadata-secret-canary",
        "private-provider-label-canary",
        "private-account-id-canary",
    ] {
        assert!(!safe.contains(value));
    }
    assert!(safe.contains("10.000000000000000001"));
    assert!(safe.contains("unqualified"));
    assert_eq!(
        FoundationStore::read_connection(&*store, connection.id)
            .await
            .unwrap()
            .status,
        "authorization_unknown",
        "metadata is not inference qualification"
    );
    let stale = claim(
        service
            .begin(&principal.subject, Uuid::new_v4(), connection.id, 2)
            .await
            .unwrap(),
    );
    foundation
        .write_credential(
            &principal,
            Uuid::new_v4(),
            connection.id,
            &CredentialInput {
                secret: Zeroizing::new("second-key-canary".into()),
                credential_type: CredentialType::ApiKey,
                expected_generation: 2,
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        service.complete(&stale, &catalog, &account).await,
        Err(HubError::PreconditionFailed)
    ));
    assert_eq!(service.fail(&stale, false).await.unwrap().status, "failed");
    assert_eq!(
        FoundationStore::catalog_page(&*store, &principal.subject, connection.id, "", 100, None)
            .await
            .unwrap()
            .models
            .len(),
        0
    );
    // Only this fixture changes the INSERT clock; production lease identity remains immutable.
    sqlx::raw_sql("CREATE FUNCTION expire_metadata_fixture() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.lease_until=clock_timestamp()-interval '1 second'; RETURN NEW; END; $$; CREATE TRIGGER expire_metadata_fixture BEFORE INSERT ON metadata_refreshes FOR EACH ROW EXECUTE FUNCTION expire_metadata_fixture();").execute(&store.pool).await.unwrap();
    let expire_key = Uuid::new_v4();
    let expired = claim(
        service
            .begin(&principal.subject, expire_key, connection.id, 3)
            .await
            .unwrap(),
    );
    sqlx::raw_sql("DROP TRIGGER expire_metadata_fixture ON metadata_refreshes; DROP FUNCTION expire_metadata_fixture();").execute(&store.pool).await.unwrap();
    assert!(matches!(
        service.complete(&expired, &catalog, &account).await,
        Err(HubError::PreconditionFailed)
    ));
    assert_eq!(store.recover_expired_metadata().await.unwrap(), 1);
    assert_eq!(store.recover_expired_metadata().await.unwrap(), 0);
    assert_eq!(
        status(
            service
                .begin(&principal.subject, expire_key, connection.id, 3)
                .await
                .unwrap()
        ),
        "unknown"
    );
    let recovered = foundation
        .operation(&principal, expired.operation.id)
        .await
        .unwrap();
    assert_eq!(recovered.status, "unknown");
    assert!(recovered.safe_error.is_some());
    let wrong_snapshot: Uuid =
        sqlx::query_scalar("SELECT id FROM catalog_snapshots ORDER BY created_at LIMIT 1")
            .fetch_one(&store.pool)
            .await
            .unwrap();
    assert!(sqlx::query("INSERT INTO metadata_account_observations(operation_id,catalog_snapshot_id,source_digest,summary,observed_at) VALUES($1,$2,$3,'{\"currency_status\":\"unqualified\"}',now())").bind(expired.operation.id).bind(wrong_snapshot).bind("a".repeat(64)).execute(&store.pool).await.is_err(),"catalog generation cannot cross refresh generation");
    assert!(
        service
            .complete(&expired, &catalog, &account)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE metadata_refreshes SET fence=$1 WHERE operation_id=$2")
            .bind(Uuid::new_v4())
            .bind(expired.operation.id)
            .execute(&store.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM metadata_account_observations")
            .execute(&store.pool)
            .await
            .is_err()
    );
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
