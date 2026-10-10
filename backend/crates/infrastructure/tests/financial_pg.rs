use aihub_application::{FinancialAdmission, FoundationStore, ResultDelivery};
use aihub_domain::replay::{ReplayOutcome, ReplayProtocol, ResultPayload, ResultReader};
use aihub_domain::{
    admission::{AdmissionIntent, Purpose, PurposeBounds, RequestOwner},
    error::HubError,
    financial::{Amount, Currency, Usage},
};
use aihub_infrastructure::{postgres::PgStore, replay::ProtectedResults, vault::Vault};
use std::sync::Arc;
use uuid::Uuid;
use zeroize::Zeroizing;

async fn fixture_intent(
    store: &PgStore,
    connection: Uuid,
    price: Uuid,
    qualification: Uuid,
) -> AdmissionIntent {
    fixture_intent_ttl(store, connection, price, qualification, 120).await
}
async fn fixture_intent_ttl(
    store: &PgStore,
    connection: Uuid,
    price: Uuid,
    qualification: Uuid,
    intent_ttl_seconds: i32,
) -> AdmissionIntent {
    fixture_context(
        store,
        connection,
        Some(price),
        qualification,
        intent_ttl_seconds,
    )
    .await
}
async fn fixture_context(
    store: &PgStore,
    connection: Uuid,
    price: Option<Uuid>,
    qualification: Uuid,
    intent_ttl_seconds: i32,
) -> AdmissionIntent {
    let usd = Currency::parse("USD").unwrap();
    let mut source = store
        .resolve_pricing(connection, "model-a", &usd)
        .await
        .unwrap();
    if let Some(price) = price {
        if source.price_revision_id != Some(price) {
            let input = aihub_domain::pricing_sources::PricingSourceInput {
                connection_id: connection,
                model_id: "model-a".into(),
                currency: usd.clone(),
                mode: aihub_domain::pricing_sources::PricingMode::Manual,
                manual_price_revision_id: Some(price),
                expected_version: source.policy_version,
                effective_from: chrono::Utc::now() - chrono::Duration::milliseconds(1),
                effective_to: None,
            };
            use aihub_application::OperationBinding;
            let vault = Vault::new(vec![7; 32]).unwrap();
            store
                .create_pricing_source(
                    "fixture-worker",
                    Uuid::new_v4(),
                    vault.bind(&serde_json::to_value(&input).unwrap()).unwrap(),
                    &input,
                )
                .await
                .unwrap();
            source = store
                .resolve_pricing(connection, "model-a", &usd)
                .await
                .unwrap();
        }
    }
    let client = Uuid::new_v4();
    let grant = Uuid::new_v4();
    let probe = Uuid::new_v4();
    let op = Uuid::new_v4();
    sqlx::query("INSERT INTO clients(id,installation_id,application_key,project_binding,allowed_profiles,scopes,cost_policy,expires_at,max_concurrency,max_rpm,status,version) VALUES($1,$2,$3,'installation','[]','[\"infer\"]','budget_guaranteed',now()+interval '1 hour',2,100,'enabled',1)")
        .bind(client).bind(store.installation_id).bind(client.to_string()).execute(&store.pool).await.unwrap();
    let bounds = PurposeBounds {
        connection_id: connection,
        generation: 1,
        model_id: "model-a".into(),
        currency: Currency::parse("USD").unwrap(),
        max_input_tokens: 1000,
        max_output_tokens: 100,
        max_requests: 5,
        max_concurrency: 2,
        max_total_provider_cost: Some(Amount::parse("0.10").unwrap()),
        cost_unknown_allowed: false,
    };
    sqlx::query("INSERT INTO grants(id,installation_id,principal_id,client_id,project_binding,action,bounds,expires_at) VALUES($1,$2,'fixture-worker',$3,'installation','verification',$4,now()+interval '1 hour')")
        .bind(grant).bind(store.installation_id).bind(client).bind(serde_json::to_value(bounds).unwrap()).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO operations(id,installation_id,principal_kind,principal_id,idempotency_key,binding_hmac,action,state,expires_at) VALUES($1,$2,'internal','fixture-worker',$1,$3,'verify','pending',now()+interval '30 days')").bind(op).bind(store.installation_id).bind(vec![7_u8;32]).execute(&store.pool).await.unwrap();
    let usage = Usage::normalized(100, 0, 50, "qualified-synthetic-fixture".into()).unwrap();
    let target = serde_json::json!({"connection_id":connection,"generation":1,"model_id":"model-a","tier":"metered","price_revision_id":price,"qualification_id":qualification,"upper_usage":usage,"protocol":"chat_completions","streaming":false,"intent_ttl_seconds":intent_ttl_seconds,"pricing_source_revision_id":source.source_revision_id,"pricing_policy_version":source.policy_version});
    sqlx::query("INSERT INTO probe_snapshots(id,installation_id,operation_id,scope,config_hash,configuration) VALUES($1,$2,$3,'connection_model',$4,$5)").bind(probe).bind(store.installation_id).bind(op).bind("1".repeat(64)).bind(serde_json::json!({"targets":[target]})).execute(&store.pool).await.unwrap();
    AdmissionIntent {
        pricing_source_revision_id: source.source_revision_id,
        pricing_policy_version: source.policy_version,
        intent_ttl_seconds,
        protocol: aihub_domain::replay::ReplayProtocol::ChatCompletions,
        streaming: false,
        client_id: client,
        grant_id: grant,
        principal_id: "fixture-worker".into(),
        purpose: Purpose::Verification,
        profile_revision_id: None,
        probe_snapshot_id: Some(probe),
        idempotency_key: Uuid::new_v4(),
        payload_hmac: [7; 32],
        connection_id: connection,
        generation: 1,
        model_id: "model-a".into(),
        tier: "metered".into(),
        price_revision_id: price,
        qualification_id: qualification,
        upper_usage: usage,
    }
}

/// Real SQL concurrency and restart readback; currency qualification is synthetic, not live.
#[tokio::test]
#[ignore = "requires explicit isolated PostgreSQL17 fixture"]
async fn durable_budget_admission_concurrency_replay_and_unknown_hold() {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    let dsn = std::env::var("AIHUB_TEST_DATABASE_URL").expect("explicit isolated fixture DSN");
    let installation = Uuid::new_v4();
    let vault = Arc::new(Vault::new(vec![7; 32]).unwrap());
    let store = PgStore::connect(&dsn, installation, vault.fingerprint())
        .await
        .unwrap();
    store.migrate().await.unwrap();
    store.initialize("financial-fixture").await.unwrap();
    store.ready().await.unwrap();
    let provider: Uuid = sqlx::query_scalar(
        "SELECT id FROM providers WHERE installation_id=$1 AND kind='openai_compatible'",
    )
    .bind(installation)
    .fetch_one(&store.pool)
    .await
    .unwrap();
    let connection = Uuid::new_v4();
    sqlx::query("INSERT INTO connections(id,installation_id,provider_id,display_name,endpoint_policy_ref,billing_mode,status) VALUES($1,$2,$3,'synthetic-fixture','controlled-http','metered','enabled')").bind(connection).bind(installation).bind(provider).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO connection_generations(connection_id,generation,authorization_state,adapter_revision,endpoint_policy_hash,billing_tier) VALUES($1,1,'active','synthetic-v1',$2,'metered')").bind(connection).bind("1".repeat(64)).execute(&store.pool).await.unwrap();
    let price = Uuid::new_v4();
    sqlx::query("INSERT INTO price_revisions(id,installation_id,connection_id,model_id,tier,currency,input_uncached,input_cached,output_billable,request_fee,effective_from,source) VALUES($1,$2,$3,'model-a','metered','USD',2,0.5,8,0,now()-interval '1 minute','synthetic-fixture')").bind(price).bind(installation).bind(connection).execute(&store.pool).await.unwrap();
    let account_op = Uuid::new_v4();
    let account_probe = Uuid::new_v4();
    let evidence = Uuid::new_v4();
    let qualification = Uuid::new_v4();
    sqlx::query("INSERT INTO operations(id,installation_id,principal_kind,principal_id,idempotency_key,binding_hmac,action,state,expires_at) VALUES($1,$2,'internal','fixture-worker',$1,$3,'account-preflight','succeeded',now()+interval '30 days')").bind(account_op).bind(installation).bind(vec![7_u8;32]).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO probe_snapshots(id,installation_id,operation_id,scope,config_hash,configuration) VALUES($1,$2,$3,'connection_model',$4,'{}')").bind(account_probe).bind(installation).bind(account_op).bind("1".repeat(64)).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO verification_evidence(id,installation_id,probe_snapshot_id,state,child_evidence,receipt_digest,expires_at) VALUES($1,$2,$3,'verified','[]',$4,now()+interval '1 hour')").bind(evidence).bind(installation).bind(account_probe).bind("2".repeat(64)).execute(&store.pool).await.unwrap();
    sqlx::query("INSERT INTO runtime_qualifications(id,installation_id,connection_id,generation,provider_model_id,adapter_revision,endpoint_policy_hash,proof_id,capabilities,state,billing_currency,billing_currency_origin) VALUES($1,$2,$3,1,'model-a','synthetic-v1',$4,$5,'{\"trusted_charge_receipts\":true}','active','USD','synthetic-statement')").bind(qualification).bind(installation).bind(connection).bind("1".repeat(64)).bind(evidence).execute(&store.pool).await.unwrap();
    let budget = Uuid::new_v4();
    sqlx::query("INSERT INTO budget_policies(id,installation_id,scope_type,scope_id,currency,period,hard_limit,thresholds,status) VALUES($1,$2,'installation',$3,'USD','utc_day',0.0006,'[80,95]','active')").bind(budget).bind(installation).bind(installation.to_string()).execute(&store.pool).await.unwrap();
    // Legitimate unconfigured source before a policy exists; a hard cap still rejects unknown cost.
    let no_price = fixture_context(&store, connection, None, qualification, 120).await;
    assert!(matches!(
        store.reserve(&no_price).await,
        Err(HubError::BudgetExceeded)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM pricing_source_policies")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        0
    );
    let one = fixture_intent(&store, connection, price, qualification).await;
    let two = fixture_intent(&store, connection, price, qualification).await;
    for bad in [
        AdmissionIntent {
            grant_id: two.grant_id,
            ..one.clone()
        },
        AdmissionIntent {
            generation: 2,
            ..one.clone()
        },
        AdmissionIntent {
            model_id: "another-model".into(),
            ..one.clone()
        },
    ] {
        assert!(matches!(
            store.reserve(&bad).await,
            Err(HubError::Forbidden)
        ));
    }
    sqlx::raw_sql("CREATE FUNCTION reject_fixture_reserve_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='request.reserve' THEN RAISE EXCEPTION 'fixture audit unavailable'; END IF; RETURN NEW; END; $$; CREATE TRIGGER reject_fixture_reserve_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_fixture_reserve_audit();").execute(&store.pool).await.unwrap();
    assert!(store.reserve(&one).await.is_err());
    for table in [
        "requests",
        "attempts",
        "reservations",
        "ledger_entries",
        "budget_periods",
        "grant_accounts",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&store.pool)
            .await
            .unwrap();
        assert_eq!(
            count, 0,
            "partial financial admission must roll back {table}"
        );
    }
    sqlx::raw_sql("DROP TRIGGER reject_fixture_reserve_audit ON audit_events; DROP FUNCTION reject_fixture_reserve_audit();").execute(&store.pool).await.unwrap();
    let second_store = PgStore::connect(&dsn, installation, vault.fingerprint())
        .await
        .unwrap();
    let (first, second) = tokio::join!(store.reserve(&one), second_store.reserve(&two));
    assert_eq!(
        usize::from(first.is_ok()) + usize::from(second.is_ok()),
        1,
        "only one durable admission may consume the last budget"
    );
    let (winner, loser, receipt) = if let Ok(receipt) = first {
        (&one, &two, receipt)
    } else {
        (&two, &one, second.unwrap())
    };
    assert_eq!(
        receipt.upper_provider_cost.unwrap().to_string(),
        "0.000600000000000000"
    );
    assert!(matches!(
        store.reserve(loser).await,
        Err(HubError::BudgetExceeded)
    ));
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM requests),(SELECT count(*) FROM attempts),(SELECT count(*) FROM reservations)").fetch_one(&store.pool).await.unwrap();
    assert_eq!(counts, (1, 1, 1));
    let totals: (String, String) = sqlx::query_as(
        "SELECT charged::text,reserved::text FROM budget_periods WHERE policy_id=$1",
    )
    .bind(budget)
    .fetch_one(&store.pool)
    .await
    .unwrap();
    assert_eq!(
        totals,
        ("0.000000000000000000".into(), "0.000600000000000000".into())
    );
    let replay = store.reserve(winner).await.unwrap();
    assert!(replay.replay);
    assert_eq!(replay.request_id, receipt.request_id);
    assert_eq!(replay.attempt_id, receipt.attempt_id);
    assert!(matches!(
        store
            .reserve(&AdmissionIntent {
                streaming: true,
                ..winner.clone()
            })
            .await,
        Err(HubError::IdempotencyConflict)
    ));
    assert!(
        sqlx::query("UPDATE requests SET streaming=true WHERE id=$1")
            .bind(receipt.request_id)
            .execute(&store.pool)
            .await
            .is_err()
    );
    let changed = AdmissionIntent {
        payload_hmac: [8; 32],
        ..fixture_intent(&store, connection, price, qualification).await
    };
    let changed = AdmissionIntent {
        client_id: winner.client_id,
        grant_id: winner.grant_id,
        idempotency_key: winner.idempotency_key,
        ..changed
    };
    assert!(matches!(
        store.reserve(&changed).await,
        Err(HubError::IdempotencyConflict)
    ));
    // Fresh authority is required between admission and dispatch.
    sqlx::query("UPDATE grants SET revoked_at=now() WHERE id=$1")
        .bind(winner.grant_id)
        .execute(&store.pool)
        .await
        .unwrap();
    assert!(matches!(
        store
            .claim_dispatch(receipt.attempt_id, Uuid::new_v4(), 5)
            .await,
        Err(HubError::Forbidden)
    ));
    sqlx::query("UPDATE grants SET revoked_at=NULL WHERE id=$1")
        .bind(winner.grant_id)
        .execute(&store.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE connections SET generation=2 WHERE id=$1")
        .bind(connection)
        .execute(&store.pool)
        .await
        .unwrap();
    assert!(matches!(
        store
            .claim_dispatch(receipt.attempt_id, Uuid::new_v4(), 5)
            .await,
        Err(HubError::PreconditionFailed)
    ));
    sqlx::query("UPDATE connections SET generation=1 WHERE id=$1")
        .bind(connection)
        .execute(&store.pool)
        .await
        .unwrap();
    let (claim_one, claim_two) = tokio::join!(
        store.claim_dispatch(receipt.attempt_id, Uuid::new_v4(), 5),
        second_store.claim_dispatch(receipt.attempt_id, Uuid::new_v4(), 5)
    );
    assert_eq!(
        usize::from(claim_one.is_ok()) + usize::from(claim_two.is_ok()),
        1,
        "two instances cannot both send: {claim_one:?}, {claim_two:?}"
    );
    let claim = claim_one.or(claim_two).unwrap();
    let forged = aihub_domain::admission::DispatchClaim {
        fence: Uuid::new_v4(),
        attempt_id: claim.attempt_id,
        owner_id: claim.owner_id,
        deployment_snapshot: claim.deployment_snapshot.clone(),
    };
    assert!(matches!(
        store.record_uncertain(&forged).await,
        Err(HubError::PreconditionFailed)
    ));
    // A dead sender expires. Restart uses status recovery, never another send claim.
    sqlx::query("SELECT pg_sleep(5.1)")
        .execute(&store.pool)
        .await
        .unwrap();
    let restarted = PgStore::connect(&dsn, installation, vault.fingerprint())
        .await
        .unwrap();
    restarted.ready().await.unwrap();
    assert_eq!(restarted.recover_expired_dispatches().await.unwrap(), 1);
    assert_eq!(restarted.recover_expired_dispatches().await.unwrap(), 0);
    assert!(matches!(
        restarted
            .claim_dispatch(receipt.attempt_id, Uuid::new_v4(), 5)
            .await,
        Err(HubError::PreconditionFailed)
    ));
    restarted.record_uncertain(&claim).await.unwrap();
    let recovery = restarted.reserve(winner).await.unwrap();
    assert!(recovery.replay);
    assert_eq!(recovery.state, "unknown");
    assert_eq!(recovery.attempt_id, receipt.attempt_id);
    let held: String = sqlx::query_scalar("SELECT state FROM reservations WHERE attempt_id=$1")
        .bind(receipt.attempt_id)
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(held, "held");
    use aihub_domain::{
        financial::Acceptance,
        settlement::{ProviderCharge, SettlementAuthority, SettlementFact, TerminalState},
    };
    let pending = SettlementFact {
        attempt_id: claim.attempt_id,
        authority: SettlementAuthority::Dispatch {
            owner_id: claim.owner_id,
            fence: claim.fence,
        },
        source: "synthetic-v1".into(),
        source_event_id: "pending-event".into(),
        acceptance: Acceptance::Unknown,
        terminal: TerminalState::Failed,
        usage: None,
        receipt: None,
    };
    let delivery = ProtectedResults::new(
        Arc::new(
            PgStore::connect(&dsn, installation, vault.fingerprint())
                .await
                .unwrap(),
        ),
        vault.clone(),
    )
    .unwrap();
    let unknown_body = ResultPayload {
        protocol: ReplayProtocol::ChatCompletions,
        status_code: 502,
        body: Zeroizing::new(br#"{"error":{"code":"provider_unconfirmed"}}"#.to_vec()),
    };
    let pending_result = delivery
        .settle_with_result(&pending, &unknown_body, 86400)
        .await
        .unwrap();
    assert_eq!(pending_result.amount, None);
    assert_eq!(pending_result.confidence, "unknown");
    assert!(restarted.settle(&pending).await.unwrap().duplicate);
    let winner_read_grant = Uuid::new_v4();
    sqlx::query("UPDATE clients SET scopes='[\"infer\",\"read_result\"]' WHERE id=$1")
        .bind(winner.client_id)
        .execute(&store.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO grants(id,installation_id,principal_id,client_id,project_binding,action,bounds,expires_at) VALUES($1,$2,'fixture-worker',$3,'installation','read_result','{}',now()+interval '1 hour')").bind(winner_read_grant).bind(installation).bind(winner.client_id).execute(&store.pool).await.unwrap();
    let winner_reader = ResultReader {
        client_id: winner.client_id,
        principal_id: "fixture-worker".into(),
        grant_id: winner_read_grant,
    };
    assert!(matches!(
        delivery
            .read_result(&winner_reader, receipt.request_id)
            .await
            .unwrap(),
        ReplayOutcome::Unknown
    ));
    let pending_hold: String =
        sqlx::query_scalar("SELECT state FROM reservations WHERE attempt_id=$1")
            .bind(claim.attempt_id)
            .fetch_one(&store.pool)
            .await
            .unwrap();
    assert_eq!(pending_hold, "held");
    let confirmation = SettlementFact {
        source_event_id: "receipt-first".into(),
        acceptance: Acceptance::Accepted,
        receipt: Some(ProviderCharge {
            amount: Amount::parse("0.0008").unwrap(),
            currency: Currency::parse("USD").unwrap(),
            external_id: "receipt-first".into(),
            digest: "a".repeat(64),
        }),
        ..pending.clone()
    };
    // Failure at the last audit statement leaves neither partial charge nor a released reserve.
    sqlx::raw_sql("CREATE FUNCTION reject_fixture_settle_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='request.settle' THEN RAISE EXCEPTION 'fixture audit unavailable'; END IF; RETURN NEW; END; $$; CREATE TRIGGER reject_fixture_settle_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_fixture_settle_audit();").execute(&store.pool).await.unwrap();
    assert!(restarted.settle(&confirmation).await.is_err());
    let still_held: (String, String) = sqlx::query_as(
        "SELECT charged::text,reserved::text FROM budget_periods WHERE policy_id=$1",
    )
    .bind(budget)
    .fetch_one(&store.pool)
    .await
    .unwrap();
    assert_eq!(
        still_held,
        ("0.000000000000000000".into(), "0.000600000000000000".into())
    );
    sqlx::raw_sql("DROP TRIGGER reject_fixture_settle_audit ON audit_events; DROP FUNCTION reject_fixture_settle_audit();").execute(&store.pool).await.unwrap();
    let (confirm_one, confirm_two) =
        tokio::join!(store.settle(&confirmation), restarted.settle(&confirmation));
    let (confirm_one, confirm_two) = (confirm_one.unwrap(), confirm_two.unwrap());
    assert_eq!(confirm_one.ledger_id, confirm_two.ledger_id);
    assert_ne!(confirm_one.duplicate, confirm_two.duplicate);
    assert_eq!(confirm_one.confidence, "confirmed");
    let overrun: (String, String) = sqlx::query_as(
        "SELECT charged::text,reserved::text FROM budget_periods WHERE policy_id=$1",
    )
    .bind(budget)
    .fetch_one(&store.pool)
    .await
    .unwrap();
    assert_eq!(
        overrun,
        ("0.000800000000000000".into(), "0.000000000000000000".into())
    );
    assert!(
        matches!(store.reserve(loser).await, Err(HubError::BudgetExceeded)),
        "actual overrun blocks new admission and is not clamped"
    );
    let conflict = SettlementFact {
        terminal: TerminalState::Completed,
        ..confirmation.clone()
    };
    assert!(matches!(
        store.settle(&conflict).await,
        Err(HubError::IdempotencyConflict)
    ));
    let refund = SettlementFact {
        source_event_id: "receipt-refund".into(),
        receipt: Some(ProviderCharge {
            amount: Amount::parse("0.0004").unwrap(),
            currency: Currency::parse("USD").unwrap(),
            external_id: "receipt-refund".into(),
            digest: "b".repeat(64),
        }),
        ..confirmation.clone()
    };
    let refunded = store.settle(&refund).await.unwrap();
    assert_eq!(refunded.amount.unwrap().to_string(), "0.000400000000000000");
    let credit: String = sqlx::query_scalar("SELECT amount::text FROM ledger_entries WHERE id=$1")
        .bind(refunded.ledger_id)
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(credit, "-0.000400000000000000");
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT charged::text FROM budget_periods WHERE policy_id=$1"
        )
        .bind(budget)
        .fetch_one(&store.pool)
        .await
        .unwrap(),
        "0.000400000000000000"
    );
    let unknown_later = SettlementFact {
        source_event_id: "unknown-later".into(),
        ..pending.clone()
    };
    assert!(matches!(
        store.settle(&unknown_later).await,
        Err(HubError::PreconditionFailed)
    ));
    // A separate current sender settles an estimate, then its receipt replaces the basis.
    sqlx::query("UPDATE budget_policies SET hard_limit=0.002 WHERE id=$1")
        .bind(budget)
        .execute(&store.pool)
        .await
        .unwrap();
    let estimate_admission = store.reserve(loser).await.unwrap();
    let estimate_claim = store
        .claim_dispatch(estimate_admission.attempt_id, Uuid::new_v4(), 120)
        .await
        .unwrap();
    let estimated_fact = SettlementFact {
        attempt_id: estimate_claim.attempt_id,
        authority: SettlementAuthority::Dispatch {
            owner_id: estimate_claim.owner_id,
            fence: estimate_claim.fence,
        },
        source: "synthetic-v1".into(),
        source_event_id: "estimate-event".into(),
        acceptance: Acceptance::Accepted,
        terminal: TerminalState::Completed,
        usage: Some(Usage::normalized(50, 0, 10, "synthetic-qualified-usage".into()).unwrap()),
        receipt: None,
    };
    let body = ResultPayload {
        protocol: ReplayProtocol::ChatCompletions,
        status_code: 200,
        body: Zeroizing::new(
            b"{ \"object\": \"chat.completion\", \"id\":\"result-canary\", \"choices\":[] }\n"
                .to_vec(),
        ),
    };
    let wrong_protocol = ResultPayload {
        protocol: ReplayProtocol::Responses,
        status_code: 200,
        body: Zeroizing::new(body.body.to_vec()),
    };
    assert!(matches!(
        delivery
            .settle_with_result(&estimated_fact, &wrong_protocol, 2)
            .await,
        Err(HubError::PreconditionFailed)
    ));
    assert!(matches!(
        delivery
            .settle_with_result(&estimated_fact, &body, 86401)
            .await,
        Err(HubError::Invalid(_))
    ));
    // Last-statement failure must leave no reply receipt/ciphertext or expense transition.
    sqlx::raw_sql("CREATE FUNCTION reject_reply_fixture_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='request.settle' THEN RAISE EXCEPTION 'fixture final audit unavailable'; END IF; RETURN NEW; END; $$; CREATE TRIGGER reject_reply_fixture_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_reply_fixture_audit();").execute(&store.pool).await.unwrap();
    assert!(
        delivery
            .settle_with_result(&estimated_fact, &body, 2)
            .await
            .is_err()
    );
    let rolled_back:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM replay_receipts WHERE request_id=$1),(SELECT count(*) FROM replay_payloads WHERE request_id=$1),(SELECT count(*) FROM attempt_expenses WHERE attempt_id=$2)").bind(estimate_admission.request_id).bind(estimated_fact.attempt_id).fetch_one(&store.pool).await.unwrap();
    assert_eq!(rolled_back, (0, 0, 0));
    sqlx::raw_sql("DROP TRIGGER reject_reply_fixture_audit ON audit_events; DROP FUNCTION reject_reply_fixture_audit();").execute(&store.pool).await.unwrap();
    let estimated = delivery
        .settle_with_result(&estimated_fact, &body, 2)
        .await
        .unwrap();
    assert_eq!(estimated.confidence, "estimated");
    assert_eq!(
        estimated.amount.unwrap().to_string(),
        "0.000180000000000000"
    );
    assert!(
        delivery
            .settle_with_result(&estimated_fact, &body, 86400)
            .await
            .unwrap()
            .duplicate
    );
    let changed_body = ResultPayload {
        protocol: body.protocol,
        status_code: 200,
        body: Zeroizing::new(br#"{"id":"different-result"}"#.to_vec()),
    };
    assert!(matches!(
        delivery
            .settle_with_result(&estimated_fact, &changed_body, 2)
            .await,
        Err(HubError::IdempotencyConflict)
    ));
    let reader = ResultReader {
        client_id: loser.client_id,
        principal_id: loser.principal_id.clone(),
        grant_id: loser.grant_id,
    };
    assert!(matches!(
        delivery
            .read_result(&reader, estimate_admission.request_id)
            .await,
        Err(HubError::Forbidden)
    ));
    sqlx::query("UPDATE clients SET scopes='[\"infer\",\"read_result\"]' WHERE id=$1")
        .bind(loser.client_id)
        .execute(&store.pool)
        .await
        .unwrap();
    assert!(
        matches!(
            delivery
                .read_result(&reader, estimate_admission.request_id)
                .await,
            Err(HubError::Forbidden)
        ),
        "verification/metadata grant is not result authority"
    );
    let result_grant = Uuid::new_v4();
    sqlx::query("INSERT INTO grants(id,installation_id,principal_id,client_id,project_binding,action,bounds,expires_at) VALUES($1,$2,'fixture-worker',$3,'installation','read_result','{}',now()+interval '1 hour')").bind(result_grant).bind(installation).bind(loser.client_id).execute(&store.pool).await.unwrap();
    let reader = ResultReader {
        grant_id: result_grant,
        ..reader
    };
    let ReplayOutcome::Available(replayed) = delivery
        .read_result(&reader, estimate_admission.request_id)
        .await
        .unwrap()
    else {
        panic!("fresh own result unavailable")
    };
    assert_eq!(&*replayed.body, &*body.body);
    assert_eq!(replayed.status_code, 200);
    assert!(matches!(
        delivery
            .read_result(&winner_reader, estimate_admission.request_id)
            .await,
        Err(HubError::NotFound)
    ));
    let safe_fields:String=sqlx::query_scalar("SELECT (SELECT jsonb_agg(to_jsonb(e))::text FROM audit_events e)||(SELECT jsonb_agg(to_jsonb(o))::text FROM operations o)||(SELECT jsonb_agg(to_jsonb(l))::text FROM ledger_entries l)").fetch_one(&store.pool).await.unwrap();
    assert!(!safe_fields.contains("result-canary"));
    sqlx::query("UPDATE grants SET revoked_at=now() WHERE id=$1")
        .bind(result_grant)
        .execute(&store.pool)
        .await
        .unwrap();
    assert!(matches!(
        delivery
            .read_result(&reader, estimate_admission.request_id)
            .await,
        Err(HubError::Forbidden)
    ));
    sqlx::query("UPDATE grants SET revoked_at=NULL WHERE id=$1")
        .bind(result_grant)
        .execute(&store.pool)
        .await
        .unwrap();
    let estimate_receipt = SettlementFact {
        source_event_id: "estimate-receipt".into(),
        receipt: Some(ProviderCharge {
            amount: Amount::parse("0.0002").unwrap(),
            currency: Currency::parse("USD").unwrap(),
            external_id: "estimate-receipt".into(),
            digest: "c".repeat(64),
        }),
        ..estimated_fact.clone()
    };
    assert_eq!(
        store.settle(&estimate_receipt).await.unwrap().confidence,
        "confirmed"
    );
    let total: String =
        sqlx::query_scalar("SELECT charged::text FROM budget_periods WHERE policy_id=$1")
            .bind(budget)
            .fetch_one(&store.pool)
            .await
            .unwrap();
    assert_eq!(
        total, "0.000600000000000000",
        "confirmed replaces estimate instead of double charging"
    );
    sqlx::query("SELECT pg_sleep(2.1)")
        .execute(&store.pool)
        .await
        .unwrap();
    assert!(matches!(
        delivery
            .read_result(&reader, estimate_admission.request_id)
            .await
            .unwrap(),
        ReplayOutcome::Expired
    ));
    assert_eq!(delivery.purge_expired_results(100).await.unwrap(), 1);
    assert!(
        delivery
            .settle_with_result(&estimated_fact, &body, 86400)
            .await
            .unwrap()
            .duplicate
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM replay_payloads WHERE request_id=$1")
            .bind(estimate_admission.request_id)
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        0,
        "expired response cannot be recreated by replay"
    );
    assert!(
        sqlx::query(
            "UPDATE replay_receipts SET expires_at=now()+interval '1 day' WHERE request_id=$1"
        )
        .bind(estimate_admission.request_id)
        .execute(&store.pool)
        .await
        .is_err()
    );
    assert!(
        sqlx::query("UPDATE settlement_facts SET fact='{}'")
            .execute(&store.pool)
            .await
            .is_err()
    );
    // Owned cancellation before a claim releases only its reserve, even with a nonzero request fee.
    sqlx::query("UPDATE budget_policies SET hard_limit=0.10 WHERE id=$1")
        .bind(budget)
        .execute(&store.pool)
        .await
        .unwrap();
    let fee_price = Uuid::new_v4();
    sqlx::query("INSERT INTO price_revisions(id,installation_id,connection_id,model_id,tier,currency,input_uncached,input_cached,output_billable,request_fee,effective_from,source) VALUES($1,$2,$3,'model-a','metered','USD',2,0.5,8,0.03,now()-interval '1 minute','synthetic-fixed-fee')").bind(fee_price).bind(installation).bind(connection).execute(&store.pool).await.unwrap();
    let unsent = fixture_intent(&store, connection, fee_price, qualification).await;
    let unsent_admission = store.reserve(&unsent).await.unwrap();
    let unsent_owner = RequestOwner {
        client_id: unsent.client_id,
        principal_id: unsent.principal_id.clone(),
    };
    assert!(matches!(
        store
            .cancel_owned(
                &RequestOwner {
                    client_id: winner.client_id,
                    principal_id: winner.principal_id.clone()
                },
                unsent_admission.request_id
            )
            .await,
        Err(HubError::NotFound)
    ));
    sqlx::raw_sql("CREATE FUNCTION reject_cancel_fixture_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='request.settle' THEN RAISE EXCEPTION 'fixture final cancel audit unavailable'; END IF; RETURN NEW; END; $$; CREATE TRIGGER reject_cancel_fixture_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_cancel_fixture_audit();").execute(&store.pool).await.unwrap();
    assert!(
        store
            .cancel_owned(&unsent_owner, unsent_admission.request_id)
            .await
            .is_err()
    );
    let pending_cancel:(String,bool,String)=sqlx::query_as("SELECT r.state,r.cancel_requested,s.state FROM requests r JOIN attempts a ON a.request_id=r.id JOIN reservations s ON s.attempt_id=a.id WHERE r.id=$1").bind(unsent_admission.request_id).fetch_one(&store.pool).await.unwrap();
    assert_eq!(pending_cancel, ("admitted".into(), true, "held".into()));
    assert!(matches!(
        store
            .claim_dispatch(unsent_admission.attempt_id, Uuid::new_v4(), 120)
            .await,
        Err(HubError::PreconditionFailed)
    ));
    sqlx::raw_sql("DROP TRIGGER reject_cancel_fixture_audit ON audit_events; DROP FUNCTION reject_cancel_fixture_audit();").execute(&store.pool).await.unwrap();
    sqlx::query("UPDATE grants SET revoked_at=now() WHERE id=$1")
        .bind(unsent.grant_id)
        .execute(&store.pool)
        .await
        .unwrap();
    let (cancel_a, cancel_b) = tokio::join!(
        store.cancel_owned(&unsent_owner, unsent_admission.request_id),
        second_store.cancel_owned(&unsent_owner, unsent_admission.request_id)
    );
    assert_eq!(cancel_a.unwrap().state, "cancelled");
    assert_eq!(cancel_b.unwrap().state, "cancelled");
    let cancel_expense: (String, String) =
        sqlx::query_as("SELECT amount::text,confidence FROM attempt_expenses WHERE attempt_id=$1")
            .bind(unsent_admission.attempt_id)
            .fetch_one(&store.pool)
            .await
            .unwrap();
    assert_eq!(
        cancel_expense,
        ("0.000000000000000000".into(), "confirmed".into())
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM ledger_entries WHERE attempt_id=$1 AND kind='release'"
        )
        .bind(unsent_admission.attempt_id)
        .fetch_one(&store.pool)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT reserved::text FROM grant_accounts WHERE grant_id=$1"
        )
        .bind(unsent.grant_id)
        .fetch_one(&store.pool)
        .await
        .unwrap(),
        "0.000000000000000000"
    );
    assert!(
        sqlx::query_scalar::<_, Option<Uuid>>("SELECT dispatch_fence FROM attempts WHERE id=$1")
            .bind(unsent_admission.attempt_id)
            .fetch_one(&store.pool)
            .await
            .unwrap()
            .is_none()
    );
    // Dispatch has already committed: cancellation is an intent, not a billing witness.
    let sent = fixture_intent(&store, connection, price, qualification).await;
    let sent_admission = store.reserve(&sent).await.unwrap();
    let sent_owner = RequestOwner {
        client_id: sent.client_id,
        principal_id: sent.principal_id.clone(),
    };
    let sent_claim = store
        .claim_dispatch(sent_admission.attempt_id, Uuid::new_v4(), 120)
        .await
        .unwrap();
    let after_claim = store
        .cancel_owned(&sent_owner, sent_admission.request_id)
        .await
        .unwrap();
    assert_eq!(after_claim.state, "dispatching");
    assert!(after_claim.cancel_requested);
    let forged_no_send = SettlementFact {
        attempt_id: sent_admission.attempt_id,
        authority: SettlementAuthority::BeforeDispatchCancellation {
            cancellation: sent_owner.clone(),
        },
        source: "hub-cancel-before-dispatch".into(),
        source_event_id: sent_admission.attempt_id.to_string(),
        acceptance: Acceptance::NotAccepted,
        terminal: TerminalState::Cancelled,
        usage: None,
        receipt: None,
    };
    assert!(matches!(
        store.settle(&forged_no_send).await,
        Err(HubError::PreconditionFailed)
    ));
    store.record_uncertain(&sent_claim).await.unwrap();
    assert_eq!(
        store
            .cancel_owned(&sent_owner, sent_admission.request_id)
            .await
            .unwrap()
            .state,
        "unknown"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT state FROM reservations WHERE attempt_id=$1")
            .bind(sent_admission.attempt_id)
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        "held"
    );
    // The request row serializes a real cancel/claim race across two stores.
    let racing = fixture_intent(&store, connection, price, qualification).await;
    let racing_admission = store.reserve(&racing).await.unwrap();
    let racing_owner = RequestOwner {
        client_id: racing.client_id,
        principal_id: racing.principal_id.clone(),
    };
    let (race_cancel, race_claim) = tokio::join!(
        store.cancel_owned(&racing_owner, racing_admission.request_id),
        second_store.claim_dispatch(racing_admission.attempt_id, Uuid::new_v4(), 120)
    );
    let race_cancel = race_cancel.unwrap();
    if race_claim.is_ok() {
        assert_eq!(race_cancel.state, "dispatching");
    } else {
        assert!(matches!(race_claim, Err(HubError::PreconditionFailed)));
        assert_eq!(race_cancel.state, "cancelled");
    }
    // Expired unclaimed intent is fenced and held; a still-live neighbor is not recovered.
    let stale_intent = fixture_intent_ttl(&store, connection, price, qualification, 5).await;
    let stale_admission = store.reserve(&stale_intent).await.unwrap();
    let live_intent = fixture_intent(&store, connection, price, qualification).await;
    let live_admission = store.reserve(&live_intent).await.unwrap();
    assert_eq!(store.recover_expired_intents().await.unwrap(), 0);
    assert!(
        sqlx::query("UPDATE requests SET intent_deadline=now()+interval '1 hour' WHERE id=$1")
            .bind(stale_admission.request_id)
            .execute(&store.pool)
            .await
            .is_err()
    );
    sqlx::query("SELECT pg_sleep(5.1)")
        .execute(&store.pool)
        .await
        .unwrap();
    assert!(matches!(
        store
            .claim_dispatch(stale_admission.attempt_id, Uuid::new_v4(), 120)
            .await,
        Err(HubError::PreconditionFailed)
    ));
    assert_eq!(store.recover_expired_intents().await.unwrap(), 1);
    assert_eq!(second_store.recover_expired_intents().await.unwrap(), 0);
    assert_eq!(store.reserve(&stale_intent).await.unwrap().state, "unknown");
    assert_eq!(store.reserve(&live_intent).await.unwrap().state, "admitted");
    assert_eq!(
        store.reserve(&live_intent).await.unwrap().request_id,
        live_admission.request_id
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT state FROM reservations WHERE attempt_id=$1")
            .bind(stale_admission.attempt_id)
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        "held"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM ledger_entries WHERE attempt_id=$1 AND kind='release'"
        )
        .bind(stale_admission.attempt_id)
        .fetch_one(&store.pool)
        .await
        .unwrap(),
        0
    );
    // A future policy change invalidates a queued proof even while the effective quote stays the same.
    let queued = fixture_intent(&store, connection, price, qualification).await;
    let queued_admission = store.reserve(&queued).await.unwrap();
    let replacement_price = Uuid::new_v4();
    sqlx::query("INSERT INTO price_revisions(id,installation_id,connection_id,model_id,tier,currency,input_uncached,input_cached,output_billable,request_fee,effective_from,source) VALUES($1,$2,$3,'model-a','metered','USD',20,5,80,0,now()-interval '1 minute','synthetic-new-price')").bind(replacement_price).bind(installation).bind(connection).execute(&store.pool).await.unwrap();
    let replacement = aihub_domain::pricing_sources::PricingSourceInput {
        connection_id: connection,
        model_id: "model-a".into(),
        currency: Currency::parse("USD").unwrap(),
        mode: aihub_domain::pricing_sources::PricingMode::Manual,
        manual_price_revision_id: Some(replacement_price),
        expected_version: queued.pricing_policy_version,
        effective_from: chrono::Utc::now() + chrono::Duration::seconds(30),
        effective_to: None,
    };
    use aihub_application::OperationBinding;
    store
        .create_pricing_source(
            "fixture-worker",
            Uuid::new_v4(),
            vault
                .bind(&serde_json::to_value(&replacement).unwrap())
                .unwrap(),
            &replacement,
        )
        .await
        .unwrap();
    let current = store
        .resolve_pricing(connection, "model-a", &Currency::parse("USD").unwrap())
        .await
        .unwrap();
    assert_eq!(
        current.source_revision_id,
        queued.pricing_source_revision_id
    );
    assert_eq!(current.price_revision_id, Some(price));
    assert!(current.policy_version > queued.pricing_policy_version);
    assert!(matches!(
        store
            .claim_dispatch(queued_admission.attempt_id, Uuid::new_v4(), 120)
            .await,
        Err(HubError::PreconditionFailed)
    ));
    assert_eq!(
        store.reserve(&queued).await.unwrap().request_id,
        queued_admission.request_id,
        "same key returns old request, never re-admits it under the new policy"
    );
    assert!(matches!(
        store
            .reserve(&AdmissionIntent {
                pricing_policy_version: current.policy_version,
                ..queued.clone()
            })
            .await,
        Err(HubError::IdempotencyConflict)
    ));
    assert!(
        sqlx::query(
            "UPDATE requests SET pricing_policy_version=pricing_policy_version+1 WHERE id=$1"
        )
        .bind(queued_admission.request_id)
        .execute(&store.pool)
        .await
        .is_err()
    );
    let fresh = fixture_intent(&store, connection, price, qualification).await;
    let fresh_admission = store.reserve(&fresh).await.unwrap();
    let fresh_claim = store
        .claim_dispatch(fresh_admission.attempt_id, Uuid::new_v4(), 120)
        .await
        .unwrap();
    let immediate = aihub_domain::pricing_sources::PricingSourceInput {
        expected_version: current.policy_version,
        effective_from: chrono::Utc::now() - chrono::Duration::milliseconds(1),
        ..replacement
    };
    store
        .create_pricing_source(
            "fixture-worker",
            Uuid::new_v4(),
            vault
                .bind(&serde_json::to_value(&immediate).unwrap())
                .unwrap(),
            &immediate,
        )
        .await
        .unwrap();
    assert_eq!(
        store
            .resolve_pricing(connection, "model-a", &Currency::parse("USD").unwrap())
            .await
            .unwrap()
            .price_revision_id,
        Some(replacement_price)
    );
    let late_usage = SettlementFact {
        attempt_id: fresh_claim.attempt_id,
        authority: SettlementAuthority::Dispatch {
            owner_id: fresh_claim.owner_id,
            fence: fresh_claim.fence,
        },
        source: "synthetic-v1".into(),
        source_event_id: "after-source-change".into(),
        acceptance: Acceptance::Accepted,
        terminal: TerminalState::Completed,
        usage: Some(Usage::normalized(50, 0, 10, "synthetic-old-snapshot".into()).unwrap()),
        receipt: None,
    };
    assert_eq!(
        store
            .settle(&late_usage)
            .await
            .unwrap()
            .amount
            .unwrap()
            .to_string(),
        "0.000180000000000000",
        "late settlement uses frozen 2/8 rates, not replacement 20/80 rates"
    );
    assert!(
        sqlx::query("UPDATE ledger_entries SET amount=0")
            .execute(&store.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM ledger_entries")
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
    assert!(
        sqlx::query("UPDATE attempts SET deployment_snapshot='{}' WHERE id=$1")
            .bind(receipt.attempt_id)
            .execute(&store.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE reservations SET currency='EUR' WHERE attempt_id=$1")
            .bind(receipt.attempt_id)
            .execute(&store.pool)
            .await
            .is_err()
    );
}
