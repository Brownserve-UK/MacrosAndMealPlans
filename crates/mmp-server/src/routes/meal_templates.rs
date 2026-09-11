use axum::Json;
use axum::extract::{Path, Query, State};
use mmp_core::domain::{
    MealItemRef, MealPlanEntryId, MealTemplate, MealTemplateComponent, MealTemplateId, UserId,
};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::Principal;
use crate::dto::{
    CreateMealTemplateFromEntryRequest, CreateMealTemplateRequest, MealTemplateDto,
    MealTemplateListQuery, MealTemplatePage, UpdateMealTemplateRequest,
};
use crate::error::ApiResult;
use crate::http::{Created, IfMatch, Tagged};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(get_one, update))
        .routes(routes!(delete))
        .routes(routes!(create_from_entry))
}

async fn resolve_names(
    state: &AppState,
    owner: UserId,
    components: &[MealTemplateComponent],
) -> Vec<String> {
    let mut names = Vec::with_capacity(components.len());
    for component in components {
        let name = match component.item {
            MealItemRef::Product { product_id } => state
                .catalogue
                .get_product(product_id)
                .await
                .map(|product| product.name)
                .unwrap_or_else(|_| "Product".to_owned()),
            MealItemRef::Recipe { recipe_id } => state
                .recipes
                .get_recipe(recipe_id, owner)
                .await
                .map(|recipe| recipe.name)
                .unwrap_or_else(|_| "Recipe".to_owned()),
            MealItemRef::Ingredient { ingredient_id } => state
                .catalogue
                .get_ingredient(ingredient_id)
                .await
                .map(|ingredient| ingredient.name)
                .unwrap_or_else(|_| "Food".to_owned()),
            MealItemRef::PreparedMeal { prepared_meal_id } => state
                .catalogue
                .get_prepared_meal(prepared_meal_id)
                .await
                .map(|prepared_meal| prepared_meal.name)
                .unwrap_or_else(|_| "Food".to_owned()),
            MealItemRef::Dish { .. } => "Cooked food".to_owned(),
        };
        names.push(name);
    }
    names
}

async fn to_dto(state: &AppState, owner: UserId, template: MealTemplate) -> MealTemplateDto {
    let names = resolve_names(state, owner, &template.components).await;
    MealTemplateDto::from_domain(template, &names)
}

#[utoipa::path(
    get,
    path = "/api/v1/meal-templates",
    params(MealTemplateListQuery),
    operation_id = "listMealTemplates",
    responses(
        (status = 200, description = "A page of the signed-in user's saved meals", body = MealTemplatePage),
        (status = 401, description = "Authentication required", body = crate::error::Problem),
    ),
    tag = "meal-templates",
    security(("basic" = []))
)]
async fn list(
    State(state): State<AppState>,
    principal: Principal,
    Query(query): Query<MealTemplateListQuery>,
) -> ApiResult<Json<MealTemplatePage>> {
    let page = state
        .meal_templates
        .list(&query.into_domain(principal.user_id))
        .await?;
    let mut names = Vec::with_capacity(page.items.len());
    for template in &page.items {
        names.push(resolve_names(&state, principal.user_id, &template.components).await);
    }
    Ok(Json(MealTemplatePage::from_domain(page, &names)))
}

#[utoipa::path(
    post,
    path = "/api/v1/meal-templates",
    request_body = CreateMealTemplateRequest,
    operation_id = "createMealTemplate",
    responses(
        (status = 201, description = "Created", body = MealTemplateDto),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "meal-templates",
    security(("basic" = []))
)]
async fn create(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreateMealTemplateRequest>,
) -> ApiResult<Created<MealTemplateDto>> {
    let created = state
        .meal_templates
        .create(body.into_domain(principal.user_id))
        .await?;
    let revision = created.revision;
    let dto = to_dto(&state, principal.user_id, created).await;
    Ok(Created(revision, dto))
}

#[utoipa::path(
    get,
    path = "/api/v1/meal-templates/{id}",
    operation_id = "getMealTemplate",
    params(("id" = Uuid, Path, description = "Saved meal id")),
    responses(
        (status = 200, description = "The saved meal", body = MealTemplateDto,
         headers(("ETag" = String, description = "The revision to send back as If-Match"))),
        (status = 404, description = "Not found", body = crate::error::Problem),
    ),
    tag = "meal-templates",
    security(("basic" = []))
)]
async fn get_one(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Tagged<MealTemplateDto>> {
    let template = state
        .meal_templates
        .get(MealTemplateId::from(id), principal.user_id)
        .await?;
    let revision = template.revision;
    let dto = to_dto(&state, principal.user_id, template).await;
    Ok(Tagged(revision, dto))
}

#[utoipa::path(
    patch,
    path = "/api/v1/meal-templates/{id}",
    operation_id = "updateMealTemplate",
    params(
        ("id" = Uuid, Path, description = "Saved meal id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = UpdateMealTemplateRequest,
    responses(
        (status = 200, description = "Updated", body = MealTemplateDto),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 428, description = "If-Match is required", body = crate::error::Problem),
    ),
    tag = "meal-templates",
    security(("basic" = []))
)]
async fn update(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<UpdateMealTemplateRequest>,
) -> ApiResult<Tagged<MealTemplateDto>> {
    let updated = state
        .meal_templates
        .update(
            MealTemplateId::from(id),
            principal.user_id,
            revision,
            body.into(),
        )
        .await?;
    let updated_revision = updated.revision;
    let dto = to_dto(&state, principal.user_id, updated).await;
    Ok(Tagged(updated_revision, dto))
}

#[utoipa::path(
    delete,
    path = "/api/v1/meal-templates/{id}",
    operation_id = "deleteMealTemplate",
    params(
        ("id" = Uuid, Path, description = "Saved meal id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
    ),
    tag = "meal-templates",
    security(("basic" = []))
)]
async fn delete(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<axum::http::StatusCode> {
    state
        .meal_templates
        .delete(MealTemplateId::from(id), principal.user_id, revision)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/v1/meal-templates/from-entry/{entry_id}",
    operation_id = "createMealTemplateFromEntry",
    params(("entry_id" = Uuid, Path, description = "Meal plan entry id")),
    request_body = CreateMealTemplateFromEntryRequest,
    responses(
        (status = 201, description = "Created", body = MealTemplateDto),
        (status = 404, description = "Not found", body = crate::error::Problem),
        (status = 422, description = "Nothing in the meal can be saved", body = crate::error::Problem),
    ),
    tag = "meal-templates",
    security(("basic" = []))
)]
async fn create_from_entry(
    State(state): State<AppState>,
    principal: Principal,
    Path(entry_id): Path<Uuid>,
    Json(body): Json<CreateMealTemplateFromEntryRequest>,
) -> ApiResult<Created<MealTemplateDto>> {
    let created = state
        .meal_templates
        .from_entry(
            MealPlanEntryId::from(entry_id),
            principal.user_id,
            body.name,
        )
        .await?;
    let revision = created.revision;
    let dto = to_dto(&state, principal.user_id, created).await;
    Ok(Created(revision, dto))
}
