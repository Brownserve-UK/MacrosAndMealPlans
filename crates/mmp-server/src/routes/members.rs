use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use mmp_core::domain::{AccessScope, Revision};
use mmp_core::ports::{MemberQuery, PageRequest};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::{AuthError, Permission, Principal};
use crate::dto::{
    CreateMemberRequest, GrantAccessRequest, HouseholdMemberDto, LinkAccountRequest,
    MemberAccessGrantDto, MemberListQuery, MemberPage, UpdateMemberRequest, member_id, user_id,
};
use crate::error::ApiResult;
use crate::http::{Created, IfMatch, Tagged};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(get_one, update))
        .routes(routes!(archive))
        .routes(routes!(unarchive))
        .routes(routes!(link_account, unlink_account))
        .routes(routes!(list_access, grant_access))
        .routes(routes!(revoke_access))
}

fn to_query(query: MemberListQuery) -> MemberQuery {
    MemberQuery {
        search: query.q.filter(|q| !q.trim().is_empty()),
        with_account: query.with_account,
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
    path = "/api/v1/members",
    params(MemberListQuery),
    operation_id = "listMembers",
    responses(
        (status = 200, description = "A page of household members", body = MemberPage),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
    ),
    tag = "household",
    security(("basic" = []))
)]
async fn list(
    State(state): State<AppState>,
    principal: Principal,
    Query(query): Query<MemberListQuery>,
) -> ApiResult<Json<MemberPage>> {
    principal.require(Permission::HouseholdRead)?;
    let page = state.household.list_members(&to_query(query)).await?;
    Ok(Json(page.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/members",
    request_body = CreateMemberRequest,
    operation_id = "createMember",
    responses(
        (status = 201, description = "Created", body = HouseholdMemberDto),
        (status = 409, description = "That name is taken", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "household",
    security(("basic" = []))
)]
async fn create(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreateMemberRequest>,
) -> ApiResult<Created<HouseholdMemberDto>> {
    principal.require(Permission::HouseholdWrite)?;
    let created = state.household.create_member(body.into()).await?;
    Ok(Created(created.revision, created.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/members/{id}",
    operation_id = "getMember",
    params(("id" = Uuid, Path, description = "Member id")),
    responses(
        (status = 200, description = "The member", body = HouseholdMemberDto,
         headers(("ETag" = String, description = "The revision to send back as If-Match"))),
        (status = 404, description = "Not found", body = crate::error::Problem),
    ),
    tag = "household",
    security(("basic" = []))
)]
async fn get_one(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Tagged<HouseholdMemberDto>> {
    principal.require(Permission::HouseholdRead)?;
    let member = state.household.get_member(member_id(id)).await?;
    Ok(Tagged(member.revision, member.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/members/{id}",
    operation_id = "updateMember",
    params(
        ("id" = Uuid, Path, description = "Member id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = UpdateMemberRequest,
    responses(
        (status = 200, description = "Updated", body = HouseholdMemberDto),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 428, description = "If-Match is required", body = crate::error::Problem),
    ),
    tag = "household",
    security(("basic" = []))
)]
async fn update(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<UpdateMemberRequest>,
) -> ApiResult<Tagged<HouseholdMemberDto>> {
    let target = member_id(id);
    require_household_write_or_self(&principal, target)?;
    let updated = state
        .household
        .update_member(target, revision, body.into())
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

fn require_household_write_or_self(
    principal: &Principal,
    target: mmp_core::domain::HouseholdMemberId,
) -> Result<(), AuthError> {
    if principal.has(Permission::HouseholdWrite) || principal.member_id == Some(target) {
        Ok(())
    } else {
        Err(AuthError::Forbidden(Permission::HouseholdWrite))
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/members/{id}/archive",
    operation_id = "archiveMember",
    params(
        ("id" = Uuid, Path, description = "Member id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Archived", body = HouseholdMemberDto)),
    tag = "household",
    security(("basic" = []))
)]
async fn archive(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<HouseholdMemberDto>> {
    set_archived(state, principal, id, revision, true).await
}

#[utoipa::path(
    post,
    path = "/api/v1/members/{id}/unarchive",
    operation_id = "unarchiveMember",
    params(
        ("id" = Uuid, Path, description = "Member id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Restored", body = HouseholdMemberDto)),
    tag = "household",
    security(("basic" = []))
)]
async fn unarchive(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<HouseholdMemberDto>> {
    set_archived(state, principal, id, revision, false).await
}

async fn set_archived(
    state: AppState,
    principal: Principal,
    id: Uuid,
    revision: Revision,
    archived: bool,
) -> ApiResult<Tagged<HouseholdMemberDto>> {
    principal.require(Permission::HouseholdWrite)?;
    let updated = state
        .household
        .set_member_archived(member_id(id), revision, archived)
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    put,
    path = "/api/v1/members/{id}/account",
    operation_id = "linkMemberAccount",
    params(
        ("id" = Uuid, Path, description = "Member id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = LinkAccountRequest,
    responses(
        (status = 200, description = "Linked", body = HouseholdMemberDto),
        (status = 422, description = "That account is already in use", body = crate::error::Problem),
    ),
    tag = "household",
    security(("basic" = []))
)]
async fn link_account(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<LinkAccountRequest>,
) -> ApiResult<Tagged<HouseholdMemberDto>> {
    principal.require(Permission::HouseholdWrite)?;
    let updated = state
        .household
        .link_account(member_id(id), revision, user_id(body.user_id))
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/members/{id}/account",
    operation_id = "unlinkMemberAccount",
    params(
        ("id" = Uuid, Path, description = "Member id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Unlinked", body = HouseholdMemberDto)),
    tag = "household",
    security(("basic" = []))
)]
async fn unlink_account(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<HouseholdMemberDto>> {
    principal.require(Permission::HouseholdWrite)?;
    let updated = state
        .household
        .unlink_account(member_id(id), revision)
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/members/{id}/access",
    operation_id = "listMemberAccess",
    params(("id" = Uuid, Path, description = "Member id")),
    responses((status = 200, description = "Who may see this member's private data",
               body = Vec<MemberAccessGrantDto>)),
    tag = "household",
    security(("basic" = []))
)]
async fn list_access(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<MemberAccessGrantDto>>> {
    principal.require(Permission::AccountAdmin)?;
    let grants = state.household.list_member_access(member_id(id)).await?;
    Ok(Json(grants.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    put,
    path = "/api/v1/members/{id}/access",
    operation_id = "grantMemberAccess",
    params(("id" = Uuid, Path, description = "Member id")),
    request_body = GrantAccessRequest,
    responses((status = 200, description = "Granted", body = MemberAccessGrantDto)),
    tag = "household",
    security(("basic" = []))
)]
async fn grant_access(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    Json(body): Json<GrantAccessRequest>,
) -> ApiResult<Json<MemberAccessGrantDto>> {
    principal.require(Permission::AccountAdmin)?;
    let grant = state
        .household
        .grant_access(
            member_id(id),
            user_id(body.user_id),
            body.scope,
            Some(principal.user_id),
        )
        .await?;
    Ok(Json(grant.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/members/{id}/access/{user_id}/{scope}",
    operation_id = "revokeMemberAccess",
    params(
        ("id" = Uuid, Path, description = "Member id"),
        ("user_id" = Uuid, Path, description = "The account losing access"),
        ("scope" = AccessScope, Path, description = "The scope being revoked"),
    ),
    responses((status = 204, description = "Revoked")),
    tag = "household",
    security(("basic" = []))
)]
async fn revoke_access(
    State(state): State<AppState>,
    principal: Principal,
    Path((id, grantee, scope)): Path<(Uuid, Uuid, AccessScope)>,
) -> ApiResult<StatusCode> {
    principal.require(Permission::AccountAdmin)?;
    state
        .household
        .revoke_access(member_id(id), user_id(grantee), scope)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
