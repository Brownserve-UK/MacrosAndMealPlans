use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use mmp_core::ports::PageRequest;
use time::Date;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use super::require_member_access;
use crate::auth::Principal;
use crate::dto::common::iso_date;
use crate::dto::{
    ConsumptionRecordDto, CreateConsumptionRequest, DiaryDayDto, HouseholdMemberDto,
    UpdateConsumptionRequest, consumption_id, member_id,
};
use crate::error::{ApiError, ApiResult};
use crate::http::{Created, IfMatch, Tagged};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create))
        .routes(routes!(get_one, update, delete))
        .routes(routes!(get_day))
        .routes(routes!(list_members))
}

fn parse_path_date(raw: &str) -> ApiResult<Date> {
    iso_date::parse(raw)
        .map_err(|_| ApiError::bad_request(format!("`{raw}` is not a valid date (YYYY-MM-DD).")))
}

#[utoipa::path(
    post,
    path = "/api/v1/consumption",
    request_body = CreateConsumptionRequest,
    operation_id = "createConsumptionRecord",
    responses(
        (status = 201, description = "Recorded", body = ConsumptionRecordDto),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "diary",
    security(("basic" = []))
)]
async fn create(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreateConsumptionRequest>,
) -> ApiResult<Created<ConsumptionRecordDto>> {
    let target = member_id(body.member_id);
    require_member_access(&state, &principal, target).await?;

    let mut input: mmp_core::domain::NewConsumptionRecord = body.into();
    input.recorded_by = Some(principal.user_id);

    let created = state.diary.record(input).await?;
    Ok(Created(created.revision, created.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/consumption/{id}",
    operation_id = "getConsumptionRecord",
    params(("id" = Uuid, Path, description = "Consumption record id")),
    responses(
        (status = 200, description = "The record", body = ConsumptionRecordDto,
         headers(("ETag" = String, description = "The revision to send back as If-Match"))),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 404, description = "Not found", body = crate::error::Problem),
    ),
    tag = "diary",
    security(("basic" = []))
)]
async fn get_one(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Tagged<ConsumptionRecordDto>> {
    let record = state.diary.get(consumption_id(id)).await?;
    require_member_access(&state, &principal, record.member_id).await?;
    Ok(Tagged(record.revision, record.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/consumption/{id}",
    operation_id = "updateConsumptionRecord",
    params(
        ("id" = Uuid, Path, description = "Consumption record id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = UpdateConsumptionRequest,
    responses(
        (status = 200, description = "Updated", body = ConsumptionRecordDto),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 428, description = "If-Match is required", body = crate::error::Problem),
    ),
    tag = "diary",
    security(("basic" = []))
)]
async fn update(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<UpdateConsumptionRequest>,
) -> ApiResult<Tagged<ConsumptionRecordDto>> {
    let id = consumption_id(id);
    let existing = state.diary.get(id).await?;
    require_member_access(&state, &principal, existing.member_id).await?;

    let updated = state.diary.amend(id, revision, body.into()).await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/consumption/{id}",
    operation_id = "deleteConsumptionRecord",
    params(
        ("id" = Uuid, Path, description = "Consumption record id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
    ),
    tag = "diary",
    security(("basic" = []))
)]
async fn delete(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<StatusCode> {
    let id = consumption_id(id);
    let existing = state.diary.get(id).await?;
    require_member_access(&state, &principal, existing.member_id).await?;

    state.diary.remove(id, revision).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/v1/diary/{member_id}/{date}",
    operation_id = "getDiaryDay",
    params(
        ("member_id" = Uuid, Path, description = "Household member id"),
        ("date" = String, Path, description = "ISO date (YYYY-MM-DD)", example = "2026-08-22"),
    ),
    responses(
        (status = 200, description = "The day's entries and totals", body = DiaryDayDto),
        (status = 400, description = "The date could not be parsed", body = crate::error::Problem),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
    ),
    tag = "diary",
    security(("basic" = []))
)]
async fn get_day(
    State(state): State<AppState>,
    principal: Principal,
    Path((member, date)): Path<(Uuid, String)>,
) -> ApiResult<Json<DiaryDayDto>> {
    let target = member_id(member);
    require_member_access(&state, &principal, target).await?;

    let date = parse_path_date(&date)?;
    let day = state.diary.day(target, date).await?;
    Ok(Json(day.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/diary/members",
    operation_id = "listDiaryMembers",
    responses((status = 200, description = "The members whose diaries you may see",
               body = Vec<HouseholdMemberDto>)),
    tag = "diary",
    security(("basic" = []))
)]
async fn list_members(
    State(state): State<AppState>,
    principal: Principal,
) -> ApiResult<Json<Vec<HouseholdMemberDto>>> {
    let user = state.household.get_user(principal.user_id).await?;
    let all = state
        .household
        .list_members(&mmp_core::ports::MemberQuery {
            page: PageRequest::new(1, PageRequest::MAX_PER_PAGE),
            ..Default::default()
        })
        .await?;

    let mut visible = Vec::new();
    for candidate in all.items {
        if state
            .household
            .can_view_member_health_data(&user, candidate.id)
            .await?
        {
            visible.push(candidate.into());
        }
    }
    Ok(Json(visible))
}
