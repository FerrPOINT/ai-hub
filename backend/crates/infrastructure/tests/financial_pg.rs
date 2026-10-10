use aihub_application::{FinancialAdmission, FoundationStore};
use aihub_domain::{
    admission::{AdmissionIntent, Purpose, PurposeBounds},
    error::HubError,
    financial::{Amount, Currency, Usage},
};
use aihub_infrastructure::postgres::PgStore;
use uuid::Uuid;

async fn fixture_intent(
    store: &PgStore,
    connection: Uuid,
    price: Uuid,
    qualification: Uuid,
) -> AdmissionIntent {
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
    let target = serde_json::json!({"connection_id":connection,"generation":1,"model_id":"model-a","tier":"metered","price_revision_id":price,"qualification_id":qualification,"upper_usage":usage});
    sqlx::query("INSERT INTO probe_snapshots(id,installation_id,operation_id,scope,config_hash,configuration) VALUES($1,$2,$3,'connection_model',$4,$5)").bind(probe).bind(store.installation_id).bind(op).bind("1".repeat(64)).bind(serde_json::json!({"targets":[target]})).execute(&store.pool).await.unwrap();
    AdmissionIntent {
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
        price_revision_id: Some(price),
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
    let store = PgStore::connect(&dsn, installation, vec![7; 32])
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
    sqlx::query("INSERT INTO connection_generations(connection_id,generation,authorization_state,adapter_revision,endpoint_policy_hash) VALUES($1,1,'active','synthetic-v1',$2)").bind(connection).bind("1".repeat(64)).execute(&store.pool).await.unwrap();
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
    // A missing price is not zero, even for an otherwise authorized exact target.
    let no_price = AdmissionIntent {
        price_revision_id: None,
        ..fixture_intent(&store, connection, price, qualification).await
    };
    let target = serde_json::json!({"connection_id":connection,"generation":1,"model_id":"model-a","tier":"metered","price_revision_id":null,"qualification_id":qualification,"upper_usage":no_price.upper_usage});
    let no_price_probe = Uuid::new_v4();
    sqlx::query("INSERT INTO probe_snapshots(id,installation_id,operation_id,scope,config_hash,configuration) SELECT $1,installation_id,operation_id,scope,config_hash,$2 FROM probe_snapshots WHERE id=$3")
        .bind(no_price_probe).bind(serde_json::json!({"targets":[target]})).bind(no_price.probe_snapshot_id).execute(&store.pool).await.unwrap();
    let no_price = AdmissionIntent {
        probe_snapshot_id: Some(no_price_probe),
        ..no_price
    };
    assert!(matches!(
        store.reserve(&no_price).await,
        Err(HubError::BudgetExceeded)
    ));
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
    let second_store = PgStore::connect(&dsn, installation, vec![7; 32])
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
    let restarted = PgStore::connect(&dsn, installation, vec![7; 32])
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
        settlement::{ProviderCharge, SettlementFact, TerminalState},
    };
    let pending = SettlementFact {
        attempt_id: claim.attempt_id,
        owner_id: claim.owner_id,
        fence: claim.fence,
        source: "synthetic-v1".into(),
        source_event_id: "pending-event".into(),
        acceptance: Acceptance::Unknown,
        terminal: TerminalState::Failed,
        usage: None,
        receipt: None,
    };
    let pending_result = restarted.settle(&pending).await.unwrap();
    assert_eq!(pending_result.amount, None);
    assert_eq!(pending_result.confidence, "unknown");
    assert!(restarted.settle(&pending).await.unwrap().duplicate);
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
        owner_id: estimate_claim.owner_id,
        fence: estimate_claim.fence,
        source: "synthetic-v1".into(),
        source_event_id: "estimate-event".into(),
        acceptance: Acceptance::Accepted,
        terminal: TerminalState::Completed,
        usage: Some(Usage::normalized(50, 0, 10, "synthetic-qualified-usage".into()).unwrap()),
        receipt: None,
    };
    let estimated = store.settle(&estimated_fact).await.unwrap();
    assert_eq!(estimated.confidence, "estimated");
    assert_eq!(
        estimated.amount.unwrap().to_string(),
        "0.000180000000000000"
    );
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
    assert!(
        sqlx::query("UPDATE settlement_facts SET fact='{}'")
            .execute(&store.pool)
            .await
            .is_err()
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
