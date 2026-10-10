use aihub_api::{AppState, router};
use aihub_application::{CentralAuthentication, Foundation, FoundationStore};
use aihub_domain::{
    NamespaceRef,
    access::{HumanAuthentication, HumanPrincipal},
    error::HubError,
    records::{AuditEvent, NamespaceBinding, Operation},
};
use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tower::ServiceExt;
use uuid::Uuid;

struct TestAuth;
#[async_trait]
impl CentralAuthentication for TestAuth {
    async fn authenticate(&self, token: &str) -> Result<HumanPrincipal, HubError> {
        let authentication = match token {
            "session" => HumanAuthentication::BrowserSession,
            "read" => HumanAuthentication::PersonalToken {
                scopes: ["ai-hub:read".into()].into(),
            },
            "write" => HumanAuthentication::PersonalToken {
                scopes: ["ai-hub:write".into()].into(),
            },
            "foreign" => HumanAuthentication::PersonalToken {
                scopes: ["wiki:read".into()].into(),
            },
            "outage" => return Err(HubError::Unavailable),
            _ => return Err(HubError::Unauthenticated),
        };
        Ok(HumanPrincipal {
            subject: "human".into(),
            authentication,
        })
    }
}
struct TestStore {
    reads: AtomicUsize,
}
#[async_trait]
impl FoundationStore for TestStore {
    async fn ready(&self) -> Result<(), HubError> {
        Ok(())
    }
    async fn project_grants(&self, _: &str, _: &str) -> Result<Vec<String>, HubError> {
        Ok(vec![])
    }
    async fn namespaces(&self, _: &str, _: i64) -> Result<Vec<NamespaceBinding>, HubError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(vec![])
    }
    async fn require_namespace(&self, _: &str, _: &NamespaceRef, _: &str) -> Result<(), HubError> {
        Err(HubError::Forbidden)
    }
    async fn audit(
        &self,
        _: &str,
        _: Option<&NamespaceRef>,
        _: i64,
    ) -> Result<Vec<AuditEvent>, HubError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(vec![])
    }
    async fn operation(&self, _: &str, _: Uuid) -> Result<Operation, HubError> {
        Err(HubError::NotFound)
    }
}
fn fixture() -> (axum::Router, Arc<TestStore>) {
    let store = Arc::new(TestStore {
        reads: AtomicUsize::new(0),
    });
    let app = router(
        AppState {
            foundation: Foundation {
                installation_id: Uuid::new_v4(),
                auth: Arc::new(TestAuth),
                store: store.clone(),
            },
            external_calls: false,
            auth_issuer: "http://localhost:8101".into(),
            public_origin: "http://localhost:8192".into(),
            admin_origin: None,
        },
        2097152,
    );
    (app, store)
}
async fn status(app: &axum::Router, path: &str, token: Option<&str>) -> StatusCode {
    let mut request = Request::builder().uri(path);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    app.clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn pat_matrix_and_revocation_fail_before_protected_queries() {
    let (app, store) = fixture();
    for (token, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some("revoked"), StatusCode::UNAUTHORIZED),
        (Some("foreign"), StatusCode::FORBIDDEN),
        (Some("write"), StatusCode::FORBIDDEN),
        (Some("outage"), StatusCode::SERVICE_UNAVAILABLE),
    ] {
        assert_eq!(status(&app, "/api/v1/namespaces", token).await, expected);
    }
    assert_eq!(store.reads.load(Ordering::SeqCst), 0);
    for token in ["read", "session"] {
        assert_eq!(
            status(&app, "/api/v1/namespaces", Some(token)).await,
            StatusCode::OK
        );
    }
    assert_eq!(
        status(&app, "/api/v1/auth/me", Some("write")).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn invalid_namespace_does_not_expand_audit_to_all() {
    let (app, store) = fixture();
    for query in [
        "namespace_id=bad",
        "namespace_id=7f8f5e01-9834-43ad-b773-ef7f7d5b1e0e",
        "registry_instance_id=00000000-0000-0000-0000-000000000000&namespace_id=7f8f5e01-9834-43ad-b773-ef7f7d5b1e0e",
    ] {
        assert_eq!(
            status(&app, &format!("/api/v1/audit?{query}"), Some("read")).await,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(store.reads.load(Ordering::SeqCst), 0);
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    assert_eq!(
        status(
            &app,
            &format!("/api/v1/audit?registry_instance_id={a}&namespace_id={b}"),
            Some("read")
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        status(&app, "/api/v1/audit", Some("read")).await,
        StatusCode::OK
    );
}
