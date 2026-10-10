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
struct TestBinding;
impl aihub_application::CredentialProtection for TestBinding {
    fn protect(
        &self,
        _: Uuid,
        _: Uuid,
        _: i64,
        _: &[u8],
    ) -> Result<aihub_domain::connections::ProtectedCredential, HubError> {
        Err(HubError::Unavailable)
    }
}
impl aihub_application::OperationBinding for TestBinding {
    fn bind(&self, _: &serde_json::Value) -> Result<[u8; 32], HubError> {
        Ok([7; 32])
    }
}
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
impl aihub_application::MetadataOperations for TestStore {
    async fn refresh(
        &self,
        _: &HumanPrincipal,
        _: Uuid,
        _: Uuid,
        _: i64,
    ) -> Result<Operation, HubError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Err(HubError::Unavailable)
    }
}
#[async_trait]
impl FoundationStore for TestStore {
    async fn operation_key(
        &self,
        _: &str,
        _: Uuid,
    ) -> Result<aihub_domain::records::OperationLookup, HubError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Err(HubError::NotFound)
    }
    async fn close_unstarted_operation(
        &self,
        _: &str,
        _: Uuid,
        _: [u8; 32],
    ) -> Result<aihub_domain::records::OperationLookup, HubError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Err(HubError::NotFound)
    }
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
    fixture_with_external(false)
}
fn fixture_with_external(external_calls: bool) -> (axum::Router, Arc<TestStore>) {
    let store = Arc::new(TestStore {
        reads: AtomicUsize::new(0),
    });
    let app = router(
        AppState {
            metadata: store.clone(),
            foundation: Foundation {
                installation_id: Uuid::new_v4(),
                auth: Arc::new(TestAuth),
                store: store.clone(),
                bindings: Arc::new(TestBinding),
                secrets: Arc::new(TestBinding),
            },
            external_calls,
            auth_issuer: "http://localhost:8101".into(),
            public_origin: "http://localhost:8192".into(),
            admin_origin: None,
        },
        2097152,
    );
    (app, store)
}
#[tokio::test]
async fn metadata_refresh_requires_write_scope_and_external_opt_in_before_intent() {
    let (app, store) = fixture();
    for (token, expected) in [
        ("read", StatusCode::FORBIDDEN),
        ("foreign", StatusCode::FORBIDDEN),
        ("write", StatusCode::SERVICE_UNAVAILABLE),
    ] {
        let request = Request::builder()
            .method("POST")
            .uri(format!(
                "/api/v1/connections/{}/models/refresh",
                Uuid::new_v4()
            ))
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .header("Idempotency-Key", Uuid::new_v4().to_string())
            .body(Body::from(r#"{"expected_generation":2}"#))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            expected
        );
    }
    assert_eq!(store.reads.load(Ordering::SeqCst), 0);
    let (app, store) = fixture_with_external(true);
    for (body, expected) in [
        (r#"{"expected_generation":0}"#, StatusCode::BAD_REQUEST),
        (
            r#"{"expected_generation":2,"url":"https://evil.example"}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            r#"{"expected_generation":2}"#,
            StatusCode::SERVICE_UNAVAILABLE,
        ),
    ] {
        let request = Request::builder()
            .method("POST")
            .uri(format!(
                "/api/v1/connections/{}/models/refresh",
                Uuid::new_v4()
            ))
            .header("authorization", "Bearer write")
            .header("content-type", "application/json")
            .header("Idempotency-Key", Uuid::new_v4().to_string())
            .body(Body::from(body))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            expected
        );
    }
    assert_eq!(store.reads.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn control_key_readback_and_no_send_close_are_scoped_before_storage() {
    let (app, store) = fixture();
    let path = format!("/api/v1/operations/by-key/{}", Uuid::new_v4());
    assert_eq!(
        status(&app, &path, Some("foreign")).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        status(&app, &path, Some("read")).await,
        StatusCode::NOT_FOUND
    );
    for (token, expected) in [
        ("read", StatusCode::FORBIDDEN),
        ("write", StatusCode::NOT_FOUND),
    ] {
        let request = Request::builder()
            .method("POST")
            .uri(format!("{path}/close-unstarted"))
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            expected
        );
    }
    assert_eq!(store.reads.load(Ordering::SeqCst), 2);
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

#[tokio::test]
async fn price_mutation_rejects_wrong_scope_invalid_headers_and_decimal_numbers() {
    let (app, _) = fixture();
    let price = serde_json::json!({"connection_id":Uuid::new_v4(),"model_id":"model-a","tier":"metered","currency":"USD","unit":"per_million_tokens","input_uncached":"2","input_cached":null,"output_billable":"8","effective_from":"2026-10-10T00:00:00Z","effective_to":null,"source":"synthetic-fixture"});
    for (token, key, body, expected) in [
        (
            "read",
            Uuid::new_v4().to_string(),
            price.clone(),
            StatusCode::FORBIDDEN,
        ),
        (
            "foreign",
            Uuid::new_v4().to_string(),
            price.clone(),
            StatusCode::FORBIDDEN,
        ),
        (
            "write",
            Uuid::nil().to_string(),
            price.clone(),
            StatusCode::BAD_REQUEST,
        ),
        (
            "write",
            "bad".into(),
            price.clone(),
            StatusCode::BAD_REQUEST,
        ),
        (
            "write",
            Uuid::new_v4().to_string(),
            {
                let mut p = price.clone();
                p["input_uncached"] = serde_json::json!(2.0);
                p
            },
            StatusCode::BAD_REQUEST,
        ),
        (
            "write",
            Uuid::new_v4().to_string(),
            {
                let mut p = price.clone();
                p["request_fee"] = serde_json::json!("-1");
                p
            },
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/prices")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .header("idempotency-key", key)
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );
    }
}

#[tokio::test]
async fn budget_http_requires_exact_scope_pair_decimal_and_strong_cas() {
    let (app, _) = fixture();
    let policy = serde_json::json!({"scope_type":"client","scope_id":Uuid::new_v4(),"currency":"USD","period":"utc_day","hard_limit":"0.10","warning_thresholds":[80,95],"namespace":null});
    let path = format!("/api/v1/budgets/{}", Uuid::new_v4());
    for (token, versions, expected) in [
        ("read", vec!["\"1\""], StatusCode::FORBIDDEN),
        ("foreign", vec!["\"1\""], StatusCode::FORBIDDEN),
        ("write", vec![], StatusCode::BAD_REQUEST),
        ("write", vec!["1"], StatusCode::BAD_REQUEST),
        ("write", vec!["W/\"1\""], StatusCode::BAD_REQUEST),
        ("write", vec!["\"0\""], StatusCode::BAD_REQUEST),
        ("write", vec!["\"-1\""], StatusCode::BAD_REQUEST),
        ("write", vec!["\"1\"", "\"2\""], StatusCode::BAD_REQUEST),
        ("write", vec!["\"1\""], StatusCode::SERVICE_UNAVAILABLE),
    ] {
        let mut request = Request::builder()
            .method("PATCH")
            .uri(&path)
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .header("idempotency-key", Uuid::new_v4().to_string());
        for version in versions {
            request = request.header("if-match", version);
        }
        let response = app
            .clone()
            .oneshot(
                request
                    .body(Body::from(serde_json::to_vec(&policy).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );
    }
    for body in [
        {
            let mut b = policy.clone();
            b["hard_limit"] = serde_json::json!(0.1);
            b
        },
        {
            let mut b = policy.clone();
            b["warning_thresholds"] = serde_json::json!([80, 80]);
            b
        },
        {
            let mut b = policy.clone();
            b.as_object_mut().unwrap().remove("namespace");
            b
        },
        {
            let mut b = policy.clone();
            b["namespace"] = serde_json::json!({"registry_instance_id":Uuid::new_v4(),"namespace_id":Uuid::new_v4()});
            b
        },
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/budgets")
                    .header("authorization", "Bearer write")
                    .header("content-type", "application/json")
                    .header("idempotency-key", Uuid::new_v4().to_string())
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    for query in [
        "binding=bad",
        "binding=all&binding=unbound",
        "namespace_id=bad",
        "binding=unbound&registry_instance_id=00000000-0000-0000-0000-000000000001&namespace_id=00000000-0000-0000-0000-000000000002",
    ] {
        assert_eq!(
            status(&app, &format!("/api/v1/budgets?{query}"), Some("read")).await,
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn source_control_requires_write_scope_key_and_explicit_nullable_fields() {
    let (app, _) = fixture();
    let input = serde_json::json!({"connection_id":Uuid::new_v4(),"model_id":"model","currency":"USD","mode":"manual","manual_price_revision_id":Uuid::new_v4(),"expected_version":0,"effective_from":"2026-10-10T00:00:00Z","effective_to":null});
    for (token, key, body, expected) in [
        (
            "read",
            Uuid::new_v4().to_string(),
            input.clone(),
            StatusCode::FORBIDDEN,
        ),
        (
            "foreign",
            Uuid::new_v4().to_string(),
            input.clone(),
            StatusCode::FORBIDDEN,
        ),
        (
            "write",
            Uuid::nil().to_string(),
            input.clone(),
            StatusCode::BAD_REQUEST,
        ),
        (
            "write",
            Uuid::new_v4().to_string(),
            {
                let mut b = input.clone();
                b.as_object_mut().unwrap().remove("effective_to");
                b
            },
            StatusCode::BAD_REQUEST,
        ),
        (
            "write",
            Uuid::new_v4().to_string(),
            {
                let mut b = input.clone();
                b.as_object_mut()
                    .unwrap()
                    .remove("manual_price_revision_id");
                b
            },
            StatusCode::BAD_REQUEST,
        ),
        (
            "write",
            Uuid::new_v4().to_string(),
            {
                let mut b = input.clone();
                b["expected_version"] = serde_json::json!(-1);
                b
            },
            StatusCode::BAD_REQUEST,
        ),
        (
            "write",
            Uuid::new_v4().to_string(),
            input.clone(),
            StatusCode::SERVICE_UNAVAILABLE,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pricing-sources")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .header("idempotency-key", key)
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );
    }
    assert_eq!(
        status(
            &app,
            "/api/v1/pricing-sources?namespace_id=bad",
            Some("read")
        )
        .await,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn connection_control_cannot_accept_a_raw_url_secret_or_wrong_pat_scope() {
    let (app, _) = fixture();
    let input = serde_json::json!({"display_name":"connection","endpoint_policy_ref":"openrouter-api-v1","billing_mode":"metered"});
    for (token, body, expected) in [
        ("read", input.clone(), StatusCode::FORBIDDEN),
        ("foreign", input.clone(), StatusCode::FORBIDDEN),
        (
            "write",
            {
                let mut p = input.clone();
                p["endpoint_policy_ref"] = serde_json::json!("https://arbitrary.test/");
                p
            },
            StatusCode::BAD_REQUEST,
        ),
        (
            "write",
            {
                let mut p = input.clone();
                p["secret"] = serde_json::json!("synthetic-secret-canary");
                p
            },
            StatusCode::BAD_REQUEST,
        ),
        ("write", input.clone(), StatusCode::SERVICE_UNAVAILABLE),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/providers/openai_compatible/connections")
                    .header("authorization", format!("Bearer {token}"))
                    .header("idempotency-key", Uuid::new_v4().to_string())
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        let bytes = axum::body::to_bytes(response.into_body(), 65536)
            .await
            .unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-secret-canary"));
    }
    assert_eq!(
        status(&app, "/api/v1/connections?namespace_id=bad", Some("read")).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status(
            &app,
            "/api/v1/connections/00000000-0000-0000-0000-000000000000",
            Some("read")
        )
        .await,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn credential_http_denies_read_scope_and_never_echoes_secret_on_errors() {
    let (app, _) = fixture();
    let path = format!("/api/v1/connections/{}/credentials", Uuid::new_v4());
    let input = serde_json::json!({"secret":"synthetic-http-credential-canary","credential_type":"api_key","expected_generation":1});
    for (token, body, expected) in [
        ("read", input.clone(), StatusCode::FORBIDDEN),
        ("foreign", input.clone(), StatusCode::FORBIDDEN),
        (
            "write",
            {
                let mut p = input.clone();
                p["expected_generation"] = serde_json::json!(0);
                p
            },
            StatusCode::BAD_REQUEST,
        ),
        (
            "write",
            {
                let mut p = input.clone();
                p["credential_type"] = serde_json::json!("managed_token");
                p
            },
            StatusCode::BAD_REQUEST,
        ),
        (
            "write",
            {
                let mut p = input.clone();
                p["secret"] = serde_json::json!("synthetic-http-credential-canary\n");
                p
            },
            StatusCode::BAD_REQUEST,
        ),
        ("write", input.clone(), StatusCode::SERVICE_UNAVAILABLE),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(&path)
                    .header("authorization", format!("Bearer {token}"))
                    .header("idempotency-key", Uuid::new_v4().to_string())
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        let bytes = axum::body::to_bytes(response.into_body(), 65536)
            .await
            .unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-http-credential-canary"));
    }
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&path)
                .header("authorization", "Bearer write")
                .header("idempotency-key", Uuid::new_v4().to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "revocation requires fresh strong If-Match"
    );
}

#[tokio::test]
async fn catalog_cache_requires_read_scope_and_rejects_broadening_filters() {
    let (app, _) = fixture();
    let path = format!("/api/v1/connections/{}/models", Uuid::new_v4());
    assert_eq!(
        status(&app, &path, Some("write")).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        status(&app, &path, Some("foreign")).await,
        StatusCode::FORBIDDEN
    );
    for query in ["q=one&q=two", "limit=0", "namespace_id=bad", "cursor=bad"] {
        assert_eq!(
            status(&app, &format!("{path}?{query}"), Some("read")).await,
            StatusCode::BAD_REQUEST
        )
    }
    assert_eq!(
        status(
            &app,
            &format!("{path}?q=vendor%2Fmodel&limit=100"),
            Some("read")
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}
