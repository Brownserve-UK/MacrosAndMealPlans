use axum::Json;
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::{Principal, Role};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_session, delete_session))
        .routes(routes!(me))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PrincipalDto {
    pub user_id: Uuid,
    #[schema(example = "admin")]
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_id: Option<Uuid>,
    pub roles: Vec<Role>,
    pub permissions: Vec<&'static str>,
}

impl From<Principal> for PrincipalDto {
    fn from(value: Principal) -> Self {
        let mut permissions: Vec<&'static str> =
            value.permissions.iter().map(|p| p.code()).collect();
        permissions.sort_unstable();
        Self {
            user_id: value.user_id.as_uuid(),
            username: value.username,
            member_id: value.member_id.map(|id| id.as_uuid()),
            roles: value.roles,
            permissions,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/session",
    operation_id = "createSession",
    responses(
        (status = 200, description = "The credentials are valid", body = PrincipalDto),
        (status = 401, description = "Rejected", body = crate::error::Problem),
    ),
    tag = "auth",
    security(("basic" = []))
)]
async fn create_session(principal: Principal) -> Json<PrincipalDto> {
    Json(principal.into())
}

#[utoipa::path(
    delete,
    path = "/api/v1/auth/session",
    operation_id = "deleteSession",
    responses((status = 204, description = "Session discarded")),
    tag = "auth"
)]
async fn delete_session() -> StatusCode {
    StatusCode::NO_CONTENT
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    operation_id = "getCurrentUser",
    responses(
        (status = 200, description = "The authenticated user", body = PrincipalDto),
        (status = 401, description = "Not authenticated", body = crate::error::Problem),
    ),
    tag = "auth",
    security(("basic" = []))
)]
async fn me(principal: Principal) -> Json<PrincipalDto> {
    Json(principal.into())
}
