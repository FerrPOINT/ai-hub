use aihub_application::{BudgetFilter, Foundation};
use aihub_domain::model_context::{ModelContextInput, ModelContextPreference};
use aihub_domain::{
    NamespaceRef,
    access::{HumanPrincipal, namespace_filter},
    budgets::{Budget, BudgetInput, BudgetPeriod, BudgetScope},
    catalog::{CatalogPage, MetadataRefreshInput, ModelMetadata},
    connections::{
        BillingMode, Connection, ConnectionInput, CredentialInput, CredentialType,
        EndpointPolicyInput, ProviderKind,
    },
    error::HubError,
    prices::{PriceInput, PriceRevision, PriceUnit},
    pricing_sources::{PricingDataStatus, PricingMode, PricingSourceInput, PricingSourceRevision},
    records::{AuditEvent, Health, Identity, NamespaceBinding, Operation, OperationLookup},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, FromRequestParts, Path, RawQuery, State},
    http::{HeaderMap, StatusCode, request::Parts},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub foundation: Foundation,
    pub metadata: std::sync::Arc<dyn aihub_application::MetadataOperations>,
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
            HubError::InvalidSemantics(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "invalid_semantics")
            }
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

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PricePage {
    pub items: Vec<PriceRevision>,
    pub next_cursor: Option<String>,
}
#[derive(Serialize, ToSchema)]
pub struct PricingSourcePage {
    pub items: Vec<PricingSourceRevision>,
    pub next_cursor: Option<String>,
}
#[derive(Serialize, ToSchema)]
pub struct ConnectionPage {
    pub items: Vec<Connection>,
    pub next_cursor: Option<String>,
}
#[utoipa::path(get,path="/api/v1/connections",operation_id="listConnections",security(("CentralAuth"=[])),params(("limit"=Option<i64>,Query,minimum=1,maximum=100),("cursor"=Option<Uuid>,Query)),responses((status=200,body=ConnectionPage),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=503,body=Error)))]
pub async fn connections(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    RawQuery(query): RawQuery,
) -> Result<Json<ConnectionPage>, ApiError> {
    let (limit, cursor) = list_options(query.as_deref().unwrap_or(""), false)?;
    let page = state
        .foundation
        .connections(&principal, limit, cursor)
        .await?;
    Ok(Json(ConnectionPage {
        items: page.items,
        next_cursor: page.next_cursor,
    }))
}
#[utoipa::path(get,path="/api/v1/connections/{connection_id}",operation_id="readConnection",security(("CentralAuth"=[])),params(("connection_id"=Uuid,Path)),responses((status=200,body=Connection),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=503,body=Error)))]
pub async fn read_connection(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(id): Path<String>,
) -> Result<([(String, String); 1], Json<Connection>), ApiError> {
    let id = Uuid::parse_str(&id).map_err(|_| HubError::Invalid("connection ID"))?;
    let value = state.foundation.connection(&principal, id).await?;
    Ok((
        [("etag".into(), format!("\"{}\"", value.version))],
        Json(value),
    ))
}
type ConnectionResponse = (StatusCode, [(String, String); 2], Json<Connection>);
fn connection_response(
    result: aihub_domain::connections::ConnectionMutation,
) -> ConnectionResponse {
    (
        StatusCode::CREATED,
        [
            ("etag".into(), format!("\"{}\"", result.value.version)),
            ("x-operation-id".into(), result.operation_id.to_string()),
        ],
        Json(result.value),
    )
}
#[utoipa::path(post,path="/api/v1/providers/{provider}/connections",operation_id="createConnection",security(("CentralAuth"=[])),params(("provider"=String,Path),("Idempotency-Key"=Uuid,Header)),request_body=ConnectionInput,responses((status=201,body=Connection),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=409,body=Error),(status=422,body=Error),(status=503,body=Error)))]
pub async fn create_connection(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(provider): Path<String>,
    headers: HeaderMap,
    body: Result<Json<ConnectionInput>, axum::extract::rejection::JsonRejection>,
) -> Result<ConnectionResponse, ApiError> {
    principal.require_config(true)?;
    let key = mutation_key(&headers)?;
    let kind = ProviderKind::parse(&provider)?;
    let input = body.map_err(|_| HubError::Invalid("connection payload"))?.0;
    Ok(connection_response(
        state
            .foundation
            .save_connection(&principal, key, kind, None, &input)
            .await?,
    ))
}
#[utoipa::path(patch,path="/api/v1/connections/{connection_id}",operation_id="updateConnection",security(("CentralAuth"=[])),params(("connection_id"=Uuid,Path),("Idempotency-Key"=Uuid,Header),("If-Match"=String,Header)),request_body=ConnectionInput,responses((status=201,body=Connection),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=409,body=Error),(status=412,body=Error),(status=422,body=Error),(status=503,body=Error)))]
pub async fn update_connection(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Result<Json<ConnectionInput>, axum::extract::rejection::JsonRejection>,
) -> Result<ConnectionResponse, ApiError> {
    principal.require_config(true)?;
    let key = mutation_key(&headers)?;
    let version = budget_version(&headers)?;
    let id = Uuid::parse_str(&id).map_err(|_| HubError::Invalid("connection ID"))?;
    let input = body.map_err(|_| HubError::Invalid("connection payload"))?.0;
    Ok(connection_response(
        state
            .foundation
            .update_connection(&principal, key, id, version, &input)
            .await?,
    ))
}
#[utoipa::path(delete,path="/api/v1/connections/{connection_id}",operation_id="disableConnection",security(("CentralAuth"=[])),params(("connection_id"=Uuid,Path),("Idempotency-Key"=Uuid,Header),("If-Match"=String,Header)),responses((status=202,body=Operation),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=409,body=Error),(status=412,body=Error),(status=503,body=Error)))]
pub async fn disable_connection(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<Operation>), ApiError> {
    principal.require_config(true)?;
    let key = mutation_key(&headers)?;
    let version = budget_version(&headers)?;
    let id = Uuid::parse_str(&id).map_err(|_| HubError::Invalid("connection ID"))?;
    Ok((
        StatusCode::ACCEPTED,
        Json(
            state
                .foundation
                .disable_connection(&principal, key, id, version)
                .await?,
        ),
    ))
}
#[utoipa::path(get,path="/api/v1/pricing-sources",operation_id="listPricingSources",security(("CentralAuth"=[])),params(("limit"=Option<i64>,Query,minimum=1,maximum=100),("cursor"=Option<Uuid>,Query)),responses((status=200,body=PricingSourcePage),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=503,body=Error)))]
pub async fn pricing_sources(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    RawQuery(query): RawQuery,
) -> Result<Json<PricingSourcePage>, ApiError> {
    let (limit, cursor) = list_options(query.as_deref().unwrap_or(""), false)?;
    let page = state
        .foundation
        .pricing_sources(&principal, limit, cursor)
        .await?;
    Ok(Json(PricingSourcePage {
        items: page.items,
        next_cursor: page.next_cursor,
    }))
}
#[utoipa::path(post,path="/api/v1/pricing-sources",operation_id="createPricingSourceRevision",security(("CentralAuth"=[])),params(("Idempotency-Key"=Uuid,Header)),request_body=PricingSourceInput,responses((status=201,body=PricingSourceRevision),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=409,body=Error),(status=412,body=Error),(status=422,body=Error),(status=503,body=Error)))]
pub async fn create_pricing_source(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    headers: HeaderMap,
    body: Result<Json<PricingSourceInput>, axum::extract::rejection::JsonRejection>,
) -> Result<
    (
        StatusCode,
        [(String, String); 1],
        Json<PricingSourceRevision>,
    ),
    ApiError,
> {
    principal.require_config(true)?;
    let key = mutation_key(&headers)?;
    let input = body
        .map_err(|_| HubError::Invalid("pricing source payload"))?
        .0;
    let result = state
        .foundation
        .create_pricing_source(&principal, key, &input)
        .await?;
    Ok((
        StatusCode::CREATED,
        [("x-operation-id".into(), result.operation_id.to_string())],
        Json(result.value),
    ))
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct BudgetPage {
    pub items: Vec<Budget>,
    pub next_cursor: Option<String>,
}

fn budget_options(raw: &str) -> Result<(BudgetFilter, i64, Option<Uuid>), HubError> {
    let namespace = namespace_filter(raw)?;
    let mut binding = None;
    let mut rest = vec![];
    for (key, value) in url::form_urlencoded::parse(raw.as_bytes()) {
        if key == "binding" {
            if binding.is_some() || !matches!(value.as_ref(), "all" | "unbound") {
                return Err(HubError::Invalid("binding filter"));
            }
            binding = Some(value.into_owned());
        } else {
            rest.push((key.into_owned(), value.into_owned()));
        }
    }
    let mut encoded = url::form_urlencoded::Serializer::new(String::new());
    encoded.extend_pairs(rest);
    let (limit, cursor) = list_options(&encoded.finish(), true)?;
    let filter = BudgetFilter {
        namespace,
        unbound_only: binding.as_deref() == Some("unbound"),
    };
    if filter.unbound_only && filter.namespace.is_some() {
        return Err(HubError::Invalid("binding filter"));
    }
    Ok((filter, limit, cursor))
}

fn budget_version(headers: &HeaderMap) -> Result<i64, HubError> {
    quoted_version(headers, 1)
}
fn quoted_version(headers: &HeaderMap, minimum: i64) -> Result<i64, HubError> {
    let mut values = headers.get_all("if-match").iter();
    let raw = values
        .next()
        .ok_or(HubError::Invalid("If-Match required"))?
        .to_str()
        .map_err(|_| HubError::Invalid("If-Match"))?;
    if values.next().is_some() {
        return Err(HubError::Invalid("duplicate If-Match"));
    }
    let raw = raw
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .filter(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
        .ok_or(HubError::Invalid("strong quoted If-Match version"))?;
    let version = raw
        .parse::<i64>()
        .map_err(|_| HubError::Invalid("If-Match version"))?;
    if version < minimum {
        return Err(HubError::Invalid("If-Match version"));
    }
    Ok(version)
}
#[derive(Serialize, ToSchema)]
pub struct ModelContextPage {
    items: Vec<ModelContextPreference>,
    next_cursor: Option<String>,
}
#[utoipa::path(get,path="/api/v1/connections/{connection_id}/model-contexts",operation_id="listModelContextPreferences",security(("CentralAuth"=[])),params(("connection_id"=Uuid,Path),("model_id"=Option<String>,Query,min_length=1,max_length=256),("limit"=Option<i64>,Query,minimum=1,maximum=100),("cursor"=Option<Uuid>,Query)),responses((status=200,body=ModelContextPage),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=503,body=Error)))]
pub async fn model_contexts(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(id): Path<String>,
    RawQuery(raw): RawQuery,
) -> Result<(HeaderMap, Json<ModelContextPage>), ApiError> {
    principal.require_config(false)?;
    let id = Uuid::parse_str(&id).map_err(|_| HubError::Invalid("connection ID"))?;
    let (model, limit, cursor) = model_context_options(raw.as_deref().unwrap_or(""))?;
    let page = state
        .foundation
        .model_contexts(&principal, id, model.as_deref(), limit, cursor)
        .await?;
    let mut headers = HeaderMap::new();
    if model.is_some() {
        headers.insert(
            "etag",
            axum::http::HeaderValue::from_str(&format!(
                "\"{}\"",
                page.items.first().map_or(0, |p| p.version)
            ))
            .map_err(|_| HubError::Unavailable)?,
        );
    }
    Ok((
        headers,
        Json(ModelContextPage {
            items: page.items,
            next_cursor: page.next_cursor,
        }),
    ))
}
fn model_context_options(raw: &str) -> Result<(Option<String>, i64, Option<Uuid>), HubError> {
    let mut model = None;
    let mut encoded = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in url::form_urlencoded::parse(raw.as_bytes()) {
        if key == "model_id" {
            if model.is_some() {
                return Err(HubError::Invalid("duplicate model ID"));
            }
            aihub_domain::model_context::validate_model_id(&value)?;
            model = Some(value.into_owned())
        } else {
            encoded.append_pair(&key, &value);
        }
    }
    let (limit, cursor) = list_options(&encoded.finish(), false)?;
    if model.is_some() && cursor.is_some() {
        return Err(HubError::Invalid("exact context cursor"));
    }
    Ok((model, limit, cursor))
}
#[utoipa::path(put,path="/api/v1/connections/{connection_id}/model-contexts",operation_id="saveModelContextPreference",security(("CentralAuth"=[])),params(("connection_id"=Uuid,Path),("Idempotency-Key"=Uuid,Header),("If-Match"=String,Header)),request_body=ModelContextInput,responses((status=200,body=ModelContextPreference),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=409,body=Error),(status=412,body=Error),(status=503,body=Error)))]
pub async fn save_model_context(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Result<Json<ModelContextInput>, axum::extract::rejection::JsonRejection>,
) -> Result<([(String, String); 2], Json<ModelContextPreference>), ApiError> {
    principal.require_config(true)?;
    let key = mutation_key(&headers)?;
    let expected = quoted_version(&headers, 0)?;
    let id = Uuid::parse_str(&id).map_err(|_| HubError::Invalid("connection ID"))?;
    let input = body
        .map_err(|_| HubError::Invalid("model context payload"))?
        .0;
    let result = state
        .foundation
        .save_model_context(&principal, key, id, expected, &input)
        .await?;
    Ok((
        [
            ("etag".into(), format!("\"{}\"", result.preference.version)),
            ("x-operation-id".into(), result.operation_id.to_string()),
        ],
        Json(result.preference),
    ))
}

#[utoipa::path(get,path="/api/v1/budgets",operation_id="listBudgets",security(("CentralAuth"=[])),params(("limit"=Option<i64>,Query,minimum=1,maximum=100),("cursor"=Option<Uuid>,Query),("registry_instance_id"=Option<Uuid>,Query),("namespace_id"=Option<Uuid>,Query),("binding"=Option<String>,Query)),responses((status=200,body=BudgetPage),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=503,body=Error)))]
pub async fn budgets(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    RawQuery(query): RawQuery,
) -> Result<Json<BudgetPage>, ApiError> {
    let (filter, limit, cursor) = budget_options(query.as_deref().unwrap_or(""))?;
    let page = state
        .foundation
        .budgets(&principal, &filter, limit, cursor)
        .await?;
    Ok(Json(BudgetPage {
        items: page.items,
        next_cursor: page.next_cursor,
    }))
}

type BudgetResponse = (StatusCode, [(String, String); 2], Json<Budget>);
fn budget_response(result: aihub_domain::budgets::BudgetMutation) -> BudgetResponse {
    (
        StatusCode::CREATED,
        [
            ("x-operation-id".into(), result.operation_id.to_string()),
            ("etag".into(), format!("\"{}\"", result.value.version)),
        ],
        Json(result.value),
    )
}
#[utoipa::path(post,path="/api/v1/budgets",operation_id="createBudget",security(("CentralAuth"=[])),params(("Idempotency-Key"=Uuid,Header)),request_body=BudgetInput,responses((status=201,body=Budget),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=409,body=Error),(status=412,body=Error),(status=503,body=Error)))]
pub async fn create_budget(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    headers: HeaderMap,
    body: Result<Json<BudgetInput>, axum::extract::rejection::JsonRejection>,
) -> Result<BudgetResponse, ApiError> {
    principal.require_config(true)?;
    let key = mutation_key(&headers)?;
    let policy = body.map_err(|_| HubError::Invalid("budget payload"))?.0;
    Ok(budget_response(
        state
            .foundation
            .write_budget(&principal, key, None, &policy)
            .await?,
    ))
}
#[utoipa::path(patch,path="/api/v1/budgets/{budget_id}",operation_id="updateBudget",security(("CentralAuth"=[])),params(("budget_id"=Uuid,Path),("Idempotency-Key"=Uuid,Header),("If-Match"=String,Header)),request_body=BudgetInput,responses((status=201,body=Budget),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=409,body=Error),(status=412,body=Error),(status=503,body=Error)))]
pub async fn update_budget(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Result<Json<BudgetInput>, axum::extract::rejection::JsonRejection>,
) -> Result<BudgetResponse, ApiError> {
    principal.require_config(true)?;
    let key = mutation_key(&headers)?;
    let version = budget_version(&headers)?;
    let id = Uuid::parse_str(&id).map_err(|_| HubError::Invalid("budget UUID"))?;
    let policy = body.map_err(|_| HubError::Invalid("budget payload"))?.0;
    Ok(budget_response(
        state
            .foundation
            .write_budget(&principal, key, Some((id, version)), &policy)
            .await?,
    ))
}

fn mutation_key(headers: &HeaderMap) -> Result<Uuid, HubError> {
    let mut values = headers.get_all("idempotency-key").iter();
    let raw = values
        .next()
        .ok_or(HubError::Invalid("Idempotency-Key required"))?;
    if values.next().is_some() {
        return Err(HubError::Invalid("duplicate Idempotency-Key"));
    }
    let key = Uuid::parse_str(
        raw.to_str()
            .map_err(|_| HubError::Invalid("Idempotency-Key"))?,
    )
    .map_err(|_| HubError::Invalid("Idempotency-Key"))?;
    if key.is_nil() {
        return Err(HubError::Invalid("nil Idempotency-Key"));
    }
    Ok(key)
}

#[utoipa::path(get,path="/api/v1/prices",operation_id="listPrices",security(("CentralAuth"=[])),params(("limit"=Option<i64>,Query,minimum=1,maximum=100),("cursor"=Option<Uuid>,Query)),responses((status=200,body=PricePage),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=503,body=Error)))]
pub async fn prices(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    RawQuery(query): RawQuery,
) -> Result<Json<PricePage>, ApiError> {
    let (limit, cursor) = list_options(query.as_deref().unwrap_or(""), false)?;
    let page = state.foundation.prices(&principal, limit, cursor).await?;
    Ok(Json(PricePage {
        items: page.items,
        next_cursor: page.next_cursor,
    }))
}

#[utoipa::path(post,path="/api/v1/prices",operation_id="createPriceRevision",security(("CentralAuth"=[])),params(("Idempotency-Key"=Uuid,Header,description="UUID operation key")),request_body=PriceInput,responses((status=201,body=PriceRevision),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=409,body=Error),(status=503,body=Error)))]
pub async fn create_price(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    headers: HeaderMap,
    body: Result<Json<PriceInput>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, [(String, String); 1], Json<PriceRevision>), ApiError> {
    principal.require_config(true)?;
    let key = mutation_key(&headers)?;
    let price = body.map_err(|_| HubError::Invalid("price payload"))?.0;
    let result = state
        .foundation
        .create_price(&principal, key, &price)
        .await?;
    Ok((
        StatusCode::CREATED,
        [("x-operation-id".into(), result.operation_id.to_string())],
        Json(result.value),
    ))
}

#[utoipa::path(put,path="/api/v1/connections/{connection_id}/credentials",operation_id="writeCredential",security(("CentralAuth"=[])),params(("connection_id"=Uuid,Path),("Idempotency-Key"=Uuid,Header)),request_body=CredentialInput,responses((status=202,body=Operation),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=409,body=Error),(status=412,body=Error),(status=422,body=Error),(status=503,body=Error)))]
pub async fn write_credential(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Result<Json<CredentialInput>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<Operation>), ApiError> {
    principal.require_config(true)?;
    let key = mutation_key(&headers)?;
    let id = Uuid::parse_str(&id).map_err(|_| HubError::Invalid("connection ID"))?;
    let input = body.map_err(|_| HubError::Invalid("credential payload"))?.0;
    Ok((
        StatusCode::ACCEPTED,
        Json(
            state
                .foundation
                .write_credential(&principal, key, id, &input)
                .await?,
        ),
    ))
}
#[utoipa::path(delete,path="/api/v1/connections/{connection_id}/credentials",operation_id="revokeConnectionAuthorization",security(("CentralAuth"=[])),params(("connection_id"=Uuid,Path),("Idempotency-Key"=Uuid,Header),("If-Match"=String,Header)),responses((status=202,body=Operation),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=409,body=Error),(status=412,body=Error),(status=503,body=Error)))]
pub async fn revoke_credential(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<Operation>), ApiError> {
    principal.require_config(true)?;
    let key = mutation_key(&headers)?;
    let version = budget_version(&headers)?;
    let id = Uuid::parse_str(&id).map_err(|_| HubError::Invalid("connection ID"))?;
    Ok((
        StatusCode::ACCEPTED,
        Json(
            state
                .foundation
                .revoke_credential(&principal, key, id, version)
                .await?,
        ),
    ))
}
#[utoipa::path(get,path="/api/v1/connections/{connection_id}/models",operation_id="readModelCatalog",security(("CentralAuth"=[])),params(("connection_id"=Uuid,Path),("q"=Option<String>,Query),("limit"=Option<i64>,Query,minimum=1,maximum=100),("cursor"=Option<Uuid>,Query)),responses((status=200,body=CatalogPage),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=503,body=Error)))]
pub async fn read_catalog(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(id): Path<String>,
    RawQuery(raw): RawQuery,
) -> Result<Json<CatalogPage>, ApiError> {
    let id = Uuid::parse_str(&id).map_err(|_| HubError::Invalid("connection ID"))?;
    let (query, limit, cursor) = catalog_options(raw.as_deref().unwrap_or(""))?;
    Ok(Json(
        state
            .foundation
            .catalog(&principal, id, &query, limit, cursor)
            .await?,
    ))
}
fn catalog_options(raw: &str) -> Result<(String, i64, Option<Uuid>), HubError> {
    let mut query = None;
    let mut rest = vec![];
    for (key, value) in url::form_urlencoded::parse(raw.as_bytes()) {
        if key == "q" {
            if query.is_some() {
                return Err(HubError::Invalid("duplicate catalog query"));
            }
            query = Some(value.into_owned())
        } else {
            rest.push((key.into_owned(), value.into_owned()))
        }
    }
    let mut encoded = url::form_urlencoded::Serializer::new(String::new());
    encoded.extend_pairs(rest);
    let (limit, cursor) = list_options(&encoded.finish(), false)?;
    Ok((query.unwrap_or_default(), limit, cursor))
}
#[utoipa::path(post,path="/api/v1/connections/{connection_id}/models/refresh",operation_id="refreshModelCatalog",security(("CentralAuth"=[])),params(("connection_id"=Uuid,Path),("Idempotency-Key"=Uuid,Header)),request_body=MetadataRefreshInput,responses((status=202,body=Operation),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=409,body=Error),(status=412,body=Error),(status=422,body=Error),(status=503,body=Error)))]
pub async fn refresh_catalog(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Result<Json<MetadataRefreshInput>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<Operation>), ApiError> {
    principal.require_config(true)?;
    if !state.external_calls {
        return Err(HubError::Unavailable.into());
    }
    let key = mutation_key(&headers)?;
    let id = Uuid::parse_str(&id).map_err(|_| HubError::Invalid("connection ID"))?;
    let input = body.map_err(|_| HubError::Invalid("metadata payload"))?.0;
    if input.expected_generation < 1 {
        return Err(HubError::Invalid("connection generation").into());
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(
            state
                .metadata
                .refresh(&principal, key, id, input.expected_generation)
                .await?,
        ),
    ))
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
#[utoipa::path(get,path="/api/v1/operations/by-key/{idempotency_key}",operation_id="readOperationByKey",security(("CentralAuth"=[])),params(("idempotency_key"=Uuid,Path)),responses((status=200,body=OperationLookup),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=503,body=Error)))]
pub async fn operation_key(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(key): Path<String>,
) -> Result<Json<OperationLookup>, ApiError> {
    let key = Uuid::parse_str(&key).map_err(|_| HubError::Invalid("operation key"))?;
    Ok(Json(state.foundation.operation_key(&principal, key).await?))
}
#[utoipa::path(post,path="/api/v1/operations/by-key/{idempotency_key}/close-unstarted",operation_id="closeUnstartedOperation",security(("CentralAuth"=[])),params(("idempotency_key"=Uuid,Path)),responses((status=200,body=OperationLookup),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=503,body=Error)))]
pub async fn close_unstarted_operation(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(key): Path<String>,
) -> Result<Json<OperationLookup>, ApiError> {
    principal.require_config(true)?;
    let key = Uuid::parse_str(&key).map_err(|_| HubError::Invalid("operation key"))?;
    Ok(Json(
        state
            .foundation
            .close_unstarted_operation(&principal, key)
            .await?,
    ))
}
#[derive(Serialize, ToSchema)]
pub struct EndpointPolicyPage {
    items: Vec<EndpointPolicyInput>,
    next_cursor: Option<String>,
}
#[utoipa::path(get,path="/api/v1/connection-presets",operation_id="listConnectionPresets",security(("CentralAuth"=[])),params(("limit"=Option<i64>,Query,minimum=1,maximum=100),("cursor"=Option<Uuid>,Query)),responses((status=200,body=EndpointPolicyPage),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=503,body=Error)))]
pub async fn endpoint_policies(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    RawQuery(raw): RawQuery,
) -> Result<Json<EndpointPolicyPage>, ApiError> {
    let (limit, cursor) = list_options(raw.as_deref().unwrap_or(""), false)?;
    let page = state
        .foundation
        .endpoint_policies(&principal, limit, cursor)
        .await?;
    Ok(Json(EndpointPolicyPage {
        items: page.items,
        next_cursor: page.next_cursor,
    }))
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
#[openapi(info(title="AI Hub — implemented API",version="0.1.0-dev"),paths(live,ready,identity,namespaces,audit,operation,version,public_config,integration_status,branding_contract,prices,create_price,budgets,create_budget,update_budget,pricing_sources,create_pricing_source,connections,read_connection,create_connection,update_connection,disable_connection,write_credential,revoke_credential,read_catalog,refresh_catalog,operation_key,close_unstarted_operation,endpoint_policies,model_contexts,save_model_context),components(schemas(Health,Identity,NamespaceRef,NamespaceBinding,AuditEvent,Operation,Error,ErrorDetail,NamespacePage,AuditPage,Version,PublicConfig,IntegrationStatus,BrandingContract,PriceInput,PriceRevision,PriceUnit,PricePage,Budget,BudgetInput,BudgetScope,BudgetPeriod,BudgetPage,PricingSourceInput,PricingSourceRevision,PricingMode,PricingDataStatus,PricingSourcePage,Connection,ConnectionInput,ConnectionPage,ProviderKind,BillingMode,CredentialInput,CredentialType,CatalogPage,ModelMetadata,MetadataRefreshInput,OperationLookup,EndpointPolicyInput,EndpointPolicyPage,ModelContextInput,ModelContextPreference,ModelContextPage)),modifiers(&SecurityAddon))]
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
        .route("/api/v1/prices", get(prices).post(create_price))
        .route("/api/v1/connections", get(connections))
        .route(
            "/api/v1/connections/{connection_id}/models",
            get(read_catalog),
        )
        .route(
            "/api/v1/connections/{connection_id}/models/refresh",
            axum::routing::post(refresh_catalog),
        )
        .route(
            "/api/v1/connections/{connection_id}/credentials",
            axum::routing::put(write_credential).delete(revoke_credential),
        )
        .route(
            "/api/v1/connections/{connection_id}",
            get(read_connection)
                .patch(update_connection)
                .delete(disable_connection),
        )
        .route(
            "/api/v1/providers/{provider}/connections",
            axum::routing::post(create_connection),
        )
        .route(
            "/api/v1/pricing-sources",
            get(pricing_sources).post(create_pricing_source),
        )
        .route("/api/v1/budgets", get(budgets).post(create_budget))
        .route(
            "/api/v1/budgets/{budget_id}",
            axum::routing::patch(update_budget),
        )
        .route("/api/v1/operations/{operation_id}", get(operation))
        .route(
            "/api/v1/operations/by-key/{idempotency_key}",
            get(operation_key),
        )
        .route(
            "/api/v1/operations/by-key/{idempotency_key}/close-unstarted",
            axum::routing::post(close_unstarted_operation),
        )
        .route("/api/v1/connection-presets", get(endpoint_policies))
        .route(
            "/api/v1/connections/{connection_id}/model-contexts",
            get(model_contexts).put(save_model_context),
        )
        .route("/openapi.json", get(openapi))
        .fallback(not_found)
        .layer(DefaultBodyLimit::max(body_limit))
        .with_state(state)
}
