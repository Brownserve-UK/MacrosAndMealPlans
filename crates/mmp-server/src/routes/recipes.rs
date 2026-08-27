use axum::Json;
use axum::extract::{Path, Query, State};
use mmp_core::domain::{RecipeId, UserId};
use mmp_core::ports::{PageRequest, RecipeQuery};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::{Permission, Principal};
use crate::dto::{
    CreateRecipeRequest, RecipeDto, RecipeListQuery, RecipeNutritionDto,
    RecipeNutritionPreviewRequest, RecipePage, UpdateRecipeRequest,
};
use crate::error::ApiResult;
use crate::http::{Created, IfMatch, Tagged};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(get_one, update))
        .routes(routes!(archive))
        .routes(routes!(unarchive))
        .routes(routes!(nutrition))
        .routes(routes!(nutrition_preview))
}

fn to_query(owner: UserId, query: RecipeListQuery) -> RecipeQuery {
    RecipeQuery {
        owner_id: owner,
        search: query.q.filter(|q| !q.trim().is_empty()),
        include_archived: query.include_archived.unwrap_or(false),
        page: PageRequest::new(
            query.page.unwrap_or(1),
            query.per_page.unwrap_or(PageRequest::DEFAULT_PER_PAGE),
        ),
        sort: query.sort.map(Into::into).unwrap_or_default(),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/recipes",
    params(RecipeListQuery),
    operation_id = "listRecipes",
    responses(
        (status = 200, description = "A page of the signed-in user's recipes", body = RecipePage),
        (status = 401, description = "Authentication required", body = crate::error::Problem),
    ),
    tag = "recipes",
    security(("basic" = []))
)]
async fn list(
    State(state): State<AppState>,
    principal: Principal,
    Query(query): Query<RecipeListQuery>,
) -> ApiResult<Json<RecipePage>> {
    principal.require(Permission::CatalogueRead)?;
    let page = state
        .recipes
        .list_recipes(&to_query(principal.user_id, query))
        .await?;
    Ok(Json(page.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/recipes",
    request_body = CreateRecipeRequest,
    operation_id = "createRecipe",
    responses(
        (status = 201, description = "Created", body = RecipeDto),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "recipes",
    security(("basic" = []))
)]
async fn create(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreateRecipeRequest>,
) -> ApiResult<Created<RecipeDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let created = state
        .recipes
        .create_recipe(body.into_domain(principal.user_id))
        .await?;
    Ok(Created(created.revision, created.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/recipes/{id}",
    operation_id = "getRecipe",
    params(("id" = Uuid, Path, description = "Recipe id")),
    responses(
        (status = 200, description = "The recipe", body = RecipeDto,
         headers(("ETag" = String, description = "The revision to send back as If-Match"))),
        (status = 404, description = "Not found", body = crate::error::Problem),
    ),
    tag = "recipes",
    security(("basic" = []))
)]
async fn get_one(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Tagged<RecipeDto>> {
    principal.require(Permission::CatalogueRead)?;
    let recipe = state
        .recipes
        .get_recipe(RecipeId::from(id), principal.user_id)
        .await?;
    Ok(Tagged(recipe.revision, recipe.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/recipes/{id}",
    operation_id = "updateRecipe",
    params(
        ("id" = Uuid, Path, description = "Recipe id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = UpdateRecipeRequest,
    responses(
        (status = 200, description = "Updated", body = RecipeDto),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 428, description = "If-Match is required", body = crate::error::Problem),
    ),
    tag = "recipes",
    security(("basic" = []))
)]
async fn update(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<UpdateRecipeRequest>,
) -> ApiResult<Tagged<RecipeDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let updated = state
        .recipes
        .update_recipe(RecipeId::from(id), revision, body.into(), principal.user_id)
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/recipes/{id}/archive",
    operation_id = "archiveRecipe",
    params(
        ("id" = Uuid, Path, description = "Recipe id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Archived", body = RecipeDto)),
    tag = "recipes",
    security(("basic" = []))
)]
async fn archive(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<RecipeDto>> {
    set_archived(state, principal, id, revision, true).await
}

#[utoipa::path(
    post,
    path = "/api/v1/recipes/{id}/unarchive",
    operation_id = "unarchiveRecipe",
    params(
        ("id" = Uuid, Path, description = "Recipe id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Restored", body = RecipeDto)),
    tag = "recipes",
    security(("basic" = []))
)]
async fn unarchive(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<RecipeDto>> {
    set_archived(state, principal, id, revision, false).await
}

async fn set_archived(
    state: AppState,
    principal: Principal,
    id: Uuid,
    revision: mmp_core::domain::Revision,
    archived: bool,
) -> ApiResult<Tagged<RecipeDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let updated = state
        .recipes
        .set_recipe_archived(RecipeId::from(id), revision, archived, principal.user_id)
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/recipes/{id}/nutrition",
    operation_id = "getRecipeNutrition",
    params(("id" = Uuid, Path, description = "Recipe id")),
    responses(
        (status = 200, description = "Derived per-serving nutrition", body = RecipeNutritionDto),
        (status = 404, description = "Not found", body = crate::error::Problem),
    ),
    tag = "recipes",
    security(("basic" = []))
)]
async fn nutrition(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RecipeNutritionDto>> {
    principal.require(Permission::CatalogueRead)?;
    let nutrition = state
        .recipes
        .nutrition_for(RecipeId::from(id), principal.user_id)
        .await?;
    Ok(Json(nutrition.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/recipes/nutrition-preview",
    request_body = RecipeNutritionPreviewRequest,
    operation_id = "previewRecipeNutrition",
    responses(
        (status = 200, description = "Derived per-serving nutrition for an unsaved draft", body = RecipeNutritionDto),
    ),
    tag = "recipes",
    security(("basic" = []))
)]
async fn nutrition_preview(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<RecipeNutritionPreviewRequest>,
) -> ApiResult<Json<RecipeNutritionDto>> {
    principal.require(Permission::CatalogueRead)?;
    let components: Vec<mmp_core::domain::NewRecipeComponent> =
        body.components.into_iter().map(Into::into).collect();
    let nutrition = state
        .recipes
        .nutrition_preview(body.servings, &components)
        .await?;
    Ok(Json(nutrition.into()))
}
