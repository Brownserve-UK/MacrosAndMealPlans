use axum::Json;
use axum::extract::State;
use mmp_core::domain::{Permission, Role};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::auth::Principal;
use crate::dto::{FoodMappingKindDto, FoodMappingReviewDto, NeedsReviewDto};
use crate::error::ApiResult;
use crate::state::AppState;

use super::meal_plan::personal_member;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(needs_review))
}

#[utoipa::path(
    get,
    path = "/api/v1/needs-review",
    operation_id = "getNeedsReview",
    responses((status = 200, body = NeedsReviewDto)),
    tag = "needs-review",
    security(("basic" = []))
)]
async fn needs_review(
    State(state): State<AppState>,
    principal: Principal,
) -> ApiResult<Json<NeedsReviewDto>> {
    let member = personal_member(&state, &principal).await?;
    let include_household = principal.has(Permission::HouseholdWrite);
    let meal_review = state
        .meal_plan
        .needs_review(member, include_household)
        .await?;
    let food_mappings = if principal.has(Permission::CatalogueWrite) {
        let foods = state
            .recipes
            .foods_needing_products(principal.user_id, principal.roles.contains(&Role::Admin))
            .await?;
        foods
            .ingredients
            .into_iter()
            .map(|ingredient| FoodMappingReviewDto {
                id: ingredient.id.as_uuid(),
                name: ingredient.name,
                kind: FoodMappingKindDto::Ingredient,
            })
            .chain(
                foods
                    .prepared_meals
                    .into_iter()
                    .map(|prepared_meal| FoodMappingReviewDto {
                        id: prepared_meal.id.as_uuid(),
                        name: prepared_meal.name,
                        kind: FoodMappingKindDto::PreparedMeal,
                    }),
            )
            .collect()
    } else {
        Vec::new()
    };

    Ok(Json(NeedsReviewDto {
        personal_meals: meal_review.personal.into_iter().map(Into::into).collect(),
        household_meals: meal_review.household.into_iter().map(Into::into).collect(),
        food_mappings,
    }))
}
