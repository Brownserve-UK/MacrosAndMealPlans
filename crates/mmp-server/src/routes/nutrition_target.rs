use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use super::require_member_access;
use crate::auth::Principal;
use crate::dto::{
    CreateNutritionTargetRequest, NutritionTargetDto, UpdateNutritionTargetRequest, member_id,
    nutrition_target_id,
};
use crate::error::ApiResult;
use crate::http::{Created, IfMatch, Tagged};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(get_one, update, delete))
}

#[utoipa::path(
    get,
    path = "/api/v1/members/{member_id}/nutrition-targets",
    operation_id = "listNutritionTargets",
    params(("member_id" = Uuid, Path, description = "Household member id")),
    responses(
        (status = 200, description = "The member's target history, oldest first",
         body = Vec<NutritionTargetDto>),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
    ),
    tag = "nutrition-targets",
    security(("basic" = []))
)]
async fn list(
    State(state): State<AppState>,
    principal: Principal,
    Path(member): Path<Uuid>,
) -> ApiResult<Json<Vec<NutritionTargetDto>>> {
    let target = member_id(member);
    require_member_access(&state, &principal, target).await?;
    let targets = state.nutrition_targets.list(target).await?;
    Ok(Json(targets.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/api/v1/members/{member_id}/nutrition-targets",
    operation_id = "createNutritionTarget",
    params(("member_id" = Uuid, Path, description = "Household member id")),
    request_body = CreateNutritionTargetRequest,
    responses(
        (status = 201, description = "Created", body = NutritionTargetDto),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 409, description = "A target already takes effect on that date", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "nutrition-targets",
    security(("basic" = []))
)]
async fn create(
    State(state): State<AppState>,
    principal: Principal,
    Path(member): Path<Uuid>,
    Json(body): Json<CreateNutritionTargetRequest>,
) -> ApiResult<Created<NutritionTargetDto>> {
    let target = member_id(member);
    require_member_access(&state, &principal, target).await?;
    let created = state
        .nutrition_targets
        .create(body.into_domain(target))
        .await?;
    Ok(Created(created.revision, created.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/nutrition-targets/{id}",
    operation_id = "getNutritionTarget",
    params(("id" = Uuid, Path, description = "Nutrition target id")),
    responses(
        (status = 200, description = "The target", body = NutritionTargetDto,
         headers(("ETag" = String, description = "The revision to send back as If-Match"))),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 404, description = "Not found", body = crate::error::Problem),
    ),
    tag = "nutrition-targets",
    security(("basic" = []))
)]
async fn get_one(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Tagged<NutritionTargetDto>> {
    let target = state.nutrition_targets.get(nutrition_target_id(id)).await?;
    require_member_access(&state, &principal, target.member_id).await?;
    Ok(Tagged(target.revision, target.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/nutrition-targets/{id}",
    operation_id = "updateNutritionTarget",
    params(
        ("id" = Uuid, Path, description = "Nutrition target id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = UpdateNutritionTargetRequest,
    responses(
        (status = 200, description = "Updated", body = NutritionTargetDto),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
        (status = 428, description = "If-Match is required", body = crate::error::Problem),
    ),
    tag = "nutrition-targets",
    security(("basic" = []))
)]
async fn update(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<UpdateNutritionTargetRequest>,
) -> ApiResult<Tagged<NutritionTargetDto>> {
    let id = nutrition_target_id(id);
    let existing = state.nutrition_targets.get(id).await?;
    require_member_access(&state, &principal, existing.member_id).await?;
    let updated = state
        .nutrition_targets
        .update(id, revision, body.into())
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/nutrition-targets/{id}",
    operation_id = "deleteNutritionTarget",
    params(
        ("id" = Uuid, Path, description = "Nutrition target id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
    ),
    tag = "nutrition-targets",
    security(("basic" = []))
)]
async fn delete(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<StatusCode> {
    let id = nutrition_target_id(id);
    let existing = state.nutrition_targets.get(id).await?;
    require_member_access(&state, &principal, existing.member_id).await?;
    state.nutrition_targets.delete(id, revision).await?;
    Ok(StatusCode::NO_CONTENT)
}
