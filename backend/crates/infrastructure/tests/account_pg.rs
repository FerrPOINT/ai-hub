use aihub_application::{CentralAuthentication, Foundation, FoundationStore, MetadataOperations};
use aihub_domain::{
    access::{HumanAuthentication, HumanPrincipal},
    connections::{
        BillingMode, ConnectionInput, CredentialInput, CredentialType, EndpointPolicyInput,
        ProviderKind,
    },
    error::HubError,
    financial::Currency,
    pricing_sources::{PricingMode, PricingSourceInput},
};
use aihub_infrastructure::{
    account_statement::decode_openrouter_statement,
    metadata_refresh::{MetadataClaim, MetadataRefresh, MetadataStart},
    openrouter_metadata::decode_catalog,
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
        _ => panic!("fresh account claim required"),
    }
}
#[tokio::test]
#[ignore = "requires explicit isolated PostgreSQL17 fixture"]
async fn native_account_statement_quotes_replay_generation_and_atomic_audit() {
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
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations WHERE success")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        22,
        "fixture must execute the current migration set"
    );
    store.initialize("account-fixture").await.unwrap();
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
    let actor = HumanPrincipal {
        subject: "account-actor".into(),
        authentication: HumanAuthentication::BrowserSession,
    };
    let connection = foundation
        .save_connection(
            &actor,
            Uuid::new_v4(),
            ProviderKind::OpenaiCompatible,
            None,
            &ConnectionInput {
                display_name: "Fixture provider".into(),
                endpoint_policy_ref: "openrouter-fixture".into(),
                billing_mode: BillingMode::Metered,
            },
        )
        .await
        .unwrap()
        .value;
    foundation
        .write_credential(
            &actor,
            Uuid::new_v4(),
            connection.id,
            &CredentialInput {
                secret: Zeroizing::new("account-secret-canary".into()),
                credential_type: CredentialType::ApiKey,
                expected_generation: 1,
            },
        )
        .await
        .unwrap();
    let service = MetadataRefresh::new(store.clone(), vault.clone()).unwrap();
    sqlx::query("INSERT INTO grants(id,installation_id,principal_id,project_binding,action,bounds,expires_at) VALUES($1,$2,$3,'installation','metadata.read','{}',now()+interval '1 hour')").bind(Uuid::new_v4()).bind(installation).bind(&actor.subject).execute(&store.pool).await.unwrap();
    let tiny = aihub_domain::financial::Amount::parse("0.000000000000000001").unwrap();
    let budget = store
        .write_budget(
            &actor.subject,
            Uuid::new_v4(),
            [7; 32],
            None,
            &aihub_domain::budgets::BudgetInput {
                scope_type: aihub_domain::budgets::BudgetScope::Installation,
                scope_id: installation,
                currency: Currency::parse("USD").unwrap(),
                period: aihub_domain::budgets::BudgetPeriod::UtcDay,
                hard_limit: tiny,
                warning_thresholds: vec![80, 95],
                namespace: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(budget.value.policy.hard_limit, tiny);
    assert_eq!(
        service
            .account_authority(&actor, connection.id)
            .await
            .unwrap()
            .status,
        "unqualified"
    );
    let at = chrono::Utc::now();
    let statement=decode_openrouter_statement(br#"{"data":{"usage":123.000000000000000001,"is_management_key":false,"label":"secret-private-label"}}"#,at).unwrap();
    let catalog=decode_catalog(br#"{"data":[{"id":"vendor/CaseSensitive","context_length":8192,"pricing":{"prompt":"0.000002","completion":"0.000008","request":"0"}}]}"#,at).unwrap();
    let key = Uuid::new_v4();
    let current = claim(
        service
            .begin_account(&actor.subject, key, connection.id, 2)
            .await
            .unwrap(),
    );
    let lookup = store.operation_key(&actor.subject, key).await.unwrap();
    assert_eq!(lookup.action, "account.qualify");
    let done = service
        .complete_account(&current, &catalog, &statement)
        .await
        .unwrap();
    assert_eq!(done.status, "succeeded");
    let view = service
        .account_authority(&actor, connection.id)
        .await
        .unwrap();
    assert_eq!(view.status, "active");
    assert_eq!(view.currency.unwrap().to_string(), "USD");
    assert_eq!(
        view.statement_usage.unwrap().to_string(),
        "123.000000000000000001"
    );
    let connection = FoundationStore::read_connection(&*store, connection.id)
        .await
        .unwrap();
    assert_eq!(connection.status, "authorization_unknown");
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
        0,
        "account statements are not cash expenses"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM metadata_account_observations")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        0,
        "old observations are never promoted"
    );
    let prices: i64 =
        sqlx::query_scalar("SELECT count(*) FROM eligible_catalog_prices WHERE connection_id=$1")
            .bind(connection.id)
            .fetch_one(&store.pool)
            .await
            .unwrap();
    assert_eq!(prices, 1);
    let quotes: Vec<String> = sqlx::query_scalar(
        "SELECT input_uncached::text FROM price_revisions WHERE connection_id=$1",
    )
    .bind(connection.id)
    .fetch_all(&store.pool)
    .await
    .unwrap();
    assert_eq!(quotes, vec!["2.000000000000"]);
    store
        .create_pricing_source(
            &actor.subject,
            Uuid::new_v4(),
            [7; 32],
            &PricingSourceInput {
                connection_id: connection.id,
                model_id: "vendor/CaseSensitive".into(),
                currency: Currency::parse("USD").unwrap(),
                mode: PricingMode::ProviderAuto,
                manual_price_revision_id: None,
                expected_version: 0,
                effective_from: at,
                effective_to: None,
            },
        )
        .await
        .unwrap();
    assert!(
        store
            .resolve_pricing(
                connection.id,
                "vendor/CaseSensitive",
                &Currency::parse("USD").unwrap()
            )
            .await
            .unwrap()
            .price_revision_id
            .is_some()
    );
    match service
        .begin_account(&actor.subject, key, connection.id, 2)
        .await
        .unwrap()
    {
        MetadataStart::Readback(reply) => assert_eq!(reply.id, done.id),
        _ => panic!("no second I/O on original key"),
    };
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM verification_account_authorities")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        1
    );
    let missing = claim(
        service
            .begin(&actor.subject, Uuid::new_v4(), connection.id, 2)
            .await
            .unwrap(),
    );
    let empty = decode_catalog(br#"{"data":[]}"#, chrono::Utc::now()).unwrap();
    let metadata = aihub_infrastructure::openrouter_metadata::decode_account(
        br#"{"data":{"usage":123.000000000000000001,"is_management_key":false}}"#,
    )
    .unwrap();
    service.complete(&missing, &empty, &metadata).await.unwrap();
    assert!(
        store
            .resolve_pricing(
                connection.id,
                "vendor/CaseSensitive",
                &Currency::parse("USD").unwrap()
            )
            .await
            .unwrap()
            .price_revision_id
            .is_none(),
        "a removed model cannot reuse an old automatic quote"
    );
    let audit_failed = claim(
        service
            .begin_account(&actor.subject, Uuid::new_v4(), connection.id, 2)
            .await
            .unwrap(),
    );
    sqlx::raw_sql("CREATE FUNCTION reject_account_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='account.qualify.finish' THEN RAISE EXCEPTION 'fixture audit unavailable'; END IF; RETURN NEW; END; $$; CREATE TRIGGER reject_account_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_account_audit();").execute(&store.pool).await.unwrap();
    assert!(matches!(
        service
            .complete_account(&audit_failed, &catalog, &statement)
            .await,
        Err(HubError::Unavailable)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM verification_account_authorities WHERE state='active'"
        )
        .fetch_one(&store.pool)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM price_revisions")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        1
    );
    sqlx::raw_sql(
        "DROP TRIGGER reject_account_audit ON audit_events; DROP FUNCTION reject_account_audit();",
    )
    .execute(&store.pool)
    .await
    .unwrap();
    let obsolete = claim(
        service
            .begin_account(&actor.subject, Uuid::new_v4(), connection.id, 2)
            .await
            .unwrap(),
    );
    foundation
        .write_credential(
            &actor,
            Uuid::new_v4(),
            connection.id,
            &CredentialInput {
                secret: Zeroizing::new("new-account-secret-canary".into()),
                credential_type: CredentialType::ApiKey,
                expected_generation: 2,
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        service
            .complete_account(&obsolete, &catalog, &statement)
            .await,
        Err(HubError::PreconditionFailed)
    ));
    let view = service
        .account_authority(&actor, connection.id)
        .await
        .unwrap();
    assert_eq!(view.generation, 3);
    assert_eq!(view.status, "unqualified");
    assert!(view.currency.is_none());
    assert!(
        sqlx::query(
            "UPDATE verification_account_authorities SET statement_usage=0 WHERE connection_id=$1"
        )
        .bind(connection.id)
        .execute(&store.pool)
        .await
        .is_err()
    );
    let summaries: String =
        sqlx::query_scalar("SELECT coalesce(string_agg(safe_result::text,' '),'') FROM operations")
            .fetch_one(&store.pool)
            .await
            .unwrap();
    assert!(!summaries.contains("account-secret-canary"));
    assert!(!summaries.contains("secret-private-label"));
}
