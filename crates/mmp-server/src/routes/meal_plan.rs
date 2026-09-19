use std::collections::{HashMap, HashSet};

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use mmp_core::domain::{
    HouseholdMemberId, MealGuestGroupId, MealOccasionId, MealPlanComponentId, MealPlanEntryId,
    Patch, Permission,
};
use mmp_core::services::{GuestChange, GuestMealTarget};
use mmp_core::services::{MealGroupView, MealOccasionView};
use serde::Deserialize;
use time::{Date, Weekday};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::Principal;
use crate::dto::common::iso_date;
use crate::dto::{
    CreateOccasionRequest, GroupPatchRequest, GroupViewDto, MarkMealPlanComponentEatenRequest,
    MarkMealPlanEatenRequest, MealPlanEntryDto, MealPlanWeekDto, MoveOrCopyOccasionRequest,
    NewGroupRequest, OccasionPatchRequest, OccasionViewDto, PlannerWeekDto,
    ReviewMealOutcomesRequest, SetAttendanceRequest, UpdateMealPlanEntryRequest,
};
use crate::error::{ApiError, ApiResult};
use crate::http::{Created, IfMatch, Tagged};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_week))
        .routes(routes!(get_planner_week))
        .routes(routes!(create_occasion))
        .routes(routes!(update_occasion, delete_occasion))
        .routes(routes!(move_occasion))
        .routes(routes!(copy_occasion))
        .routes(routes!(add_group))
        .routes(routes!(add_guest))
        .routes(routes!(change_guest, remove_guest))
        .routes(routes!(split_guests))
        .routes(routes!(update_group, delete_group))
        .routes(routes!(set_attendance))
        .routes(routes!(copy_week))
        .routes(routes!(get_one, update, delete))
        .routes(routes!(mark_eaten))
        .routes(routes!(mark_not_eaten))
        .routes(routes!(reopen))
        .routes(routes!(mark_component_eaten))
        .routes(routes!(mark_component_not_eaten))
        .routes(routes!(reopen_component))
        .routes(routes!(review_outcomes))
}

#[derive(Debug, Deserialize, ToSchema)]
struct AddGuestRequest {
    name: Option<String>,
    note: Option<String>,
    group_id: Option<Uuid>,
    new_group: Option<NewGroupRequest>,
}

#[derive(Debug, Deserialize, ToSchema)]
struct ChangeGuestRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    name: Patch<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    note: Patch<String>,
    group_id: Option<Uuid>,
    new_group: Option<NewGroupRequest>,
}

fn guest_target(
    group_id: Option<Uuid>,
    new_group: Option<NewGroupRequest>,
) -> ApiResult<GuestMealTarget> {
    match (group_id, new_group) {
        (Some(id), None) => Ok(GuestMealTarget::Existing(id.into())),
        (None, Some(group)) => Ok(GuestMealTarget::New(group.into_domain())),
        _ => Err(ApiError::bad_request("Choose one existing or new meal.")),
    }
}

async fn guest_occasion_response(
    state: &AppState,
    view: MealOccasionView,
) -> ApiResult<Tagged<OccasionViewDto>> {
    let revision = view.occasion.revision;
    let to_buy = to_buy_counts(state).await?;
    Ok(Tagged(
        revision,
        OccasionViewDto::build(view, |gid| *to_buy.get(&gid).unwrap_or(&0)),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/planner/occasions/{id}/guests",
    operation_id = "addPlannerGuest",
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    request_body = AddGuestRequest,
    responses((status = 200, body = OccasionViewDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn add_guest(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<AddGuestRequest>,
) -> ApiResult<Tagged<OccasionViewDto>> {
    personal_member(&state, &principal).await?;
    let target = guest_target(body.group_id, body.new_group)?;
    let view = state
        .meal_plan
        .add_guest(
            id.into(),
            revision,
            body.name,
            body.note,
            target,
            principal.user_id,
        )
        .await?;
    guest_occasion_response(&state, view).await
}

#[utoipa::path(
    patch,
    path = "/api/v1/planner/occasions/{id}/guests/{guest_id}",
    operation_id = "changePlannerGuest",
    params(("id" = Uuid, Path), ("guest_id" = Uuid, Path), ("If-Match" = String, Header)),
    request_body = ChangeGuestRequest,
    responses((status = 200, body = OccasionViewDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn change_guest(
    State(state): State<AppState>,
    principal: Principal,
    Path((id, guest_id)): Path<(Uuid, Uuid)>,
    IfMatch(revision): IfMatch,
    Json(body): Json<ChangeGuestRequest>,
) -> ApiResult<Tagged<OccasionViewDto>> {
    personal_member(&state, &principal).await?;
    let target = if body.group_id.is_some() || body.new_group.is_some() {
        Some(guest_target(body.group_id, body.new_group)?)
    } else {
        None
    };
    let name = match body.name {
        Patch::Unchanged => None,
        Patch::Clear => Some(None),
        Patch::Set(value) => Some(Some(value)),
    };
    let note = match body.note {
        Patch::Unchanged => None,
        Patch::Clear => Some(None),
        Patch::Set(value) => Some(Some(value)),
    };
    let view = state
        .meal_plan
        .change_guest(
            id.into(),
            revision,
            MealGuestGroupId::from(guest_id),
            GuestChange::Update { name, note, target },
            principal.user_id,
        )
        .await?;
    guest_occasion_response(&state, view).await
}

#[utoipa::path(
    delete,
    path = "/api/v1/planner/occasions/{id}/guests/{guest_id}",
    operation_id = "removePlannerGuest",
    params(("id" = Uuid, Path), ("guest_id" = Uuid, Path), ("If-Match" = String, Header)),
    responses((status = 200, body = OccasionViewDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn remove_guest(
    State(state): State<AppState>,
    principal: Principal,
    Path((id, guest_id)): Path<(Uuid, Uuid)>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<OccasionViewDto>> {
    personal_member(&state, &principal).await?;
    let view = state
        .meal_plan
        .change_guest(
            id.into(),
            revision,
            MealGuestGroupId::from(guest_id),
            GuestChange::Remove,
            principal.user_id,
        )
        .await?;
    guest_occasion_response(&state, view).await
}

#[utoipa::path(
    post,
    path = "/api/v1/planner/occasions/{id}/guests/{guest_id}/split",
    operation_id = "splitPlannerGuests",
    params(("id" = Uuid, Path), ("guest_id" = Uuid, Path), ("If-Match" = String, Header)),
    responses((status = 200, body = OccasionViewDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn split_guests(
    State(state): State<AppState>,
    principal: Principal,
    Path((id, guest_id)): Path<(Uuid, Uuid)>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<OccasionViewDto>> {
    personal_member(&state, &principal).await?;
    let view = state
        .meal_plan
        .split_guests(
            id.into(),
            revision,
            MealGuestGroupId::from(guest_id),
            principal.user_id,
        )
        .await?;
    guest_occasion_response(&state, view).await
}

fn entry_id(id: Uuid) -> MealPlanEntryId {
    id.into()
}

fn component_id(id: Uuid) -> MealPlanComponentId {
    id.into()
}

fn not_found(what: &str) -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "not-found", "Not found", what)
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

async fn can_manage_personal_meal(
    state: &AppState,
    principal: &Principal,
    member_id: HouseholdMemberId,
) -> ApiResult<bool> {
    if principal.roles.contains(&mmp_core::domain::Role::Admin)
        || principal.member_id == Some(member_id)
    {
        return Ok(true);
    }
    Ok(state
        .household
        .can_manage_member_meal_plan(principal.user_id, member_id)
        .await?)
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

async fn to_buy_counts(state: &AppState) -> ApiResult<HashMap<Uuid, i64>> {
    let list = state.shopping.requirements(None).await?;
    let mut counts: HashMap<Uuid, i64> = HashMap::new();
    for requirement in &list.requirements {
        if !requirement.purchases.is_empty() {
            continue;
        }
        let mut seen = HashSet::new();
        for claim in &requirement.claims {
            if seen.insert(claim.entry_id) {
                *counts.entry(claim.entry_id.as_uuid()).or_insert(0) += 1;
            }
        }
    }
    Ok(counts)
}

fn group_in(view: &MealOccasionView, id: MealPlanEntryId) -> ApiResult<MealGroupView> {
    view.groups
        .iter()
        .find(|group| group.entry.entry.id == id)
        .cloned()
        .ok_or_else(|| not_found("That meal could not be found."))
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
    let objective = state.weight.goal(member).await?.map(|goal| goal.objective);
    Ok(Json(
        MealPlanWeekDto::from(week).with_calorie_direction(objective),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/planner/{week_start}",
    operation_id = "getPlannerWeek",
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
    let week_start = parse_week_start(&week_start)?;
    personal_member(&state, &principal).await?;
    let week = state.meal_plan.planner_week(week_start).await?;
    let to_buy = to_buy_counts(&state).await?;
    Ok(Json(PlannerWeekDto::build(week, |id| {
        *to_buy.get(&id).unwrap_or(&0)
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/planner/occasions",
    operation_id = "createPlannerOccasion",
    request_body = CreateOccasionRequest,
    responses((status = 201, body = OccasionViewDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn create_occasion(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreateOccasionRequest>,
) -> ApiResult<Created<OccasionViewDto>> {
    personal_member(&state, &principal).await?;
    let input = body.into_domain(principal.user_id);
    let view = state.meal_plan.create_occasion(input).await?;
    let to_buy = to_buy_counts(&state).await?;
    let revision = view.occasion.revision;
    Ok(Created(
        revision,
        OccasionViewDto::build(view, |id| *to_buy.get(&id).unwrap_or(&0)),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/planner/occasions/{id}",
    operation_id = "updatePlannerOccasion",
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    request_body = OccasionPatchRequest,
    responses((status = 200, body = OccasionViewDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn update_occasion(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<OccasionPatchRequest>,
) -> ApiResult<Tagged<OccasionViewDto>> {
    personal_member(&state, &principal).await?;
    let patch = body.into_domain().map_err(|message| {
        let mut errors = mmp_core::ValidationErrors::new();
        errors.push("planned_time", message);
        mmp_core::CoreError::Validation(errors)
    })?;
    let view: MealOccasionView = state
        .meal_plan
        .update_occasion(MealOccasionId::from(id), revision, patch, principal.user_id)
        .await?;
    let to_buy = to_buy_counts(&state).await?;
    let out_revision = view.occasion.revision;
    Ok(Tagged(
        out_revision,
        OccasionViewDto::build(view, |gid| *to_buy.get(&gid).unwrap_or(&0)),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/planner/occasions/{id}",
    operation_id = "deletePlannerOccasion",
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    responses((status = 204)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn delete_occasion(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<StatusCode> {
    personal_member(&state, &principal).await?;
    state
        .meal_plan
        .delete_occasion(MealOccasionId::from(id), revision)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/v1/planner/occasions/{id}/move",
    operation_id = "movePlannerOccasion",
    params(("id" = Uuid, Path)),
    request_body = MoveOrCopyOccasionRequest,
    responses((status = 200, body = OccasionViewDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn move_occasion(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    Json(body): Json<MoveOrCopyOccasionRequest>,
) -> ApiResult<Json<OccasionViewDto>> {
    personal_member(&state, &principal).await?;
    let occasion_id = MealOccasionId::from(id);
    let current = state.meal_plan.get_occasion(occasion_id).await?;
    let view = state
        .meal_plan
        .move_occasion(
            occasion_id,
            current.occasion.revision,
            body.planned_on,
            body.slot,
            principal.user_id,
        )
        .await?;
    let to_buy = to_buy_counts(&state).await?;
    Ok(Json(OccasionViewDto::build(view, |gid| {
        *to_buy.get(&gid).unwrap_or(&0)
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/planner/occasions/{id}/copy",
    operation_id = "copyPlannerOccasion",
    params(("id" = Uuid, Path)),
    request_body = MoveOrCopyOccasionRequest,
    responses((status = 200, body = OccasionViewDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn copy_occasion(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    Json(body): Json<MoveOrCopyOccasionRequest>,
) -> ApiResult<Json<OccasionViewDto>> {
    personal_member(&state, &principal).await?;
    let view = state
        .meal_plan
        .copy_occasion(
            MealOccasionId::from(id),
            body.planned_on,
            body.slot,
            principal.user_id,
        )
        .await?;
    let to_buy = to_buy_counts(&state).await?;
    Ok(Json(OccasionViewDto::build(view, |gid| {
        *to_buy.get(&gid).unwrap_or(&0)
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/planner/occasions/{id}/groups",
    operation_id = "addPlannerGroup",
    params(("id" = Uuid, Path)),
    request_body = NewGroupRequest,
    responses((status = 201, body = GroupViewDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn add_group(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    Json(body): Json<NewGroupRequest>,
) -> ApiResult<Created<GroupViewDto>> {
    personal_member(&state, &principal).await?;
    let group_input = body.into_domain();
    let group_id = group_input
        .id
        .expect("group id is always pre-assigned by the request DTO");
    let view = state
        .meal_plan
        .add_group(MealOccasionId::from(id), group_input, principal.user_id)
        .await?;
    let group = group_in(&view, group_id)?;
    let to_buy = to_buy_counts(&state).await?;
    let revision = group.entry.entry.revision;
    Ok(Created(
        revision,
        GroupViewDto::build(group, *to_buy.get(&group_id.as_uuid()).unwrap_or(&0)),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/planner/groups/{id}",
    operation_id = "updatePlannerGroup",
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    request_body = GroupPatchRequest,
    responses((status = 200, body = GroupViewDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn update_group(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<GroupPatchRequest>,
) -> ApiResult<Tagged<GroupViewDto>> {
    personal_member(&state, &principal).await?;
    let group_id = entry_id(id);
    let patch = body.into_domain();
    let view = state
        .meal_plan
        .update_group(group_id, revision, patch, principal.user_id)
        .await?;
    let group = group_in(&view, group_id)?;
    let to_buy = to_buy_counts(&state).await?;
    let out_revision = group.entry.entry.revision;
    Ok(Tagged(
        out_revision,
        GroupViewDto::build(group, *to_buy.get(&id).unwrap_or(&0)),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/planner/groups/{id}",
    operation_id = "deletePlannerGroup",
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    responses((status = 204)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn delete_group(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<StatusCode> {
    personal_member(&state, &principal).await?;
    state
        .meal_plan
        .delete_group(entry_id(id), revision, principal.user_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/api/v1/planner/occasions/{id}/attendance/{member_id}",
    operation_id = "setPlannerAttendance",
    params(("id" = Uuid, Path), ("member_id" = Uuid, Path)),
    request_body = SetAttendanceRequest,
    responses((status = 200, body = OccasionViewDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn set_attendance(
    State(state): State<AppState>,
    principal: Principal,
    Path((id, member_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<SetAttendanceRequest>,
) -> ApiResult<Json<OccasionViewDto>> {
    personal_member(&state, &principal).await?;
    let target: HouseholdMemberId = member_id.into();
    if principal.member_id != Some(target) && !principal.has(Permission::HouseholdWrite) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Forbidden",
            "You cannot set someone else's attendance.",
        ));
    }
    let view = state
        .meal_plan
        .set_attendance(
            MealOccasionId::from(id),
            target,
            body.attendance.into_domain(),
            principal.user_id,
        )
        .await?;
    let to_buy = to_buy_counts(&state).await?;
    Ok(Json(OccasionViewDto::build(view, |gid| {
        *to_buy.get(&gid).unwrap_or(&0)
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/planner/{week_start}/copy-from/{source_week_start}",
    operation_id = "copyPlannerWeek",
    params(
        ("week_start" = String, Path, example = "2026-08-24"),
        ("source_week_start" = String, Path, example = "2026-08-17"),
    ),
    responses((status = 200, body = PlannerWeekDto)),
    tag = "meal-plan",
    security(("basic" = []))
)]
async fn copy_week(
    State(state): State<AppState>,
    principal: Principal,
    Path((week_start, source_week_start)): Path<(String, String)>,
) -> ApiResult<Json<PlannerWeekDto>> {
    personal_member(&state, &principal).await?;
    let week_start = parse_week_start(&week_start)?;
    let source_week_start = parse_week_start(&source_week_start)?;
    let week = state
        .meal_plan
        .copy_week(week_start, source_week_start, principal.user_id)
        .await?;
    let to_buy = to_buy_counts(&state).await?;
    Ok(Json(PlannerWeekDto::build(week, |id| {
        *to_buy.get(&id).unwrap_or(&0)
    })))
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
    personal_member(&state, &principal).await?;
    let entry = state.meal_plan.get(entry_id(id)).await?;
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
    personal_member(&state, &principal).await?;
    let id = entry_id(id);
    let patch = body.into_domain();
    state
        .meal_plan
        .update_group(id, revision, patch, principal.user_id)
        .await?;
    let updated = state.meal_plan.get(id).await?;
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
    personal_member(&state, &principal).await?;
    let id = entry_id(id);
    state
        .meal_plan
        .delete_group(id, revision, principal.user_id)
        .await?;
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
    let subject = body
        .member_id
        .map(Into::into)
        .or(principal.member_id)
        .ok_or_else(|| ApiError::bad_request("Choose whose meal to record."))?;
    require_outcome_access(&state, &principal, subject).await?;
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
    let subject = principal.member_id.ok_or_else(|| {
        ApiError::bad_request("Your account is not linked to a household member.")
    })?;
    require_outcome_access(&state, &principal, subject).await?;
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
    let subject = principal.member_id.ok_or_else(|| {
        ApiError::bad_request("Your account is not linked to a household member.")
    })?;
    require_outcome_access(&state, &principal, subject).await?;
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
    let subject = body
        .member_id
        .map(Into::into)
        .or(principal.member_id)
        .ok_or_else(|| ApiError::bad_request("Choose whose meal to record."))?;
    require_outcome_access(&state, &principal, subject).await?;
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
    let subject = principal.member_id.ok_or_else(|| {
        ApiError::bad_request("Your account is not linked to a household member.")
    })?;
    require_outcome_access(&state, &principal, subject).await?;
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
    let subject = principal.member_id.ok_or_else(|| {
        ApiError::bad_request("Your account is not linked to a household member.")
    })?;
    require_outcome_access(&state, &principal, subject).await?;
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
    let updated = state
        .meal_plan
        .review_outcomes(id, revision, body.into_domain(principal.user_id))
        .await?;
    Ok(Tagged(updated.entry.revision, updated.into()))
}
