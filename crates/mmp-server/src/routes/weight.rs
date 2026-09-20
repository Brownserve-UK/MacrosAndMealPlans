use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use time::Date;
use utoipa::IntoParams;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use super::require_member_access;
use crate::auth::Principal;
use crate::dto::{
    CreateWeightGoalRequest, CreateWeightRecordRequest, UpdateWeightGoalRequest,
    UpdateWeightRecordRequest, WeightGoalDto, WeightRecordDto, WeightSummaryDto, member_id,
    weight_goal_id, weight_record_id,
};
use crate::error::ApiResult;
use crate::http::{Created, IfMatch, Tagged};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_records, create_record))
        .routes(routes!(get_record, update_record, delete_record))
        .routes(routes!(get_goal, create_goal))
        .routes(routes!(update_goal, delete_goal))
        .routes(routes!(summary))
}

#[derive(Debug, Default, Deserialize, IntoParams)]
struct WeightRecordQuery {
    #[param(value_type = Option<String>, format = Date)]
    from: Option<Date>,
    #[param(maximum = 1000)]
    limit: Option<u32>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
struct WeightSummaryQuery {
    #[param(value_type = Option<String>, format = Date)]
    from: Option<Date>,
}

#[utoipa::path(
    get,
    path = "/api/v1/members/{member_id}/weight-records",
    operation_id = "listWeightRecords",
    params(("member_id" = Uuid, Path, description = "Household member id"), WeightRecordQuery),
    responses(
        (status = 200, description = "The member's weigh-ins, newest first",
         body = Vec<WeightRecordDto>),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
    ),
    tag = "weight",
    security(("basic" = []))
)]
async fn list_records(
    State(state): State<AppState>,
    principal: Principal,
    Path(member): Path<Uuid>,
    Query(query): Query<WeightRecordQuery>,
) -> ApiResult<Json<Vec<WeightRecordDto>>> {
    let member = member_id(member);
    require_member_access(&state, &principal, member).await?;
    let records = state
        .weight
        .list_records_filtered(member, query.from, query.limit.map(|limit| limit.min(1000)))
        .await?;
    Ok(Json(records.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/api/v1/members/{member_id}/weight-records",
    operation_id = "recordWeight",
    params(("member_id" = Uuid, Path, description = "Household member id")),
    request_body = CreateWeightRecordRequest,
    responses(
        (status = 201, description = "Recorded", body = WeightRecordDto),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "weight",
    security(("basic" = []))
)]
async fn create_record(
    State(state): State<AppState>,
    principal: Principal,
    Path(member): Path<Uuid>,
    Json(body): Json<CreateWeightRecordRequest>,
) -> ApiResult<Created<WeightRecordDto>> {
    let member = member_id(member);
    require_member_access(&state, &principal, member).await?;
    let created = state
        .weight
        .record(body.into_domain(member, Some(principal.user_id)))
        .await?;
    Ok(Created(created.revision, created.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/weight-records/{id}",
    operation_id = "getWeightRecord",
    params(("id" = Uuid, Path, description = "Weight record id")),
    responses(
        (status = 200, description = "The weigh-in", body = WeightRecordDto,
         headers(("ETag" = String, description = "The revision to send back as If-Match"))),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 404, description = "Not found", body = crate::error::Problem),
    ),
    tag = "weight",
    security(("basic" = []))
)]
async fn get_record(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Tagged<WeightRecordDto>> {
    let record = state.weight.get_record(weight_record_id(id)).await?;
    require_member_access(&state, &principal, record.member_id).await?;
    Ok(Tagged(record.revision, record.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/weight-records/{id}",
    operation_id = "updateWeightRecord",
    params(
        ("id" = Uuid, Path, description = "Weight record id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = UpdateWeightRecordRequest,
    responses(
        (status = 200, description = "Updated", body = WeightRecordDto),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
        (status = 428, description = "If-Match is required", body = crate::error::Problem),
    ),
    tag = "weight",
    security(("basic" = []))
)]
async fn update_record(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<UpdateWeightRecordRequest>,
) -> ApiResult<Tagged<WeightRecordDto>> {
    let id = weight_record_id(id);
    let existing = state.weight.get_record(id).await?;
    require_member_access(&state, &principal, existing.member_id).await?;
    let updated = state
        .weight
        .update_record(id, revision, body.into())
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/weight-records/{id}",
    operation_id = "deleteWeightRecord",
    params(
        ("id" = Uuid, Path, description = "Weight record id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
    ),
    tag = "weight",
    security(("basic" = []))
)]
async fn delete_record(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<StatusCode> {
    let id = weight_record_id(id);
    let existing = state.weight.get_record(id).await?;
    require_member_access(&state, &principal, existing.member_id).await?;
    state.weight.delete_record(id, revision).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/v1/members/{member_id}/weight-goal",
    operation_id = "getWeightGoal",
    params(("member_id" = Uuid, Path, description = "Household member id")),
    responses(
        (status = 200, description = "The member's goal", body = WeightGoalDto,
         headers(("ETag" = String, description = "The revision to send back as If-Match"))),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 404, description = "No goal set", body = crate::error::Problem),
    ),
    tag = "weight",
    security(("basic" = []))
)]
async fn get_goal(
    State(state): State<AppState>,
    principal: Principal,
    Path(member): Path<Uuid>,
) -> ApiResult<Tagged<WeightGoalDto>> {
    let member = member_id(member);
    require_member_access(&state, &principal, member).await?;
    let goal = state
        .weight
        .goal(member)
        .await?
        .ok_or_else(|| mmp_core::CoreError::not_found("weight goal", member))?;
    Ok(Tagged(goal.revision, goal.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/members/{member_id}/weight-goal",
    operation_id = "setWeightGoal",
    params(("member_id" = Uuid, Path, description = "Household member id")),
    request_body = CreateWeightGoalRequest,
    responses(
        (status = 201, description = "Set", body = WeightGoalDto),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 409, description = "A goal is already set", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "weight",
    security(("basic" = []))
)]
async fn create_goal(
    State(state): State<AppState>,
    principal: Principal,
    Path(member): Path<Uuid>,
    Json(body): Json<CreateWeightGoalRequest>,
) -> ApiResult<Created<WeightGoalDto>> {
    let member = member_id(member);
    require_member_access(&state, &principal, member).await?;
    let created = state.weight.set_goal(body.into_domain(member)).await?;
    Ok(Created(created.revision, created.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/weight-goals/{id}",
    operation_id = "updateWeightGoal",
    params(
        ("id" = Uuid, Path, description = "Weight goal id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = UpdateWeightGoalRequest,
    responses(
        (status = 200, description = "Updated", body = WeightGoalDto),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
        (status = 428, description = "If-Match is required", body = crate::error::Problem),
    ),
    tag = "weight",
    security(("basic" = []))
)]
async fn update_goal(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<UpdateWeightGoalRequest>,
) -> ApiResult<Tagged<WeightGoalDto>> {
    let id = weight_goal_id(id);
    let existing = state.weight.get_goal(id).await?;
    require_member_access(&state, &principal, existing.member_id).await?;
    let updated = state.weight.update_goal(id, revision, body.into()).await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/weight-goals/{id}",
    operation_id = "clearWeightGoal",
    params(
        ("id" = Uuid, Path, description = "Weight goal id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses(
        (status = 204, description = "Cleared"),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
    ),
    tag = "weight",
    security(("basic" = []))
)]
async fn delete_goal(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<StatusCode> {
    let id = weight_goal_id(id);
    let existing = state.weight.get_goal(id).await?;
    require_member_access(&state, &principal, existing.member_id).await?;
    state.weight.clear_goal(id, revision).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/v1/members/{member_id}/weight-summary",
    operation_id = "getWeightSummary",
    params(("member_id" = Uuid, Path, description = "Household member id"), WeightSummaryQuery),
    responses(
        (status = 200, description = "Current weight, goal, projection and trend",
         body = WeightSummaryDto),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
    ),
    tag = "weight",
    security(("basic" = []))
)]
async fn summary(
    State(state): State<AppState>,
    principal: Principal,
    Path(member): Path<Uuid>,
    Query(query): Query<WeightSummaryQuery>,
) -> ApiResult<Json<WeightSummaryDto>> {
    let member = member_id(member);
    require_member_access(&state, &principal, member).await?;
    let summary = state.weight.summary_since(member, query.from).await?;
    Ok(Json(summary.into()))
}
