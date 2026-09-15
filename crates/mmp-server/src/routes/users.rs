use axum::Json;
use axum::extract::{Path, Query, State};
use mmp_core::domain::Revision;
use mmp_core::ports::{PageRequest, UserQuery};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::{Permission, Principal};
use crate::dto::{
    CreateUserRequest, SetRolesRequest, UpdateUserRequest, UserDto, UserListQuery, UserPage,
    user_id,
};
use crate::error::ApiResult;
use crate::http::{Created, IfMatch, Tagged};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(get_one, update))
        .routes(routes!(set_roles))
        .routes(routes!(archive))
        .routes(routes!(unarchive))
}

fn to_query(query: UserListQuery) -> UserQuery {
    UserQuery {
        search: query.q.filter(|q| !q.trim().is_empty()),
        role: query.role,
        include_archived: query.include_archived.unwrap_or(false),
        page: PageRequest::new(
            query.page.unwrap_or(1),
            query.per_page.unwrap_or(PageRequest::DEFAULT_PER_PAGE),
        ),
        sort: query.sort.map(Into::into).unwrap_or_default(),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/users",
    params(UserListQuery),
    operation_id = "listUsers",
    responses(
        (status = 200, description = "A page of accounts", body = UserPage),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
    ),
    tag = "accounts",
    security(("basic" = []))
)]
async fn list(
    State(state): State<AppState>,
    principal: Principal,
    Query(query): Query<UserListQuery>,
) -> ApiResult<Json<UserPage>> {
    principal.require(Permission::AccountAdmin)?;
    let page = state.household.list_users(&to_query(query)).await?;
    Ok(Json(page.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/users",
    request_body = CreateUserRequest,
    operation_id = "createUser",
    responses(
        (status = 201, description = "Created", body = UserDto),
        (status = 409, description = "That username is taken", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "accounts",
    security(("basic" = []))
)]
async fn create(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreateUserRequest>,
) -> ApiResult<Created<UserDto>> {
    principal.require(Permission::AccountAdmin)?;
    let created = state.household.create_user(body.into()).await?;
    Ok(Created(created.revision, created.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    operation_id = "getUser",
    params(("id" = Uuid, Path, description = "Account id")),
    responses(
        (status = 200, description = "The account", body = UserDto,
         headers(("ETag" = String, description = "The revision to send back as If-Match"))),
        (status = 404, description = "Not found", body = crate::error::Problem),
    ),
    tag = "accounts",
    security(("basic" = []))
)]
async fn get_one(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Tagged<UserDto>> {
    principal.require(Permission::AccountAdmin)?;
    let user = state.household.get_user(user_id(id)).await?;
    Ok(Tagged(user.revision, user.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/users/{id}",
    operation_id = "updateUser",
    params(
        ("id" = Uuid, Path, description = "Account id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "Updated", body = UserDto),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 428, description = "If-Match is required", body = crate::error::Problem),
    ),
    tag = "accounts",
    security(("basic" = []))
)]
async fn update(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<UpdateUserRequest>,
) -> ApiResult<Tagged<UserDto>> {
    principal.require(Permission::AccountAdmin)?;
    let updated = state
        .household
        .update_user(user_id(id), revision, body.into())
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    put,
    path = "/api/v1/users/{id}/roles",
    operation_id = "setUserRoles",
    params(
        ("id" = Uuid, Path, description = "Account id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = SetRolesRequest,
    responses(
        (status = 200, description = "Roles set", body = UserDto),
        (status = 422, description = "The last admin has to stay an admin",
         body = crate::error::Problem),
    ),
    tag = "accounts",
    security(("basic" = []))
)]
async fn set_roles(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<SetRolesRequest>,
) -> ApiResult<Tagged<UserDto>> {
    principal.require(Permission::AccountAdmin)?;
    let updated = state
        .household
        .set_user_roles(user_id(id), revision, body.roles)
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/archive",
    operation_id = "archiveUser",
    params(
        ("id" = Uuid, Path, description = "Account id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Archived", body = UserDto)),
    tag = "accounts",
    security(("basic" = []))
)]
async fn archive(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<UserDto>> {
    set_archived(state, principal, id, revision, true).await
}

#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/unarchive",
    operation_id = "unarchiveUser",
    params(
        ("id" = Uuid, Path, description = "Account id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Restored", body = UserDto)),
    tag = "accounts",
    security(("basic" = []))
)]
async fn unarchive(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<UserDto>> {
    set_archived(state, principal, id, revision, false).await
}

async fn set_archived(
    state: AppState,
    principal: Principal,
    id: Uuid,
    revision: Revision,
    archived: bool,
) -> ApiResult<Tagged<UserDto>> {
    principal.require(Permission::AccountAdmin)?;
    let updated = state
        .household
        .set_user_archived(user_id(id), revision, archived)
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}
