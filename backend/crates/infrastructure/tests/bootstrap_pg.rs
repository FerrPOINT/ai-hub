use aihub_application::{CredentialProtection, FinancialAdmission, FoundationStore};
use aihub_domain::{
    admission::{AdmissionIntent, Purpose, PurposeBounds, RequestOwner},
    error::HubError,
    financial::{Acceptance, Amount, Currency, Usage},
    pricing_sources::{PricingMode, PricingSourceInput},
    replay::ReplayProtocol,
    settlement::{SettlementAuthority, SettlementFact, TerminalState},
};
use aihub_infrastructure::{postgres::PgStore, vault::Vault};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires explicit isolated PostgreSQL17 fixture"]
async fn first_probe_reserves_without_model_proof_and_keeps_unknown_fenced() {
    let installation = Uuid::new_v4();
    let vault = Vault::new(vec![7_u8; 32]).unwrap();
    let store = PgStore::connect(
        &std::env::var("AIHUB_TEST_DATABASE_URL").unwrap(),
        installation,
        vault.fingerprint(),
    )
    .await
    .unwrap();
    store.migrate().await.unwrap();
    store.initialize("bootstrap-fixture").await.unwrap();
    let provider: Uuid = sqlx::query_scalar(
        "SELECT id FROM providers WHERE installation_id=$1 AND kind='openai_compatible'",
    )
    .bind(installation)
    .fetch_one(&store.pool)
    .await
    .unwrap();
    let connection = Uuid::new_v4();
    let endpoint_hash = "1".repeat(64);
    sqlx::query("INSERT INTO connections(id,installation_id,provider_id,display_name,endpoint_policy_ref,billing_mode,status) VALUES($1,$2,$3,'Unverified model','controlled-fixture','metered','enabled')").bind(connection).bind(installation).bind(provider).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO connection_generations(connection_id,generation,authorization_state,adapter_revision,endpoint_policy_hash,billing_tier) VALUES($1,1,'prepared','synthetic-v1',$2,'metered')").bind(connection).bind(&endpoint_hash).execute(&store.pool).await.unwrap();
    let secret = vault
        .protect(installation, connection, 1, b"fixture-only-not-a-live-key")
        .unwrap();
    sqlx::query("INSERT INTO credential_versions(connection_id,generation,ciphertext,nonce,key_id,state) VALUES($1,1,$2,$3,$4,'prepared')").bind(connection).bind(secret.ciphertext).bind(secret.nonce.as_slice()).bind(secret.key_id).execute(&store.pool).await.unwrap();
    let authority = Uuid::new_v4();
    let auth_op = Uuid::new_v4();
    sqlx::query("INSERT INTO operations(id,installation_id,principal_kind,principal_id,idempotency_key,binding_hmac,action,state,expires_at) VALUES($1,$2,'internal','fixture',$1,$3,'account.qualify','succeeded',now()+interval '30 days')").bind(auth_op).bind(installation).bind(vec![7_u8;32]).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO verification_account_authorities(id,installation_id,connection_id,generation,operation_id,adapter_revision,endpoint_policy_hash,currency,billing_tier,statement_origin,statement_digest,statement_usage,billing_capabilities,observed_at,expires_at,state) VALUES($1,$2,$3,1,$4,'synthetic-v1',$5,'USD','metered','synthetic-statement',$6,0,'{}',now(),now()+interval '1 hour','active')").bind(authority).bind(installation).bind(connection).bind(auth_op).bind(&endpoint_hash).bind("2".repeat(64)).execute(&store.pool).await.unwrap();
    let price = Uuid::new_v4();
    sqlx::query("INSERT INTO price_revisions(id,installation_id,connection_id,model_id,tier,currency,input_uncached,input_cached,output_billable,request_fee,effective_from,source) VALUES($1,$2,$3,'model-new','metered','USD',2,0.5,8,0,now()-interval '1 minute','synthetic-fixture')").bind(price).bind(installation).bind(connection).execute(&store.pool).await.unwrap();
    let usd = Currency::parse("USD").unwrap();
    store
        .create_pricing_source(
            "fixture",
            Uuid::new_v4(),
            [7; 32],
            &PricingSourceInput {
                connection_id: connection,
                model_id: "model-new".into(),
                currency: usd.clone(),
                mode: PricingMode::Manual,
                manual_price_revision_id: Some(price),
                expected_version: 0,
                effective_from: chrono::Utc::now() - chrono::Duration::seconds(30),
                effective_to: None,
            },
        )
        .await
        .unwrap();
    let source = store
        .resolve_pricing(connection, "model-new", &usd)
        .await
        .unwrap();
    sqlx::query("UPDATE connections SET status='authorization_unknown' WHERE id=$1")
        .bind(connection)
        .execute(&store.pool)
        .await
        .unwrap();
    let client = Uuid::new_v4();
    let grant = Uuid::new_v4();
    let probe = Uuid::new_v4();
    let op = Uuid::new_v4();
    sqlx::query("INSERT INTO clients(id,installation_id,application_key,project_binding,allowed_profiles,scopes,cost_policy,expires_at,max_concurrency,max_rpm,status,version) VALUES($1,$2,$3,'unbound','[]','[\"infer\"]','budget_guaranteed',now()+interval '1 hour',1,10,'enabled',1)").bind(client).bind(installation).bind(client.to_string()).execute(&store.pool).await.unwrap();
    let bounds = PurposeBounds {
        connection_id: connection,
        generation: 1,
        model_id: "model-new".into(),
        currency: usd,
        max_input_tokens: 100,
        max_output_tokens: 50,
        max_requests: 5,
        max_concurrency: 1,
        max_total_provider_cost: Some(Amount::parse("0.1").unwrap()),
        cost_unknown_allowed: false,
    };
    sqlx::query("INSERT INTO grants(id,installation_id,principal_id,client_id,project_binding,action,bounds,expires_at) VALUES($1,$2,'fixture',$3,'unbound','verification',$4,now()+interval '1 hour')").bind(grant).bind(installation).bind(client).bind(serde_json::to_value(&bounds).unwrap()).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO operations(id,installation_id,principal_kind,principal_id,idempotency_key,binding_hmac,action,state,expires_at) VALUES($1,$2,'internal','fixture',$1,$3,'verify','pending',now()+interval '30 days')").bind(op).bind(installation).bind(vec![7_u8;32]).execute(&store.pool).await.unwrap();
    let usage = Usage::normalized(100, 0, 50, "bounded-synthetic-fixture".into()).unwrap();
    let target = serde_json::json!({"connection_id":connection,"generation":1,"model_id":"model-new","tier":"metered","price_revision_id":price,"qualification_id":null,"account_authority_id":authority,"upper_usage":usage,"protocol":"chat_completions","streaming":false,"intent_ttl_seconds":120,"pricing_source_revision_id":source.source_revision_id,"pricing_policy_version":source.policy_version});
    sqlx::query("INSERT INTO probe_snapshots(id,installation_id,operation_id,scope,config_hash,configuration) VALUES($1,$2,$3,'connection_model',$4,$5)").bind(probe).bind(installation).bind(op).bind("3".repeat(64)).bind(serde_json::json!({"targets":[target]})).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO budget_policies(id,installation_id,scope_type,scope_id,currency,period,hard_limit,thresholds,status) VALUES($1,$2,'installation',$3,'USD','utc_day',0.0012,'[80,95]','active')").bind(Uuid::new_v4()).bind(installation).bind(installation.to_string()).execute(&store.pool).await.unwrap();
    let intent = AdmissionIntent {
        pricing_source_revision_id: source.source_revision_id,
        pricing_policy_version: source.policy_version,
        intent_ttl_seconds: 120,
        protocol: ReplayProtocol::ChatCompletions,
        streaming: false,
        client_id: client,
        grant_id: grant,
        principal_id: "fixture".into(),
        purpose: Purpose::Verification,
        profile_revision_id: None,
        probe_snapshot_id: Some(probe),
        idempotency_key: Uuid::new_v4(),
        payload_hmac: [7; 32],
        connection_id: connection,
        generation: 1,
        model_id: "model-new".into(),
        tier: "metered".into(),
        price_revision_id: Some(price),
        qualification_id: None,
        account_authority_id: Some(authority),
        upper_usage: usage.clone(),
    };
    let receipt = store.reserve(&intent).await.expect(
        "the first model probe must reserve against its own account statement without model proof",
    );
    assert_eq!(
        receipt.upper_provider_cost.unwrap(),
        Amount::parse("0.0006").unwrap()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM runtime_qualifications")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        0,
        "never fabricate model qualification to break the cycle"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT reserved::text FROM grant_accounts WHERE grant_id=$1"
        )
        .bind(grant)
        .fetch_one(&store.pool)
        .await
        .unwrap(),
        "0.000600000000000000"
    );
    let claim = store
        .claim_dispatch(receipt.attempt_id, Uuid::new_v4(), 30)
        .await
        .unwrap();
    assert_eq!(
        claim.deployment_snapshot["target"]["account_authority_id"],
        serde_json::json!(authority)
    );
    store.record_uncertain(&claim).await.unwrap();
    assert!(matches!(
        store
            .claim_dispatch(receipt.attempt_id, Uuid::new_v4(), 30)
            .await,
        Err(HubError::PreconditionFailed)
    ));
    assert_eq!(
        store.reserve(&intent).await.unwrap().attempt_id,
        receipt.attempt_id,
        "readback never dispatches a second paid call"
    );
    let mut other = intent.clone();
    other.idempotency_key = Uuid::new_v4();
    assert!(
        matches!(store.reserve(&other).await, Err(HubError::BudgetExceeded)),
        "unknown retains the concurrency slot"
    );
    let settled = store
        .settle(&SettlementFact {
            attempt_id: receipt.attempt_id,
            authority: SettlementAuthority::Dispatch {
                owner_id: claim.owner_id,
                fence: claim.fence,
            },
            source: "synthetic-v1".into(),
            source_event_id: Uuid::new_v4().to_string(),
            acceptance: Acceptance::Accepted,
            terminal: TerminalState::Completed,
            usage: Some(usage),
            receipt: None,
        })
        .await
        .unwrap();
    assert_eq!(settled.amount.unwrap(), Amount::parse("0.0006").unwrap());
    assert_eq!(settled.confidence, "estimated");
    let mut unauthorized = intent.clone();
    unauthorized.purpose = Purpose::Inference;
    unauthorized.idempotency_key = Uuid::new_v4();
    assert!(matches!(
        store.reserve(&unauthorized).await,
        Err(HubError::Unavailable) | Err(HubError::Forbidden)
    ));
    unauthorized.purpose = Purpose::Verification;
    unauthorized.account_authority_id = Some(Uuid::new_v4());
    assert!(store.reserve(&unauthorized).await.is_err());
    let queued = store.reserve(&other).await.unwrap();
    sqlx::query("UPDATE verification_account_authorities SET state='invalidated' WHERE id=$1")
        .bind(authority)
        .execute(&store.pool)
        .await
        .unwrap();
    assert!(matches!(
        store
            .claim_dispatch(queued.attempt_id, Uuid::new_v4(), 30)
            .await,
        Err(HubError::PreconditionFailed)
    ));
    store
        .cancel_owned(
            &RequestOwner {
                client_id: client,
                principal_id: "fixture".into(),
            },
            queued.request_id,
        )
        .await
        .unwrap();
    assert!(
        sqlx::query("UPDATE verification_account_authorities SET currency='EUR' WHERE id=$1")
            .bind(authority)
            .execute(&store.pool)
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM connections WHERE id=$1")
            .bind(connection)
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        "authorization_unknown"
    );
}
