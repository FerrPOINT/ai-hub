use super::*;
use aihub_domain::profiles::{DraftMutation, Profile, ProfileInput};

#[derive(Serialize, ToSchema)]
pub struct ProfilePage {
    #[schema(max_items = 100)]
    pub items: Vec<Profile>,
    #[schema(required = true)]
    pub next_cursor: Option<String>,
}
#[utoipa::path(get,path="/api/v1/virtual-models",operation_id="listVirtualModels",security(("CentralAuth"=[])),params(("limit"=Option<i64>,Query,minimum=1,maximum=100),("cursor"=Option<Uuid>,Query)),responses((status=200,body=ProfilePage),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=503,body=Error)))]
pub async fn list_profiles(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    RawQuery(raw): RawQuery,
) -> Result<Json<ProfilePage>, ApiError> {
    principal.require_config(false)?;
    let (limit, cursor) = list_options(raw.as_deref().unwrap_or(""), false)?;
    let page = state.foundation.profiles(&principal, limit, cursor).await?;
    Ok(Json(ProfilePage {
        items: page.items,
        next_cursor: page.next_cursor,
    }))
}
#[utoipa::path(get,path="/api/v1/virtual-models/{model_id}",operation_id="readVirtualModel",security(("CentralAuth"=[])),params(("model_id"=Uuid,Path)),responses((status=200,body=Profile,headers(("ETag"=String))),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=503,body=Error)))]
pub async fn read_profile(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(id): Path<String>,
) -> Result<([(String, String); 1], Json<Profile>), ApiError> {
    principal.require_config(false)?;
    let id = Uuid::parse_str(&id).map_err(|_| HubError::Invalid("profile ID"))?;
    let value = state.foundation.profile(&principal, id).await?;
    Ok((
        [("etag".into(), format!("\"{}\"", value.draft_version))],
        Json(value),
    ))
}
type ProfileResponse = (StatusCode, [(String, String); 2], Json<Profile>);
fn response(status: StatusCode, result: DraftMutation) -> ProfileResponse {
    (
        status,
        [
            (
                "etag".into(),
                format!("\"{}\"", result.profile.draft_version),
            ),
            ("x-operation-id".into(), result.operation_id.to_string()),
        ],
        Json(result.profile),
    )
}
#[utoipa::path(post,path="/api/v1/virtual-models",operation_id="createVirtualModel",security(("CentralAuth"=[])),params(("Idempotency-Key"=Uuid,Header)),request_body=ProfileInput,responses((status=201,body=Profile,headers(("ETag"=String),("X-Operation-Id"=Uuid))),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=409,body=Error),(status=422,body=Error),(status=503,body=Error)))]
pub async fn create_profile(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    headers: HeaderMap,
    body: Result<Json<ProfileInput>, axum::extract::rejection::JsonRejection>,
) -> Result<ProfileResponse, ApiError> {
    principal.require_config(true)?;
    let key = mutation_key(&headers)?;
    let input = body.map_err(|_| HubError::Invalid("profile payload"))?.0;
    Ok(response(
        StatusCode::CREATED,
        state
            .foundation
            .save_profile(&principal, key, None, &input)
            .await?,
    ))
}
#[utoipa::path(patch,path="/api/v1/virtual-models/{model_id}",operation_id="updateVirtualModelDraft",security(("CentralAuth"=[])),params(("model_id"=Uuid,Path),("Idempotency-Key"=Uuid,Header),("If-Match"=String,Header)),request_body=ProfileInput,responses((status=201,body=Profile,headers(("ETag"=String),("X-Operation-Id"=Uuid))),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=409,body=Error),(status=412,body=Error),(status=422,body=Error),(status=503,body=Error)))]
pub async fn update_profile(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Result<Json<ProfileInput>, axum::extract::rejection::JsonRejection>,
) -> Result<ProfileResponse, ApiError> {
    principal.require_config(true)?;
    let key = mutation_key(&headers)?;
    let version = budget_version(&headers)?;
    let id = Uuid::parse_str(&id).map_err(|_| HubError::Invalid("profile ID"))?;
    let input = body.map_err(|_| HubError::Invalid("profile payload"))?.0;
    Ok(response(
        StatusCode::CREATED,
        state
            .foundation
            .save_profile(&principal, key, Some((id, version)), &input)
            .await?,
    ))
}
