use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use mmp_core::domain::{HouseholdMemberId, MealPlanComponentId, MealPlanEntryId, Role};
use time::{Date, Weekday};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::Principal;
use crate::dto::common::iso_date;
use crate::dto::{
    CreateMealPlanEntryRequest, MarkMealPlanComponentEatenRequest, MarkMealPlanEatenRequest,
    MealPlanEntryDto, MealPlanWeekDto, UpdateMealPlanEntryRequest,
};
use crate::error::{ApiError, ApiResult};
use crate::http::{Created, IfMatch, Tagged};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_week))
        .routes(routes!(create))
        .routes(routes!(get_one, update, delete))
        .routes(routes!(mark_eaten))
        .routes(routes!(mark_not_eaten))
        .routes(routes!(reopen))
        .routes(routes!(mark_component_eaten))
        .routes(routes!(mark_component_not_eaten))
        .routes(routes!(reopen_component))
}

fn entry_id(id: Uuid) -> MealPlanEntryId {
    id.into()
}

fn component_id(id: Uuid) -> MealPlanComponentId {
    id.into()
}

async fn personal_member(state: &AppState, principal: &Principal) -> ApiResult<HouseholdMemberId> {
    let member_id = principal.member_id.ok_or_else(|| {
        ApiError::new(
            StatusCode::CONFLICT,
            "member-link-required",
            "Meal plan unavailable",
            "Your account is not linked to an active household member.",
        )
    })?;
    let member = state.household.get_member(member_id).await?;
    if member.is_archived() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "archived-member",
            "Archived member",
            "Archived household members cannot have their meal plan changed.",
        ));
    }
    Ok(member_id)
}

async fn require_personal_entry(
    state: &AppState,
    principal: &Principal,
    entry_member_id: HouseholdMemberId,
) -> ApiResult<()> {
    let member = state.household.get_member(entry_member_id).await?;
    if member.is_archived() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "archived-member",
            "Archived member",
            "Archived household members cannot have their meal plan changed.",
        ));
    }
    if principal.roles.contains(&Role::Admin) || principal.member_id == Some(entry_member_id) {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Forbidden",
            "This meal does not belong to your personal plan.",
        ))
    }
}

fn parse_week_start(raw: &str) -> ApiResult<Date> {
    let date = iso_date::parse(raw)
        .map_err(|_| ApiError::bad_request(format!("`{raw}` is not a valid date (YYYY-MM-DD).")))?;
    if date.weekday() != Weekday::Monday {
        return Err(ApiError::bad_request("The week must start on a Monday."));
    }
    Ok(date)
}

#[utoipa::path(
    get,
    path = "/api/v1/meal-plan/{week_start}",
    operation_id = "getMealPlanWeek",
    params(("week_start" = String, Path, example = "2026-08-24")),
    responses(
        (status = 200, body = MealPlanWeekDto),
        (status = 400, body = crate::error::Problem),
        (status = 409, body = crate::error::Problem)
    ),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn get_week(
    State(state): State<AppState>,
    principal: Principal,
    Path(week_start): Path<String>,
) -> ApiResult<Json<MealPlanWeekDto>> {
    let member = personal_member(&state, &principal).await?;
    let week = state
        .meal_plan
        .week(member, parse_week_start(&week_start)?)
        .await?;
    Ok(Json(week.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/meal-plan-entries",
    operation_id = "createMealPlanEntry",
    request_body = CreateMealPlanEntryRequest,
    responses((status = 201, body = MealPlanEntryDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn create(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreateMealPlanEntryRequest>,
) -> ApiResult<Created<MealPlanEntryDto>> {
    let member = personal_member(&state, &principal).await?;
    let mut new_entry = body.into_domain(member, principal.user_id);
    if new_entry.planned_time.is_none() {
        let settings = state.household_settings.get().await?;
        new_entry.planned_time = settings.meal_times.for_slot(new_entry.slot);
    }
    let created = state.meal_plan.create(new_entry).await?;
    Ok(Created(created.entry.revision, created.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/meal-plan-entries/{id}",
    operation_id = "getMealPlanEntry",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = MealPlanEntryDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn get_one(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Tagged<MealPlanEntryDto>> {
    let entry = state.meal_plan.get(entry_id(id)).await?;
    require_personal_entry(&state, &principal, entry.entry.member_id).await?;
    Ok(Tagged(entry.entry.revision, entry.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/meal-plan-entries/{id}",
    operation_id = "updateMealPlanEntry",
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    request_body = UpdateMealPlanEntryRequest,
    responses((status = 200, body = MealPlanEntryDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn update(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<UpdateMealPlanEntryRequest>,
) -> ApiResult<Tagged<MealPlanEntryDto>> {
    let id = entry_id(id);
    let current = state.meal_plan.get(id).await?;
    require_personal_entry(&state, &principal, current.entry.member_id).await?;
    let patch = body.into_domain().map_err(|message| {
        let mut errors = mmp_core::ValidationErrors::new();
        errors.push("planned_time", message);
        mmp_core::CoreError::Validation(errors)
    })?;
    let updated = state
        .meal_plan
        .update(id, revision, patch, principal.user_id)
        .await?;
    Ok(Tagged(updated.entry.revision, updated.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/meal-plan-entries/{id}",
    operation_id = "deleteMealPlanEntry",
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    responses((status = 204)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn delete(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<StatusCode> {
    let id = entry_id(id);
    let current = state.meal_plan.get(id).await?;
    require_personal_entry(&state, &principal, current.entry.member_id).await?;
    state.meal_plan.delete(id, revision).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/v1/meal-plan-entries/{id}/eaten",
    operation_id = "markMealPlanEntryEaten",
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    request_body = MarkMealPlanEatenRequest,
    responses((status = 200, body = MealPlanEntryDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn mark_eaten(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<MarkMealPlanEatenRequest>,
) -> ApiResult<Tagged<MealPlanEntryDto>> {
    let id = entry_id(id);
    let current = state.meal_plan.get(id).await?;
    require_personal_entry(&state, &principal, current.entry.member_id).await?;
    let updated = state
        .meal_plan
        .mark_eaten(id, revision, body.into_domain(principal.user_id))
        .await?;
    Ok(Tagged(updated.entry.revision, updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/meal-plan-entries/{id}/not-eaten",
    operation_id = "markMealPlanEntryNotEaten",
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    responses((status = 200, body = MealPlanEntryDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn mark_not_eaten(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<MealPlanEntryDto>> {
    let id = entry_id(id);
    let current = state.meal_plan.get(id).await?;
    require_personal_entry(&state, &principal, current.entry.member_id).await?;
    let updated = state
        .meal_plan
        .mark_not_eaten(id, revision, principal.user_id)
        .await?;
    Ok(Tagged(updated.entry.revision, updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/meal-plan-entries/{id}/reopen",
    operation_id = "reopenMealPlanEntry",
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    responses((status = 200, body = MealPlanEntryDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn reopen(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<MealPlanEntryDto>> {
    let id = entry_id(id);
    let current = state.meal_plan.get(id).await?;
    require_personal_entry(&state, &principal, current.entry.member_id).await?;
    let updated = state
        .meal_plan
        .reopen(id, revision, principal.user_id)
        .await?;
    Ok(Tagged(updated.entry.revision, updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/meal-plan-entries/{id}/components/{component_id}/eaten",
    operation_id = "markMealPlanComponentEaten",
    params(("id" = Uuid, Path), ("component_id" = Uuid, Path), ("If-Match" = String, Header)),
    request_body = MarkMealPlanComponentEatenRequest,
    responses((status = 200, body = MealPlanEntryDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn mark_component_eaten(
    State(state): State<AppState>,
    principal: Principal,
    Path((id, component)): Path<(Uuid, Uuid)>,
    IfMatch(revision): IfMatch,
    Json(body): Json<MarkMealPlanComponentEatenRequest>,
) -> ApiResult<Json<MealPlanEntryDto>> {
    let id = entry_id(id);
    let current = state.meal_plan.get(id).await?;
    require_personal_entry(&state, &principal, current.entry.member_id).await?;
    let updated = state
        .meal_plan
        .mark_component_eaten(
            id,
            component_id(component),
            revision,
            body.into_domain(principal.user_id),
        )
        .await?;
    Ok(Json(updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/meal-plan-entries/{id}/components/{component_id}/not-eaten",
    operation_id = "markMealPlanComponentNotEaten",
    params(("id" = Uuid, Path), ("component_id" = Uuid, Path), ("If-Match" = String, Header)),
    responses((status = 200, body = MealPlanEntryDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn mark_component_not_eaten(
    State(state): State<AppState>,
    principal: Principal,
    Path((id, component)): Path<(Uuid, Uuid)>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Json<MealPlanEntryDto>> {
    let id = entry_id(id);
    let current = state.meal_plan.get(id).await?;
    require_personal_entry(&state, &principal, current.entry.member_id).await?;
    let updated = state
        .meal_plan
        .mark_component_not_eaten(id, component_id(component), revision, principal.user_id)
        .await?;
    Ok(Json(updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/meal-plan-entries/{id}/components/{component_id}/reopen",
    operation_id = "reopenMealPlanComponent",
    params(("id" = Uuid, Path), ("component_id" = Uuid, Path), ("If-Match" = String, Header)),
    responses((status = 200, body = MealPlanEntryDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn reopen_component(
    State(state): State<AppState>,
    principal: Principal,
    Path((id, component)): Path<(Uuid, Uuid)>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Json<MealPlanEntryDto>> {
    let id = entry_id(id);
    let current = state.meal_plan.get(id).await?;
    require_personal_entry(&state, &principal, current.entry.member_id).await?;
    let updated = state
        .meal_plan
        .reopen_component(id, component_id(component), revision, principal.user_id)
        .await?;
    Ok(Json(updated.into()))
}
