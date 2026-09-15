use axum::Json;
use axum::extract::{Path, Query, State};
use mmp_core::domain::PreparedMealId;
use mmp_core::ports::{PageRequest, PreparedMealQuery};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::{Permission, Principal};
use crate::dto::{
    CreatePreparedMealRequest, PreparedMealDto, PreparedMealListQuery, PreparedMealPage,
    ProductPage, UpdatePreparedMealRequest,
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
        .routes(routes!(products_for_prepared_meal))
}

fn to_query(query: PreparedMealListQuery) -> PreparedMealQuery {
    PreparedMealQuery {
        search: query.q.filter(|q| !q.trim().is_empty()),
        origin: query.origin,
        needs_products: query.needs_products,
        include_archived: query.include_archived.unwrap_or(false),
        page: PageRequest::new(
            query.page.unwrap_or(1),
            query.per_page.unwrap_or(PageRequest::DEFAULT_PER_PAGE),
        ),
        sort_by: query.sort_by.map(Into::into).unwrap_or_default(),
        sort: query.sort.map(Into::into).unwrap_or_default(),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/prepared-meals",
    params(PreparedMealListQuery),
    operation_id = "listPreparedMeals",
    responses(
        (status = 200, description = "A page of prepared meals", body = PreparedMealPage),
        (status = 401, description = "Authentication required", body = crate::error::Problem),
    ),
    tag = "prepared-meals",
    security(("basic" = []))
)]
async fn list(
    State(state): State<AppState>,
    principal: Principal,
    Query(query): Query<PreparedMealListQuery>,
) -> ApiResult<Json<PreparedMealPage>> {
    principal.require(Permission::CatalogueRead)?;
    let page = state
        .catalogue
        .list_prepared_meals(&to_query(query))
        .await?;
    Ok(Json(page.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/prepared-meals",
    request_body = CreatePreparedMealRequest,
    operation_id = "createPreparedMeal",
    responses(
        (status = 201, description = "Created", body = PreparedMealDto),
        (status = 409, description = "The name is already taken", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "prepared-meals",
    security(("basic" = []))
)]
async fn create(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreatePreparedMealRequest>,
) -> ApiResult<Created<PreparedMealDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let created = state.catalogue.create_prepared_meal(body.into()).await?;
    Ok(Created(created.revision, created.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/prepared-meals/{id}",
    operation_id = "getPreparedMeal",
    params(("id" = Uuid, Path, description = "Prepared meal id")),
    responses(
        (status = 200, description = "The prepared meal", body = PreparedMealDto,
         headers(("ETag" = String, description = "The revision to send back as If-Match"))),
        (status = 404, description = "Not found", body = crate::error::Problem),
    ),
    tag = "prepared-meals",
    security(("basic" = []))
)]
async fn get_one(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Tagged<PreparedMealDto>> {
    principal.require(Permission::CatalogueRead)?;
    let prepared_meal = state
        .catalogue
        .get_prepared_meal(PreparedMealId::from(id))
        .await?;
    Ok(Tagged(prepared_meal.revision, prepared_meal.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/prepared-meals/{id}",
    operation_id = "updatePreparedMeal",
    params(
        ("id" = Uuid, Path, description = "Prepared meal id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = UpdatePreparedMealRequest,
    responses(
        (status = 200, description = "Updated", body = PreparedMealDto),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 428, description = "If-Match is required", body = crate::error::Problem),
    ),
    tag = "prepared-meals",
    security(("basic" = []))
)]
async fn update(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<UpdatePreparedMealRequest>,
) -> ApiResult<Tagged<PreparedMealDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let updated = state
        .catalogue
        .update_prepared_meal(PreparedMealId::from(id), revision, body.into())
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/prepared-meals/{id}/archive",
    operation_id = "archivePreparedMeal",
    params(
        ("id" = Uuid, Path, description = "Prepared meal id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Archived", body = PreparedMealDto)),
    tag = "prepared-meals",
    security(("basic" = []))
)]
async fn archive(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<PreparedMealDto>> {
    set_archived(state, principal, id, revision, true).await
}

#[utoipa::path(
    post,
    path = "/api/v1/prepared-meals/{id}/unarchive",
    operation_id = "unarchivePreparedMeal",
    params(
        ("id" = Uuid, Path, description = "Prepared meal id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Restored", body = PreparedMealDto)),
    tag = "prepared-meals",
    security(("basic" = []))
)]
async fn unarchive(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<PreparedMealDto>> {
    set_archived(state, principal, id, revision, false).await
}

async fn set_archived(
    state: AppState,
    principal: Principal,
    id: Uuid,
    revision: mmp_core::domain::Revision,
    archived: bool,
) -> ApiResult<Tagged<PreparedMealDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let updated = state
        .catalogue
        .set_prepared_meal_archived(PreparedMealId::from(id), revision, archived)
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/prepared-meals/{id}/products",
    operation_id = "listPreparedMealProducts",
    params(("id" = Uuid, Path, description = "Prepared meal id")),
    responses((status = 200, description = "Products that fulfil this prepared meal", body = ProductPage)),
    tag = "prepared-meals",
    security(("basic" = []))
)]
async fn products_for_prepared_meal(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    Query(query): Query<crate::dto::ProductListQuery>,
) -> ApiResult<Json<ProductPage>> {
    principal.require(Permission::CatalogueRead)?;
    let prepared_meal_id = PreparedMealId::from(id);
    state.catalogue.get_prepared_meal(prepared_meal_id).await?;

    let mut product_query = crate::routes::products::to_query(query);
    product_query.mapped_prepared_meal_id = Some(prepared_meal_id);
    let page = state.catalogue.list_products(&product_query).await?;
    Ok(Json(page.into()))
}
