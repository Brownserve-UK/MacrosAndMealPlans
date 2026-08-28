use axum::Json;
use axum::extract::{Path, Query, State};
use mmp_core::domain::IngredientId;
use mmp_core::ports::{IngredientQuery, PageRequest};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::{Permission, Principal};
use crate::dto::{
    CreateIngredientRequest, IngredientDto, IngredientListQuery, IngredientPage, ProductPage,
    UpdateIngredientRequest,
};
use crate::error::{ApiError, ApiResult};
use crate::http::{Created, IfMatch, Tagged};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(get_one, update))
        .routes(routes!(archive))
        .routes(routes!(unarchive))
        .routes(routes!(products_for_ingredient))
}

fn to_query(query: IngredientListQuery) -> IngredientQuery {
    IngredientQuery {
        search: query.q.filter(|q| !q.trim().is_empty()),
        origin: query.origin,
        needs_products: query.needs_products,
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
    path = "/api/v1/ingredients",
    params(IngredientListQuery),
    operation_id = "listIngredients",
    responses(
        (status = 200, description = "A page of ingredients", body = IngredientPage),
        (status = 401, description = "Authentication required", body = crate::error::Problem),
    ),
    tag = "ingredients",
    security(("basic" = []))
)]
async fn list(
    State(state): State<AppState>,
    principal: Principal,
    Query(query): Query<IngredientListQuery>,
) -> ApiResult<Json<IngredientPage>> {
    principal.require(Permission::CatalogueRead)?;
    let page = state.catalogue.list_ingredients(&to_query(query)).await?;
    Ok(Json(page.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/ingredients",
    request_body = CreateIngredientRequest,
    operation_id = "createIngredient",
    responses(
        (status = 201, description = "Created", body = IngredientDto),
        (status = 409, description = "The name is already taken", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "ingredients",
    security(("basic" = []))
)]
async fn create(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreateIngredientRequest>,
) -> ApiResult<Created<IngredientDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let created = state.catalogue.create_ingredient(body.into()).await?;
    Ok(Created(created.revision, created.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/ingredients/{id}",
    operation_id = "getIngredient",
    params(("id" = Uuid, Path, description = "Ingredient id")),
    responses(
        (status = 200, description = "The ingredient", body = IngredientDto,
         headers(("ETag" = String, description = "The revision to send back as If-Match"))),
        (status = 404, description = "Not found", body = crate::error::Problem),
    ),
    tag = "ingredients",
    security(("basic" = []))
)]
async fn get_one(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Tagged<IngredientDto>> {
    principal.require(Permission::CatalogueRead)?;
    let ingredient = state
        .catalogue
        .get_ingredient(IngredientId::from(id))
        .await?;
    Ok(Tagged(ingredient.revision, ingredient.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/ingredients/{id}",
    operation_id = "updateIngredient",
    params(
        ("id" = Uuid, Path, description = "Ingredient id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = UpdateIngredientRequest,
    responses(
        (status = 200, description = "Updated", body = IngredientDto),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 428, description = "If-Match is required", body = crate::error::Problem),
    ),
    tag = "ingredients",
    security(("basic" = []))
)]
async fn update(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<UpdateIngredientRequest>,
) -> ApiResult<Tagged<IngredientDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let updated = state
        .catalogue
        .update_ingredient(IngredientId::from(id), revision, body.into())
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/ingredients/{id}/archive",
    operation_id = "archiveIngredient",
    params(
        ("id" = Uuid, Path, description = "Ingredient id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Archived", body = IngredientDto)),
    tag = "ingredients",
    security(("basic" = []))
)]
async fn archive(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<IngredientDto>> {
    set_archived(state, principal, id, revision, true).await
}

#[utoipa::path(
    post,
    path = "/api/v1/ingredients/{id}/unarchive",
    operation_id = "unarchiveIngredient",
    params(
        ("id" = Uuid, Path, description = "Ingredient id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Restored", body = IngredientDto)),
    tag = "ingredients",
    security(("basic" = []))
)]
async fn unarchive(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<IngredientDto>> {
    set_archived(state, principal, id, revision, false).await
}

async fn set_archived(
    state: AppState,
    principal: Principal,
    id: Uuid,
    revision: mmp_core::domain::Revision,
    archived: bool,
) -> ApiResult<Tagged<IngredientDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let updated = state
        .catalogue
        .set_ingredient_archived(IngredientId::from(id), revision, archived)
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/ingredients/{id}/products",
    operation_id = "listIngredientProducts",
    params(("id" = Uuid, Path, description = "Ingredient id")),
    responses((status = 200, description = "Products that fulfil this ingredient", body = ProductPage)),
    tag = "ingredients",
    security(("basic" = []))
)]
async fn products_for_ingredient(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    Query(query): Query<crate::dto::ProductListQuery>,
) -> ApiResult<Json<ProductPage>> {
    principal.require(Permission::CatalogueRead)?;
    let ingredient_id = IngredientId::from(id);
    state.catalogue.get_ingredient(ingredient_id).await?;

    let mut product_query = crate::routes::products::to_query(query);
    product_query.mapped_ingredient_id = Some(ingredient_id);
    let page = state.catalogue.list_products(&product_query).await?;
    Ok(Json(page.into()))
}

#[allow(unused)]
fn _assert_error_type(_: ApiError) {}
