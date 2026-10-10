use aihub_application::Foundation;
use aihub_domain::{
    NamespaceRef,
    access::{HumanPrincipal, namespace_filter},
    error::HubError,
    records::{AuditEvent, Health, Identity, NamespaceBinding, Operation},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, FromRequestParts, Path, RawQuery, State},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub foundation: Foundation,
    pub external_calls: bool,
    pub auth_issuer: String,
    pub public_origin: String,
    pub admin_origin: Option<String>,
}

pub struct Authenticated(pub HumanPrincipal);
impl FromRequestParts<AppState> for Authenticated {
    type Rejection = ApiError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let mut values = parts.headers.get_all("authorization").iter();
        let header = values
            .next()
            .ok_or(HubError::Unauthenticated)?
            .to_str()
            .map_err(|_| HubError::Unauthenticated)?;
        if values.next().is_some() {
            return Err(HubError::Unauthenticated.into());
        }
        let token = header
            .strip_prefix("Bearer ")
            .filter(|v| !v.is_empty() && v.len() <= 16384)
            .ok_or(HubError::Unauthenticated)?;
        Ok(Self(state.foundation.auth.authenticate(token).await?))
    }
}

#[derive(Serialize, ToSchema)]
pub struct Error {
    pub error: ErrorDetail,
}
#[derive(Serialize, ToSchema)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    pub request_id: Uuid,
}
pub struct ApiError(pub HubError);
impl From<HubError> for ApiError {
    fn from(value: HubError) -> Self {
        Self(value)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match self.0 {
            HubError::Unauthenticated => (StatusCode::UNAUTHORIZED, "unauthenticated"),
            HubError::Forbidden => (StatusCode::FORBIDDEN, "permission_denied"),
            HubError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            HubError::PreconditionFailed => {
                (StatusCode::PRECONDITION_FAILED, "precondition_failed")
            }
            HubError::IdempotencyConflict => (StatusCode::CONFLICT, "idempotency_conflict"),
            HubError::BudgetExceeded => (StatusCode::TOO_MANY_REQUESTS, "budget_exceeded"),
            HubError::Invalid(_) => (StatusCode::BAD_REQUEST, "invalid_request"),
            HubError::Unavailable | HubError::InstallationMismatch => {
                (StatusCode::SERVICE_UNAVAILABLE, "unavailable")
            }
        };
        let request_id = Uuid::new_v4();
        (
            status,
            [("x-request-id", request_id.to_string())],
            Json(Error {
                error: ErrorDetail {
                    code: code.into(),
                    message: self.0.to_string(),
                    request_id,
                },
            }),
        )
            .into_response()
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct NamespacePage {
    pub items: Vec<NamespaceBinding>,
    pub next_cursor: Option<String>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AuditPage {
    pub items: Vec<AuditEvent>,
    pub next_cursor: Option<String>,
}

fn list_options(raw: &str, namespace_allowed: bool) -> Result<(i64, Option<Uuid>), HubError> {
    let mut limit = None;
    let mut cursor = None;
    for (key, value) in url::form_urlencoded::parse(raw.as_bytes()) {
        match key.as_ref() {
            "limit" if limit.is_none() => {
                limit = Some(
                    value
                        .parse::<i64>()
                        .map_err(|_| HubError::Invalid("limit"))?,
                );
            }
            "registry_instance_id" | "namespace_id" if namespace_allowed => (),
            "cursor" if cursor.is_none() => {
                cursor = Some(Uuid::parse_str(&value).map_err(|_| HubError::Invalid("cursor"))?);
            }
            _ => {
                return Err(HubError::Invalid(
                    "неподдержанный или повторяющийся query parameter",
                ));
            }
        }
    }
    Ok((
        aihub_application::bounded_limit(limit.unwrap_or(50))?,
        cursor,
    ))
}

#[utoipa::path(get, path="/health/live", operation_id="healthLive", responses((status=200,body=Health)))]
pub async fn live() -> Json<Health> {
    Json(Health {
        status: "ok".into(),
        schema_revision: None,
    })
}

#[utoipa::path(get, path="/health/ready", operation_id="healthReady", responses((status=200,body=Health),(status=503,body=Health)))]
pub async fn ready(State(state): State<AppState>) -> (StatusCode, Json<Health>) {
    let healthy = state.foundation.store.ready().await.is_ok();
    (
        if healthy {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        Json(Health {
            status: if healthy { "ok" } else { "not_ready" }.into(),
            schema_revision: healthy.then(|| env!("AIHUB_SCHEMA_REVISION").into()),
        }),
    )
}

#[utoipa::path(get,path="/api/v1/auth/me",operation_id="readIdentity",security(("CentralAuth"=[])),responses((status=200,body=Identity),(status=401,body=Error),(status=403,body=Error),(status=503,body=Error)))]
pub async fn identity(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
) -> Result<Json<Identity>, ApiError> {
    Ok(Json(state.foundation.identity(&principal).await?))
}

#[utoipa::path(get,path="/api/v1/namespaces",operation_id="listNamespaceBindings",security(("CentralAuth"=[])),params(("limit"=Option<i64>,Query,minimum=1,maximum=100),("cursor"=Option<Uuid>,Query)),responses((status=200,body=NamespacePage),(status=400,body=Error),(status=403,body=Error)))]
pub async fn namespaces(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    RawQuery(raw): RawQuery,
) -> Result<Json<NamespacePage>, ApiError> {
    let (limit, cursor) = list_options(raw.as_deref().unwrap_or(""), false)?;
    let page = state
        .foundation
        .namespaces(&principal, limit, cursor)
        .await?;
    Ok(Json(NamespacePage {
        items: page.items,
        next_cursor: page.next_cursor,
    }))
}

#[utoipa::path(get,path="/api/v1/audit",operation_id="listAudit",security(("CentralAuth"=[])),params(("limit"=Option<i64>,Query),("cursor"=Option<Uuid>,Query),("registry_instance_id"=Option<Uuid>,Query),("namespace_id"=Option<Uuid>,Query)),responses((status=200,body=AuditPage),(status=400,body=Error),(status=403,body=Error)))]
pub async fn audit(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    RawQuery(raw): RawQuery,
) -> Result<Json<AuditPage>, ApiError> {
    let raw = raw.as_deref().unwrap_or("");
    let (limit, cursor) = list_options(raw, true)?;
    let namespace = namespace_filter(raw)?;
    let page = state
        .foundation
        .audit(&principal, namespace.as_ref(), limit, cursor)
        .await?;
    Ok(Json(AuditPage {
        items: page.items,
        next_cursor: page.next_cursor,
    }))
}

#[utoipa::path(get,path="/api/v1/operations/{operation_id}",operation_id="readOperation",security(("CentralAuth"=[])),params(("operation_id"=Uuid,Path)),responses((status=200,body=Operation),(status=404,body=Error)))]
pub async fn operation(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(id): Path<Uuid>,
) -> Result<Json<Operation>, ApiError> {
    Ok(Json(state.foundation.operation(&principal, id).await?))
}

#[derive(Serialize, ToSchema)]
pub struct Version {
    pub service_key: String,
    pub installation_id: Uuid,
    pub source_revision: String,
    pub source_dirty: bool,
    pub base_revision: String,
    pub namespace_base_revision: String,
    pub external_calls: bool,
    pub schema_revision: String,
}
#[utoipa::path(get,path="/version",operation_id="readVersion",responses((status=200,body=Version)))]
pub async fn version(State(state): State<AppState>) -> Json<Version> {
    Json(Version {
        service_key: "ai-hub".into(),
        installation_id: state.foundation.installation_id,
        source_revision: env!("AIHUB_SOURCE_REVISION").into(),
        source_dirty: env!("AIHUB_SOURCE_DIRTY") == "true",
        base_revision: include_str!("../../../../.base-revision").trim().into(),
        namespace_base_revision: include_str!("../../../../.namespace-base-revision")
            .trim()
            .into(),
        external_calls: state.external_calls,
        schema_revision: env!("AIHUB_SCHEMA_REVISION").into(),
    })
}

#[derive(Serialize, ToSchema)]
pub struct PublicConfig {
    pub auth_issuer: String,
    pub oidc_client_id: String,
    pub installation_id: Uuid,
    pub branding_url: Option<String>,
    pub service_catalog_url: Option<String>,
}
#[utoipa::path(get,path="/api/v1/public/config",operation_id="readPublicConfig",responses((status=200,body=PublicConfig)))]
pub async fn public_config(State(state): State<AppState>) -> Json<PublicConfig> {
    Json(PublicConfig {
        auth_issuer: state.auth_issuer,
        oidc_client_id: "ai-hub".into(),
        installation_id: state.foundation.installation_id,
        branding_url: state
            .admin_origin
            .as_ref()
            .map(|origin| format!("{}/api/v1/runtime/branding", origin.trim_end_matches('/'))),
        service_catalog_url: state
            .admin_origin
            .as_ref()
            .map(|origin| format!("{}/api/v1/runtime/services", origin.trim_end_matches('/'))),
    })
}

#[derive(Serialize, ToSchema)]
pub struct IntegrationStatus {
    pub service_key: String,
    pub contract_version: String,
    pub status: String,
    pub source_revision: String,
    pub base_revision: String,
}
#[utoipa::path(get,path="/integration/status",operation_id="readIntegrationStatus",responses((status=200,body=IntegrationStatus),(status=503,body=IntegrationStatus)))]
pub async fn integration_status(
    State(state): State<AppState>,
) -> (StatusCode, Json<IntegrationStatus>) {
    let healthy = state.foundation.store.ready().await.is_ok();
    (
        if healthy {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        Json(IntegrationStatus {
            service_key: "ai-hub".into(),
            contract_version: "1.0".into(),
            status: if healthy { "ready" } else { "not_ready" }.into(),
            source_revision: env!("AIHUB_SOURCE_REVISION").into(),
            base_revision: include_str!("../../../../.base-revision").trim().into(),
        }),
    )
}
#[derive(Serialize, ToSchema)]
pub struct BrandingContract {
    pub schema_version: u8,
    pub service_key: String,
    pub central_branding_consumer: bool,
    pub local_fallback: bool,
}
#[utoipa::path(get,path="/branding/contract",operation_id="readBrandingContract",responses((status=200,body=BrandingContract)))]
pub async fn branding_contract() -> Json<BrandingContract> {
    Json(BrandingContract {
        schema_version: 1,
        service_key: "ai-hub".into(),
        central_branding_consumer: true,
        local_fallback: true,
    })
}

#[derive(OpenApi)]
#[openapi(info(title="AI Hub — implemented API",version="0.1.0-dev"),paths(live,ready,identity,namespaces,audit,operation,version,public_config,integration_status,branding_contract),components(schemas(Health,Identity,NamespaceRef,NamespaceBinding,AuditEvent,Operation,Error,ErrorDetail,NamespacePage,AuditPage,Version,PublicConfig,IntegrationStatus,BrandingContract)),modifiers(&SecurityAddon))]
pub struct ApiDoc;
struct SecurityAddon;
impl utoipa::Modify for SecurityAddon {
    fn modify(&self, doc: &mut utoipa::openapi::OpenApi) {
        if let Some(c) = doc.components.as_mut() {
            c.add_security_scheme(
                "CentralAuth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::Http::new(
                        utoipa::openapi::security::HttpAuthScheme::Bearer,
                    ),
                ),
            );
        }
    }
}

async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
async fn not_found() -> ApiError {
    HubError::NotFound.into()
}

pub fn router(state: AppState, body_limit: usize) -> Router {
    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .route("/health", get(ready))
        .route("/version", get(version))
        .route("/integration/status", get(integration_status))
        .route("/branding/contract", get(branding_contract))
        .route("/api/v1/public/config", get(public_config))
        .route("/api/v1/auth/me", get(identity))
        .route("/api/v1/namespaces", get(namespaces))
        .route("/api/v1/audit", get(audit))
        .route("/api/v1/operations/{operation_id}", get(operation))
        .route("/openapi.json", get(openapi))
        .fallback(not_found)
        .layer(DefaultBodyLimit::max(body_limit))
        .with_state(state)
}
