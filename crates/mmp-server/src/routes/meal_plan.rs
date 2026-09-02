use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use mmp_core::domain::{HouseholdMemberId, MealPlanComponentId, MealPlanEntryId, Role};
use time::{Date, Duration, Weekday};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::Principal;
use crate::dto::common::iso_date;
use crate::dto::{
    CreateMealPlanEntryRequest, MarkMealPlanComponentEatenRequest, MarkMealPlanEatenRequest,
    MealGuestGroupDto, MealPlanEntryDto, MealPlanWeekDto, PlannerCapabilitiesDto, PlannerFoodDto,
    PlannerMealDto, PlannerPersonDto, PlannerWeekDto, ReviewMealOutcomesRequest,
    SetMealPlanParticipantsRequest, UpdateMealPlanEntryRequest,
};
use crate::error::{ApiError, ApiResult};
use crate::http::{Created, IfMatch, Tagged};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_week))
        .routes(routes!(get_planner_week))
        .routes(routes!(create))
        .routes(routes!(get_one, update, delete))
        .routes(routes!(mark_eaten))
        .routes(routes!(mark_not_eaten))
        .routes(routes!(reopen))
        .routes(routes!(mark_component_eaten))
        .routes(routes!(mark_component_not_eaten))
        .routes(routes!(reopen_component))
        .routes(routes!(set_participants))
        .routes(routes!(opt_out, opt_in))
        .routes(routes!(household_slot_attendance))
        .routes(routes!(review_outcomes))
}

fn entry_id(id: Uuid) -> MealPlanEntryId {
    id.into()
}

fn component_id(id: Uuid) -> MealPlanComponentId {
    id.into()
}

pub(crate) async fn personal_member(
    state: &AppState,
    principal: &Principal,
) -> ApiResult<HouseholdMemberId> {
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

async fn require_entry_access(
    state: &AppState,
    principal: &Principal,
    entry: &mmp_core::domain::MealPlanEntry,
) -> ApiResult<()> {
    if principal.roles.contains(&Role::Admin) {
        return Ok(());
    }
    if let Some(entry_member_id) = entry.member_id {
        let member = state.household.get_member(entry_member_id).await?;
        if member.is_archived() {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "archived-member",
                "Archived member",
                "Archived household members cannot have their meal plan changed.",
            ));
        }
        if principal.member_id == Some(entry_member_id) {
            return Ok(());
        }
    }
    let participating = principal
        .member_id
        .is_some_and(|member_id| entry.participant_for(member_id).is_some());
    if participating || principal.roles.contains(&Role::HouseholdManager) {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Forbidden",
            "This meal does not belong to your plan.",
        ))
    }
}

async fn can_manage_personal_meal(
    state: &AppState,
    principal: &Principal,
    member_id: HouseholdMemberId,
) -> ApiResult<bool> {
    if principal.roles.contains(&Role::Admin) || principal.member_id == Some(member_id) {
        return Ok(true);
    }
    Ok(state
        .household
        .can_manage_member_meal_plan(principal.user_id, member_id)
        .await?)
}

async fn require_plan_edit_access(
    state: &AppState,
    principal: &Principal,
    entry: &mmp_core::domain::MealPlanEntry,
) -> ApiResult<()> {
    let allowed = match entry.scope {
        mmp_core::domain::MealPlanScope::Member => {
            let member_id = entry.member_id.ok_or_else(|| {
                ApiError::new(
                    StatusCode::CONFLICT,
                    "invalid-meal",
                    "Meal unavailable",
                    "This personal meal has no owner.",
                )
            })?;
            can_manage_personal_meal(state, principal, member_id).await?
        }
        mmp_core::domain::MealPlanScope::Household => {
            principal.has(mmp_core::domain::Permission::HouseholdWrite)
        }
    };
    if allowed {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Forbidden",
            "You cannot edit this meal.",
        ))
    }
}

async fn require_outcome_access(
    state: &AppState,
    principal: &Principal,
    member_id: HouseholdMemberId,
) -> ApiResult<()> {
    if can_manage_personal_meal(state, principal, member_id).await? {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Forbidden",
            "You cannot record this person's meal.",
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
    get,
    path = "/api/v1/planner/{week_start}",
    operation_id = "getHouseholdPlannerWeek",
    params(("week_start" = String, Path, example = "2026-08-24")),
    responses((status = 200, body = PlannerWeekDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn get_planner_week(
    State(state): State<AppState>,
    principal: Principal,
    Path(week_start): Path<String>,
) -> ApiResult<Json<PlannerWeekDto>> {
    principal.require(mmp_core::domain::Permission::HouseholdWrite)?;
    let week_start = parse_week_start(&week_start)?;
    let views = state.meal_plan.planner_entries(week_start).await?;
    let mut meals = Vec::new();
    for view in views {
        if view.entry.scope != mmp_core::domain::MealPlanScope::Household {
            continue;
        }
        let mut people = Vec::with_capacity(view.participants.len());
        for participant in &view.participants {
            let can_record = principal.member_id == Some(participant.member_id)
                || principal.has(mmp_core::domain::Permission::AccountAdmin)
                || state
                    .household
                    .can_manage_member_meal_plan(principal.user_id, participant.member_id)
                    .await?;
            people.push(PlannerPersonDto {
                member_id: participant.member_id.as_uuid(),
                display_name: participant.display_name.clone(),
                status: participant.status,
                allocations: participant
                    .allocations
                    .clone()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                can_record,
            });
        }
        let can_opt_out = false;
        let can_join = false;
        let can_edit = view.status.is_unresolved();
        let owner_name = None;
        let foods = view
            .components
            .iter()
            .map(|component| PlannerFoodDto {
                id: component.component.id.as_uuid(),
                item: component.component.item.into(),
                item_name: component.item_name.clone(),
                amount: component.component.amount.into(),
                shortage: component.preparation.shortage,
            })
            .collect();
        meals.push(PlannerMealDto {
            id: view.entry.id.as_uuid(),
            scope: view.entry.scope,
            member_id: view.entry.member_id.map(|id| id.as_uuid()),
            owner_name,
            planned_on: view.entry.planned_on,
            planned_time: view.entry.planned_time,
            slot: view.entry.slot,
            portioning: view.entry.portioning,
            status: view.status,
            foods,
            people,
            guest_groups: view
                .entry
                .guest_groups
                .clone()
                .into_iter()
                .map(|group| MealGuestGroupDto::build(group, view.assumption))
                .collect(),
            opted_out: view
                .entry
                .opted_out
                .clone()
                .into_iter()
                .map(Into::into)
                .collect(),
            can_opt_out,
            can_join,
            capabilities: PlannerCapabilitiesDto {
                can_edit,
                can_delete: can_edit,
                can_record_guests: true,
            },
            revision: view.entry.revision.get(),
        });
    }
    Ok(Json(PlannerWeekDto {
        week_start,
        week_end: week_start + Duration::days(6),
        meals,
    }))
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
    if body.household {
        if !principal.has(mmp_core::domain::Permission::HouseholdWrite) {
            return Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "forbidden",
                "Forbidden",
                "You cannot plan a household meal.",
            ));
        }
    } else if let Some(requested) = body.member_id
        && !can_manage_personal_meal(&state, &principal, requested.into()).await?
    {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Forbidden",
            "You cannot plan this person's meal.",
        ));
    }
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
    require_entry_access(&state, &principal, &entry.entry).await?;
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
    require_plan_edit_access(&state, &principal, &current.entry).await?;
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
    require_plan_edit_access(&state, &principal, &current.entry).await?;
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
    let subject = body
        .member_id
        .map(Into::into)
        .or(principal.member_id)
        .ok_or_else(|| ApiError::bad_request("Choose whose meal to record."))?;
    require_outcome_access(&state, &principal, subject).await?;
    require_entry_access(&state, &principal, &current.entry).await?;
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
    let subject = principal.member_id.ok_or_else(|| {
        ApiError::bad_request("Your account is not linked to a household member.")
    })?;
    require_outcome_access(&state, &principal, subject).await?;
    require_entry_access(&state, &principal, &current.entry).await?;
    let updated = state
        .meal_plan
        .mark_not_eaten(
            id,
            revision,
            mmp_core::domain::OutcomeActor {
                actor_id: principal.user_id,
                subject_member_id: Some(subject),
            },
        )
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
    let subject = principal.member_id.ok_or_else(|| {
        ApiError::bad_request("Your account is not linked to a household member.")
    })?;
    require_outcome_access(&state, &principal, subject).await?;
    require_entry_access(&state, &principal, &current.entry).await?;
    let updated = state
        .meal_plan
        .reopen(
            id,
            revision,
            mmp_core::domain::OutcomeActor {
                actor_id: principal.user_id,
                subject_member_id: Some(subject),
            },
        )
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
    let subject = body
        .member_id
        .map(Into::into)
        .or(principal.member_id)
        .ok_or_else(|| ApiError::bad_request("Choose whose meal to record."))?;
    require_outcome_access(&state, &principal, subject).await?;
    require_entry_access(&state, &principal, &current.entry).await?;
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
    let subject = principal.member_id.ok_or_else(|| {
        ApiError::bad_request("Your account is not linked to a household member.")
    })?;
    require_outcome_access(&state, &principal, subject).await?;
    require_entry_access(&state, &principal, &current.entry).await?;
    let updated = state
        .meal_plan
        .mark_component_not_eaten(
            id,
            component_id(component),
            revision,
            mmp_core::domain::OutcomeActor {
                actor_id: principal.user_id,
                subject_member_id: Some(subject),
            },
        )
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
    let subject = principal.member_id.ok_or_else(|| {
        ApiError::bad_request("Your account is not linked to a household member.")
    })?;
    require_outcome_access(&state, &principal, subject).await?;
    require_entry_access(&state, &principal, &current.entry).await?;
    let updated = state
        .meal_plan
        .reopen_component(
            id,
            component_id(component),
            revision,
            mmp_core::domain::OutcomeActor {
                actor_id: principal.user_id,
                subject_member_id: Some(subject),
            },
        )
        .await?;
    Ok(Json(updated.into()))
}

#[utoipa::path(
    put,
    path = "/api/v1/meal-plan-entries/{id}/participants",
    operation_id = "setMealPlanParticipants",
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    request_body = SetMealPlanParticipantsRequest,
    responses((status = 200, body = MealPlanEntryDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn set_participants(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<SetMealPlanParticipantsRequest>,
) -> ApiResult<Tagged<MealPlanEntryDto>> {
    let id = entry_id(id);
    let current = state.meal_plan.get(id).await?;
    require_plan_edit_access(&state, &principal, &current.entry).await?;
    let updated = state
        .meal_plan
        .set_participants(id, revision, body.into_domain(principal.user_id))
        .await?;
    Ok(Tagged(updated.entry.revision, updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/meal-plan-entries/{id}/opt-out",
    operation_id = "optOutOfMealPlanEntry",
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    responses((status = 200, body = MealPlanEntryDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn opt_out(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<MealPlanEntryDto>> {
    let id = entry_id(id);
    let member = personal_member(&state, &principal).await?;
    let updated = state
        .meal_plan
        .opt_out(id, revision, principal.user_id, member)
        .await?;
    Ok(Tagged(updated.entry.revision, updated.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/meal-plan-entries/{id}/opt-out",
    operation_id = "rejoinMealPlanEntry",
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    responses((status = 200, body = MealPlanEntryDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn opt_in(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<MealPlanEntryDto>> {
    let id = entry_id(id);
    let member = personal_member(&state, &principal).await?;
    let updated = state
        .meal_plan
        .opt_in(id, revision, principal.user_id, member)
        .await?;
    Ok(Tagged(updated.entry.revision, updated.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/household/planner/attendance/{date}/{slot}",
    operation_id = "getHouseholdSlotAttendance",
    params(
        ("date" = String, Path, example = "2026-09-10"),
        ("slot" = String, Path, example = "dinner"),
        ("exclude_entry" = Option<Uuid>, Query)
    ),
    responses((status = 200, body = [crate::dto::SlotAttendanceDto])),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn household_slot_attendance(
    State(state): State<AppState>,
    principal: Principal,
    Path((date, slot)): Path<(String, String)>,
    axum::extract::Query(query): axum::extract::Query<AttendanceQuery>,
) -> ApiResult<Json<Vec<crate::dto::SlotAttendanceDto>>> {
    principal.require(mmp_core::domain::Permission::HouseholdWrite)?;
    let date = iso_date::parse(&date).map_err(|_| {
        ApiError::bad_request(format!("`{date}` is not a valid date (YYYY-MM-DD)."))
    })?;
    let slot: mmp_core::domain::MealSlot = slot
        .parse()
        .map_err(|_| ApiError::bad_request(format!("`{slot}` is not a meal slot.")))?;
    let rows = state
        .meal_plan
        .slot_attendance(date, slot, query.exclude_entry.map(entry_id))
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for (member_id, attendance, claimed_time) in rows {
        let member = state.household.get_member(member_id).await?;
        out.push(crate::dto::SlotAttendanceDto {
            member_id: member_id.as_uuid(),
            display_name: member.display_name,
            attendance,
            claimed_time,
        });
    }
    Ok(Json(out))
}

#[derive(serde::Deserialize)]
struct AttendanceQuery {
    #[serde(default)]
    exclude_entry: Option<Uuid>,
}

#[utoipa::path(
    post,
    path = "/api/v1/meal-plan-entries/{id}/outcomes",
    operation_id = "reviewMealPlanOutcomes",
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    request_body = ReviewMealOutcomesRequest,
    responses((status = 200, body = MealPlanEntryDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn review_outcomes(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<ReviewMealOutcomesRequest>,
) -> ApiResult<Tagged<MealPlanEntryDto>> {
    let id = entry_id(id);
    let current = state.meal_plan.get(id).await?;
    for member in &body.members {
        let member_id: HouseholdMemberId = member.member_id.into();
        let allowed = principal.member_id == Some(member_id)
            || principal.has(mmp_core::domain::Permission::AccountAdmin)
            || state
                .household
                .can_manage_member_meal_plan(principal.user_id, member_id)
                .await?;
        if !allowed {
            return Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "forbidden",
                "Forbidden",
                "You cannot record this person's meal.",
            ));
        }
    }
    if !body.guests.is_empty() && !principal.has(mmp_core::domain::Permission::HouseholdWrite) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Forbidden",
            "You cannot record guest meals.",
        ));
    }
    require_entry_access(&state, &principal, &current.entry).await?;
    let updated = state
        .meal_plan
        .review_outcomes(id, revision, body.into_domain(principal.user_id))
        .await?;
    Ok(Tagged(updated.entry.revision, updated.into()))
}
