use axum::Json;
use axum::extract::{Path, State};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use super::require_member_access;
use crate::auth::Principal;
use crate::dto::{
    GuidedNutritionPlanDto, ManualCalorieTargetRequest, MemberBodyProfileDto,
    NutritionPlanAnswersRequest, NutritionPlanDto, NutritionPlanRecommendationDto,
    UpdateBodyProfileRequest, member_id,
};
use crate::error::ApiResult;
use crate::http::{IfMatch, Tagged};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_body_profile, update_body_profile))
        .routes(routes!(get_nutrition_plan))
        .routes(routes!(preview_calorie_target))
        .routes(routes!(set_guided_calorie_target))
        .routes(routes!(set_manual_calorie_target))
}

#[utoipa::path(
    get,
    path = "/api/v1/members/{member_id}/body-profile",
    operation_id = "getMemberBodyProfile",
    params(("member_id" = Uuid, Path, description = "Household member id")),
    responses(
        (status = 200, description = "The member's body profile", body = MemberBodyProfileDto,
         headers(("ETag" = String, description = "The revision to send back as If-Match"))),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 404, description = "No body profile set", body = crate::error::Problem),
    ),
    tag = "nutrition-plan",
    security(("basic" = []))
)]
async fn get_body_profile(
    State(state): State<AppState>,
    principal: Principal,
    Path(member): Path<Uuid>,
) -> ApiResult<Tagged<MemberBodyProfileDto>> {
    let member = member_id(member);
    require_member_access(&state, &principal, member).await?;
    let profile = state
        .nutrition_plan
        .body_profile(member)
        .await?
        .ok_or_else(|| mmp_core::CoreError::not_found("member body profile", member))?;
    Ok(Tagged(profile.revision, profile.into()))
}

#[utoipa::path(
    put,
    path = "/api/v1/members/{member_id}/body-profile",
    operation_id = "updateMemberBodyProfile",
    params(
        ("member_id" = Uuid, Path, description = "Household member id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = UpdateBodyProfileRequest,
    responses(
        (status = 200, description = "Updated", body = MemberBodyProfileDto),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 404, description = "No body profile set", body = crate::error::Problem),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
        (status = 428, description = "If-Match is required", body = crate::error::Problem),
    ),
    tag = "nutrition-plan",
    security(("basic" = []))
)]
async fn update_body_profile(
    State(state): State<AppState>,
    principal: Principal,
    Path(member): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<UpdateBodyProfileRequest>,
) -> ApiResult<Tagged<MemberBodyProfileDto>> {
    let member = member_id(member);
    require_member_access(&state, &principal, member).await?;
    let profile = state
        .nutrition_plan
        .update_body_profile(member, revision, body.into())
        .await?;
    Ok(Tagged(profile.revision, profile.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/members/{member_id}/nutrition-plan",
    operation_id = "getNutritionPlan",
    params(("member_id" = Uuid, Path, description = "Household member id")),
    responses(
        (status = 200, description = "The current calorie target and its calculation", body = NutritionPlanDto),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
    ),
    tag = "nutrition-plan",
    security(("basic" = []))
)]
async fn get_nutrition_plan(
    State(state): State<AppState>,
    principal: Principal,
    Path(member): Path<Uuid>,
) -> ApiResult<Json<NutritionPlanDto>> {
    let member = member_id(member);
    require_member_access(&state, &principal, member).await?;
    let plan = state.nutrition_plan.current(member).await?;
    Ok(Json(plan.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/members/{member_id}/calorie-target/preview",
    operation_id = "previewCalorieTarget",
    params(("member_id" = Uuid, Path, description = "Household member id")),
    request_body = NutritionPlanAnswersRequest,
    responses(
        (status = 200, description = "The calculated recommendation without saving it", body = NutritionPlanRecommendationDto),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "nutrition-plan",
    security(("basic" = []))
)]
async fn preview_calorie_target(
    State(state): State<AppState>,
    principal: Principal,
    Path(member): Path<Uuid>,
    Json(body): Json<NutritionPlanAnswersRequest>,
) -> ApiResult<Json<NutritionPlanRecommendationDto>> {
    let member = member_id(member);
    require_member_access(&state, &principal, member).await?;
    let calculation = state
        .nutrition_plan
        .preview(body.into_domain(member, principal.user_id))
        .await?;
    Ok(Json(calculation.into()))
}

#[utoipa::path(
    put,
    path = "/api/v1/members/{member_id}/calorie-target/guided",
    operation_id = "setGuidedCalorieTarget",
    params(("member_id" = Uuid, Path, description = "Household member id")),
    request_body = NutritionPlanAnswersRequest,
    responses(
        (status = 200, description = "Saved the guided target and its calculation", body = GuidedNutritionPlanDto),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "nutrition-plan",
    security(("basic" = []))
)]
async fn set_guided_calorie_target(
    State(state): State<AppState>,
    principal: Principal,
    Path(member): Path<Uuid>,
    Json(body): Json<NutritionPlanAnswersRequest>,
) -> ApiResult<Tagged<GuidedNutritionPlanDto>> {
    let member = member_id(member);
    require_member_access(&state, &principal, member).await?;
    let plan = state
        .nutrition_plan
        .set_guided(body.into_domain(member, principal.user_id))
        .await?;
    let revision = plan.target.revision;
    Ok(Tagged(revision, plan.into()))
}

#[utoipa::path(
    put,
    path = "/api/v1/members/{member_id}/calorie-target/manual",
    operation_id = "setManualCalorieTarget",
    params(("member_id" = Uuid, Path, description = "Household member id")),
    request_body = ManualCalorieTargetRequest,
    responses(
        (status = 200, description = "Saved the user-defined calorie target", body = crate::dto::NutritionTargetDto),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "nutrition-plan",
    security(("basic" = []))
)]
async fn set_manual_calorie_target(
    State(state): State<AppState>,
    principal: Principal,
    Path(member): Path<Uuid>,
    Json(body): Json<ManualCalorieTargetRequest>,
) -> ApiResult<Tagged<crate::dto::NutritionTargetDto>> {
    let member = member_id(member);
    require_member_access(&state, &principal, member).await?;
    let energy_kcal = body.energy_kcal;
    let macros = body.macros();
    let target = state
        .nutrition_plan
        .set_manual(member, energy_kcal, macros)
        .await?;
    Ok(Tagged(target.revision, target.into()))
}
