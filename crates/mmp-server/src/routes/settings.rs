use axum::Json;
use axum::extract::State;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::auth::{Permission, Principal};
use crate::dto::{HouseholdSettingsDto, UpdateMealTimesRequest};
use crate::error::ApiResult;
use crate::http::{IfMatch, Tagged};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_meal_times, update_meal_times))
}

#[utoipa::path(
    get,
    path = "/api/v1/household/meal-times",
    operation_id = "getHouseholdMealTimes",
    responses(
        (status = 200, description = "The household's default meal times", body = HouseholdSettingsDto,
         headers(("ETag" = String, description = "The revision to send back as If-Match"))),
    ),
    tag = "household",
    security(("basic" = []))
)]
async fn get_meal_times(
    State(state): State<AppState>,
    _principal: Principal,
) -> ApiResult<Tagged<HouseholdSettingsDto>> {
    let settings = state.household_settings.get().await?;
    Ok(Tagged(settings.revision, settings.into()))
}

#[utoipa::path(
    put,
    path = "/api/v1/household/meal-times",
    operation_id = "updateHouseholdMealTimes",
    params(("If-Match" = String, Header, description = "The revision you loaded")),
    request_body = UpdateMealTimesRequest,
    responses(
        (status = 200, description = "Updated", body = HouseholdSettingsDto),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 428, description = "If-Match is required", body = crate::error::Problem),
    ),
    tag = "household",
    security(("basic" = []))
)]
async fn update_meal_times(
    State(state): State<AppState>,
    principal: Principal,
    IfMatch(revision): IfMatch,
    Json(body): Json<UpdateMealTimesRequest>,
) -> ApiResult<Tagged<HouseholdSettingsDto>> {
    principal.require(Permission::HouseholdWrite)?;
    let updated = state
        .household_settings
        .update(revision, body.into())
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}
