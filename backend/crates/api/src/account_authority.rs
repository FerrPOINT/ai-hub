use super::*;
use aihub_domain::catalog::AccountAuthorityView;
#[utoipa::path(get,path="/api/v1/connections/{connection_id}/account-authority",operation_id="readAccountAuthority",security(("CentralAuth"=[])),params(("connection_id"=Uuid,Path)),responses((status=200,body=AccountAuthorityView),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=503,body=Error)))]
pub async fn read(
    State(state): State<AppState>,
    Authenticated(principal): Authenticated,
    Path(id): Path<String>,
) -> Result<Json<AccountAuthorityView>, ApiError> {
    principal.require_config(false)?;
    let id = Uuid::parse_str(&id).map_err(|_| HubError::Invalid("connection ID"))?;
    Ok(Json(
        state.metadata.account_authority(&principal, id).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/connections/{connection_id}/account-authority",operation_id="qualifyAccountAuthority",security(("CentralAuth"=[])),params(("connection_id"=Uuid,Path),("Idempotency-Key"=Uuid,Header)),request_body=MetadataRefreshInput,responses((status=202,body=Operation),(status=400,body=Error),(status=401,body=Error),(status=403,body=Error),(status=404,body=Error),(status=409,body=Error),(status=412,body=Error),(status=503,body=Error)))]
pub async fn qualify(
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
    let input = body
        .map_err(|_| HubError::Invalid("account authority payload"))?
        .0;
    if input.expected_generation < 1 {
        return Err(HubError::Invalid("connection generation").into());
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(
            state
                .metadata
                .verify_account(&principal, key, id, input.expected_generation)
                .await?,
        ),
    ))
}
