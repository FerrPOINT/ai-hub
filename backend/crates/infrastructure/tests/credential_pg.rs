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
    postgres::PgStore,
    vault::{Sealed, Vault},
};
use async_trait::async_trait;
use sqlx::Row;
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
#[tokio::test]
#[ignore = "requires explicit isolated PostgreSQL17 fixture"]
async fn credential_write_only_generation_revoke_replay_and_atomic_audit() {
    let dsn = std::env::var("AIHUB_TEST_DATABASE_URL").unwrap();
    let installation = Uuid::new_v4();
    let vault = Arc::new(Vault::new(vec![7; 32]).unwrap());
    let store = Arc::new(
        PgStore::connect(&dsn, installation, vault.fingerprint())
            .await
            .unwrap(),
    );
    store.migrate().await.unwrap();
    store.initialize("credential-fixture").await.unwrap();
    store.ready().await.unwrap();
    store
        .configure_endpoints(&[EndpointPolicyInput {
            policy_ref: "credential-fixture".into(),
            provider_kind: ProviderKind::OpenaiCompatible,
            base_url: "https://api.example.test/v1/".into(),
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
        subject: "credential-actor".into(),
        authentication: HumanAuthentication::PersonalToken {
            scopes: ["ai-hub:write".into()].into(),
        },
    };
    let conn = foundation
        .save_connection(
            &principal,
            Uuid::new_v4(),
            ProviderKind::OpenaiCompatible,
            None,
            &ConnectionInput {
                display_name: "credential fixture".into(),
                endpoint_policy_ref: "credential-fixture".into(),
                billing_mode: BillingMode::Metered,
            },
        )
        .await
        .unwrap()
        .value;
    let input = CredentialInput {
        secret: Zeroizing::new("synthetic-credential-canary-20261010".into()),
        credential_type: CredentialType::ApiKey,
        expected_generation: 1,
    };
    let key = Uuid::new_v4();
    let second = Foundation {
        installation_id: installation,
        auth: Arc::new(UnusedAuth),
        store: Arc::new(
            PgStore::connect(&dsn, installation, vault.fingerprint())
                .await
                .unwrap(),
        ),
        bindings: vault.clone(),
        secrets: vault.clone(),
    };
    let (one, two) = tokio::join!(
        foundation.write_credential(&principal, key, conn.id, &input),
        second.write_credential(&principal, key, conn.id, &input)
    );
    let (one, two) = (one.unwrap(), two.unwrap());
    assert_eq!(one.id, two.id);
    assert_eq!(one.status, "succeeded");
    let read = FoundationStore::read_connection(&*store, conn.id)
        .await
        .unwrap();
    assert_eq!(read.generation, 2);
    assert_eq!(read.version, 2);
    assert!(read.has_credentials);
    assert_eq!(read.status, "authorization_unknown");
    assert!(read.quota.is_none());
    let row =
        sqlx::query("SELECT * FROM credential_versions WHERE connection_id=$1 AND generation=2")
            .bind(conn.id)
            .fetch_one(&store.pool)
            .await
            .unwrap();
    assert_eq!(row.get::<String, _>("state"), "prepared");
    let sealed = Sealed {
        nonce: row.get::<Vec<u8>, _>("nonce").try_into().unwrap(),
        ciphertext: row.get("ciphertext"),
    };
    assert_eq!(
        &*vault
            .open(installation, conn.id, 2, "credential", &sealed)
            .unwrap(),
        input.secret.as_bytes()
    );
    assert!(
        vault
            .open(installation, conn.id, 1, "credential", &sealed)
            .is_err()
    );
    assert!(
        Vault::new(vec![8; 32])
            .unwrap()
            .open(installation, conn.id, 2, "credential", &sealed)
            .is_err()
    );
    let safe:String=sqlx::query_scalar("SELECT (SELECT jsonb_agg(to_jsonb(o))::text FROM operations o)||(SELECT jsonb_agg(to_jsonb(a))::text FROM audit_events a)").fetch_one(&store.pool).await.unwrap();
    assert!(!safe.contains(input.secret.as_str()));
    assert!(
        !serde_json::to_string(&one)
            .unwrap()
            .contains(input.secret.as_str())
    );
    assert!(
        !serde_json::to_string(&read)
            .unwrap()
            .contains(input.secret.as_str())
    );
    let changed = CredentialInput {
        secret: Zeroizing::new("different-synthetic-secret".into()),
        credential_type: CredentialType::ApiKey,
        expected_generation: 1,
    };
    assert!(matches!(
        foundation
            .write_credential(&principal, key, conn.id, &changed)
            .await,
        Err(HubError::IdempotencyConflict)
    ));
    assert!(matches!(
        foundation
            .write_credential(&principal, Uuid::new_v4(), conn.id, &input)
            .await,
        Err(HubError::PreconditionFailed)
    ));
    assert!(
        foundation
            .operation(
                &HumanPrincipal {
                    subject: "other-actor".into(),
                    authentication: HumanAuthentication::BrowserSession
                },
                one.id
            )
            .await
            .is_err()
    );
    let rotate = CredentialInput {
        secret: Zeroizing::new("rotation-synthetic-canary".into()),
        credential_type: CredentialType::ApiKey,
        expected_generation: 2,
    };
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM operations")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE FUNCTION reject_credential_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='credential.write' THEN RAISE EXCEPTION 'fixture audit unavailable'; END IF; RETURN NEW; END; $$; CREATE TRIGGER reject_credential_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_credential_audit();").execute(&store.pool).await.unwrap();
    assert!(
        foundation
            .write_credential(&principal, Uuid::new_v4(), conn.id, &rotate)
            .await
            .is_err()
    );
    assert_eq!(
        FoundationStore::read_connection(&*store, conn.id)
            .await
            .unwrap()
            .generation,
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM operations")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        before
    );
    sqlx::raw_sql("DROP TRIGGER reject_credential_audit ON audit_events; DROP FUNCTION reject_credential_audit();").execute(&store.pool).await.unwrap();
    let revoke_key = Uuid::new_v4();
    let revoked = foundation
        .revoke_credential(&principal, revoke_key, conn.id, 2)
        .await
        .unwrap();
    assert_eq!(
        foundation
            .revoke_credential(&principal, revoke_key, conn.id, 2)
            .await
            .unwrap()
            .id,
        revoked.id
    );
    let after = FoundationStore::read_connection(&*store, conn.id)
        .await
        .unwrap();
    assert_eq!(after.generation, 3);
    assert_eq!(after.status, "revoked");
    assert!(!after.has_credentials);
    assert_eq!(
        foundation
            .write_credential(&principal, key, conn.id, &input)
            .await
            .unwrap()
            .id,
        one.id,
        "old successful write readback cannot restore revoked auth"
    );
    assert_eq!(
        FoundationStore::read_connection(&*store, conn.id)
            .await
            .unwrap()
            .generation,
        3
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT state FROM credential_versions WHERE connection_id=$1 AND generation=2"
        )
        .bind(conn.id)
        .fetch_one(&store.pool)
        .await
        .unwrap(),
        "revoked"
    );
    assert!(
        sqlx::query(
            "UPDATE credential_versions SET ciphertext=$1 WHERE connection_id=$2 AND generation=2"
        )
        .bind(vec![0u8; 32])
        .bind(conn.id)
        .execute(&store.pool)
        .await
        .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM credential_versions WHERE connection_id=$1")
            .bind(conn.id)
            .execute(&store.pool)
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM ledger_entries")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        0
    );
}
