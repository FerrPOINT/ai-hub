use aihub_application::{BudgetFilter, FoundationStore, OperationBinding};
use aihub_domain::{
    NamespaceRef,
    budgets::{BudgetInput, BudgetPeriod, BudgetScope},
    error::HubError,
    financial::{Amount, Currency},
};
use aihub_infrastructure::{postgres::PgStore, vault::Vault};
use chrono::{Datelike, Timelike};
use uuid::Uuid;

async fn seed_namespace(store: &PgStore, registry: Uuid, namespace: Uuid, tracker: Uuid) -> Uuid {
    let binding = Uuid::new_v4();
    sqlx::query("INSERT INTO namespace_bindings(id,installation_id,registry_instance_id,namespace_id,tracker_instance_id,tracker_project_id,state,generation,observed_at,label,tracker_project_key) VALUES($1,$2,$3,$4,$5,$6,'active',1,now(),'same label','FIXTURE')")
        .bind(binding).bind(store.installation_id).bind(registry).bind(namespace).bind(Uuid::new_v4()).bind(tracker).execute(&store.pool).await.unwrap();
    binding
}
async fn grant(store: &PgStore, subject: &str, binding: Option<Uuid>, project: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO grants(id,installation_id,principal_id,project_binding,action,bounds,expires_at,namespace_binding_id) VALUES($1,$2,$3,$4,'metadata.read','{}',now()+interval '1 hour',$5)")
        .bind(id).bind(store.installation_id).bind(subject).bind(project).bind(binding).execute(&store.pool).await.unwrap();
    id
}
fn binding(vault: &Vault, input: &BudgetInput, update: Option<(Uuid, i64)>) -> [u8; 32] {
    vault
        .bind(&serde_json::json!({"policy":input,"update":update}))
        .unwrap()
}

#[tokio::test]
#[ignore = "requires explicit isolated PostgreSQL17 fixture"]
async fn exact_budget_namespace_cas_readback_and_atomic_audit() {
    let dsn = std::env::var("AIHUB_TEST_DATABASE_URL").unwrap();
    let installation = Uuid::new_v4();
    let vault = Vault::new(vec![7; 32]).unwrap();
    let store = PgStore::connect(&dsn, installation, vault.fingerprint())
        .await
        .unwrap();
    store.migrate().await.unwrap();
    store.initialize("budget-fixture").await.unwrap();
    store.ready().await.unwrap();
    grant(&store, "actor-a", None, "installation").await;
    let policy = BudgetInput {
        scope_type: BudgetScope::Installation,
        scope_id: installation,
        currency: Currency::parse("USD").unwrap(),
        period: BudgetPeriod::UtcDay,
        hard_limit: Amount::parse("0.10").unwrap(),
        warning_thresholds: vec![80, 95],
        namespace: None,
    };
    let key = Uuid::new_v4();
    let hash = binding(&vault, &policy, None);
    let second = PgStore::connect(&dsn, installation, vault.fingerprint())
        .await
        .unwrap();
    let (one, two) = tokio::join!(
        store.write_budget("actor-a", key, hash, None, &policy),
        second.write_budget("actor-a", key, hash, None, &policy)
    );
    let (one, two) = (one.unwrap(), two.unwrap());
    assert_eq!(one.operation_id, two.operation_id);
    assert_eq!(one.value.id, two.value.id);
    assert_eq!(one.value.remaining.to_string(), "0.100000000000000000");
    assert_eq!(one.value.period_start.hour(), 0);
    assert_eq!(one.value.period_start.minute(), 0);
    assert_eq!(
        (one.value.period_end - one.value.period_start).num_hours(),
        24
    );
    assert!(matches!(
        store
            .write_budget("actor-b", Uuid::new_v4(), hash, None, &policy)
            .await,
        Err(HubError::Forbidden)
    ));
    assert!(matches!(
        store
            .write_budget("actor-a", Uuid::new_v4(), hash, None, &policy)
            .await,
        Err(HubError::IdempotencyConflict)
    ));
    // Existing charged/reserved balances survive lower limits and CAS mutations.
    sqlx::query("INSERT INTO budget_periods(id,installation_id,policy_id,period_start,period_end,charged,reserved) VALUES($1,$2,$3,$4,$5,0.04,0.02)").bind(Uuid::new_v4()).bind(installation).bind(one.value.id).bind(one.value.period_start).bind(one.value.period_end).execute(&store.pool).await.unwrap();
    let lower = BudgetInput {
        hard_limit: Amount::parse("0.03").unwrap(),
        ..policy.clone()
    };
    let update_key = Uuid::new_v4();
    let update = Some((one.value.id, 1));
    let lower_hash = binding(&vault, &lower, update);
    let lowered = store
        .write_budget("actor-a", update_key, lower_hash, update, &lower)
        .await
        .unwrap();
    assert_eq!(lowered.value.version, 2);
    assert_eq!(lowered.value.remaining.to_string(), "-0.030000000000000000");
    assert_eq!(lowered.value.charged.to_string(), "0.040000000000000000");
    assert_eq!(lowered.value.reserved.to_string(), "0.020000000000000000");
    assert_eq!(
        store
            .write_budget("actor-a", update_key, lower_hash, update, &lower)
            .await
            .unwrap()
            .value
            .version,
        2
    );
    assert!(matches!(
        store
            .write_budget("actor-a", Uuid::new_v4(), lower_hash, update, &lower)
            .await,
        Err(HubError::PreconditionFailed)
    ));
    let foreign_currency = BudgetInput {
        currency: Currency::parse("EUR").unwrap(),
        ..lower.clone()
    };
    assert!(matches!(
        store
            .write_budget(
                "actor-a",
                Uuid::new_v4(),
                binding(&vault, &foreign_currency, Some((one.value.id, 2))),
                Some((one.value.id, 2)),
                &foreign_currency
            )
            .await,
        Err(HubError::Invalid(_))
    ));
    assert!(
        sqlx::query("UPDATE budget_policies SET currency='EUR' WHERE id=$1")
            .bind(one.value.id)
            .execute(&store.pool)
            .await
            .is_err()
    );
    let race_update = Some((one.value.id, 2));
    let race_hash = binding(&vault, &lower, race_update);
    let (first_update, second_update) = tokio::join!(
        store.write_budget("actor-a", Uuid::new_v4(), race_hash, race_update, &lower),
        second.write_budget("actor-a", Uuid::new_v4(), race_hash, race_update, &lower)
    );
    assert_eq!(
        usize::from(first_update.is_ok()) + usize::from(second_update.is_ok()),
        1
    );
    let loser = if first_update.is_err() {
        first_update
    } else {
        second_update
    };
    assert!(matches!(loser, Err(HubError::PreconditionFailed)));
    // The original mutation's readback stays pinned after another accepted version.
    assert_eq!(
        store
            .write_budget("actor-a", update_key, lower_hash, update, &lower)
            .await
            .unwrap()
            .value
            .version,
        2
    );
    let namespace = Uuid::new_v4();
    let tracker = Uuid::new_v4();
    let registry_one = Uuid::new_v4();
    let registry_two = Uuid::new_v4();
    let ns_one = seed_namespace(&store, registry_one, namespace, tracker).await;
    let ns_two = seed_namespace(&store, registry_two, namespace, tracker).await;
    let project_grant = grant(&store, "actor-a", Some(ns_one), &tracker.to_string()).await;
    grant(&store, "actor-b", Some(ns_two), &tracker.to_string()).await;
    let project_one = BudgetInput {
        scope_type: BudgetScope::Project,
        scope_id: namespace,
        namespace: Some(NamespaceRef {
            registry_instance_id: registry_one,
            namespace_id: namespace,
        }),
        ..policy.clone()
    };
    let project_two = BudgetInput {
        namespace: Some(NamespaceRef {
            registry_instance_id: registry_two,
            namespace_id: namespace,
        }),
        ..project_one.clone()
    };
    assert!(matches!(
        store
            .write_budget(
                "actor-a",
                Uuid::new_v4(),
                binding(&vault, &project_two, None),
                None,
                &project_two
            )
            .await,
        Err(HubError::Forbidden)
    ));
    let project = store
        .write_budget(
            "actor-a",
            Uuid::new_v4(),
            binding(&vault, &project_one, None),
            None,
            &project_one,
        )
        .await
        .unwrap();
    let neighboring = store
        .write_budget(
            "actor-b",
            Uuid::new_v4(),
            binding(&vault, &project_two, None),
            None,
            &project_two,
        )
        .await
        .unwrap();
    assert_ne!(
        project.value.id, neighboring.value.id,
        "same label/Namespace UUID/Tracker UUID in another registry is a separate authorized scope"
    );
    let all = BudgetFilter {
        namespace: None,
        unbound_only: false,
    };
    let visible = store.budget_page("actor-a", &all, 100, None).await.unwrap();
    assert_eq!(visible.items.len(), 2);
    assert!(visible.items.iter().all(|b| b.id != neighboring.value.id));
    let unbound = BudgetFilter {
        namespace: None,
        unbound_only: true,
    };
    assert_eq!(
        store
            .budget_page("actor-a", &unbound, 100, None)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    let initial = store.budget_page("actor-a", &all, 1, None).await.unwrap();
    let cursor = Uuid::parse_str(initial.next_cursor.as_ref().unwrap()).unwrap();
    assert!(
        store
            .budget_page("actor-b", &all, 1, Some(cursor))
            .await
            .is_err()
    );
    let revoked = if initial.items[0].id == one.value.id {
        project_grant
    } else {
        sqlx::query_scalar("SELECT id FROM grants WHERE installation_id=$1 AND principal_id='actor-a' AND namespace_binding_id IS NULL").bind(installation).fetch_one(&store.pool).await.unwrap()
    };
    sqlx::query("UPDATE grants SET revoked_at=now() WHERE id=$1")
        .bind(revoked)
        .execute(&store.pool)
        .await
        .unwrap();
    assert!(matches!(
        store.budget_page("actor-a", &all, 1, Some(cursor)).await,
        Err(HubError::Forbidden)
    ));
    // Client Namespace authority is derived from its persisted binding.
    grant(&store, "actor-a", Some(ns_one), &tracker.to_string()).await;
    let client = Uuid::new_v4();
    sqlx::query("INSERT INTO clients(id,installation_id,application_key,project_binding,allowed_profiles,scopes,cost_policy,expires_at,max_concurrency,max_rpm,status,version,namespace_binding_id) VALUES($1,$2,'budget-client',$3,'[]','[\"infer\"]','budget_guaranteed',now()+interval '1 hour',1,10,'enabled',1,$4)")
        .bind(client).bind(installation).bind(tracker.to_string()).bind(ns_one).execute(&store.pool).await.unwrap();
    let client_input = BudgetInput {
        scope_type: BudgetScope::Client,
        scope_id: client,
        namespace: None,
        ..policy.clone()
    };
    assert!(matches!(
        store
            .write_budget(
                "actor-b",
                Uuid::new_v4(),
                binding(&vault, &client_input, None),
                None,
                &client_input
            )
            .await,
        Err(HubError::Forbidden)
    ));
    let client_budget = store
        .write_budget(
            "actor-a",
            Uuid::new_v4(),
            binding(&vault, &client_input, None),
            None,
            &client_input,
        )
        .await
        .unwrap();
    assert_eq!(client_budget.value.namespace, project_one.namespace);
    assert_eq!(client_budget.value.policy.namespace, None);
    let forged_client = BudgetInput {
        namespace: project_two.namespace.clone(),
        ..client_input
    };
    assert!(matches!(
        store
            .write_budget(
                "actor-a",
                Uuid::new_v4(),
                binding(&vault, &forged_client, None),
                None,
                &forged_client
            )
            .await,
        Err(HubError::Invalid(_))
    ));
    let filtered = store
        .budget_page(
            "actor-a",
            &BudgetFilter {
                namespace: project_one.namespace.clone(),
                unbound_only: false,
            },
            100,
            None,
        )
        .await
        .unwrap();
    assert_eq!(filtered.items.len(), 2);
    assert!(
        filtered
            .items
            .iter()
            .all(|b| b.namespace == project_one.namespace)
    );
    // Independent monthly policy proves correct UTC calendar bounds; actor-b has its own authority.
    grant(&store, "actor-b", None, "installation").await;
    let monthly = BudgetInput {
        period: BudgetPeriod::UtcMonth,
        ..policy.clone()
    };
    let month = store
        .write_budget(
            "actor-b",
            Uuid::new_v4(),
            binding(&vault, &monthly, None),
            None,
            &monthly,
        )
        .await
        .unwrap();
    assert_eq!(month.value.period_start.day(), 1);
    assert_eq!(month.value.period_end.day(), 1);
    assert!((28..=31).contains(&(month.value.period_end - month.value.period_start).num_days()));
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM operations")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE FUNCTION reject_budget_fixture_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='budget.update' THEN RAISE EXCEPTION 'fixture audit unavailable'; END IF; RETURN NEW; END; $$; CREATE TRIGGER reject_budget_fixture_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_budget_fixture_audit();").execute(&store.pool).await.unwrap();
    let monthly_update = Some((month.value.id, 1));
    let raised = BudgetInput {
        hard_limit: Amount::parse("1").unwrap(),
        ..monthly
    };
    assert!(
        store
            .write_budget(
                "actor-b",
                Uuid::new_v4(),
                binding(&vault, &raised, monthly_update),
                monthly_update,
                &raised
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
        sqlx::query_scalar::<_, i64>("SELECT version FROM budget_policies WHERE id=$1")
            .bind(month.value.id)
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        1
    );
}
