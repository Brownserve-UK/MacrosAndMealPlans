use axum::Json;
use axum::extract::{Path, Query, State};
use mmp_core::domain::{ConsumedAmount, IngredientId, ProductId, Quantity, Revision};
use mmp_core::ports::{PageRequest, ProductQuery};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::{Permission, Principal};
use crate::dto::{
    AmountKindDto, CreateProductRequest, ProductDto, ProductListQuery, ProductNutritionDto,
    ProductNutritionQuery, ProductPage, SetMappingRequest, UpdateProductRequest,
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
        .routes(routes!(set_mapping, clear_mapping))
        .routes(routes!(get_nutrition))
}

pub(crate) fn to_query(query: ProductListQuery) -> ProductQuery {
    ProductQuery {
        search: query.q.filter(|q| !q.trim().is_empty()),
        origin: query.origin,
        barcode: query.barcode.filter(|b| !b.trim().is_empty()),
        retailer: query.retailer.filter(|r| !r.trim().is_empty()),
        mapped_ingredient_id: query.mapped_ingredient_id.map(IngredientId::from),
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
    path = "/api/v1/products",
    params(ProductListQuery),
    operation_id = "listProducts",
    responses((status = 200, description = "A page of products", body = ProductPage)),
    tag = "products",
    security(("basic" = []))
)]
async fn list(
    State(state): State<AppState>,
    principal: Principal,
    Query(query): Query<ProductListQuery>,
) -> ApiResult<Json<ProductPage>> {
    principal.require(Permission::CatalogueRead)?;
    let page = state.catalogue.list_products(&to_query(query)).await?;
    Ok(Json(page.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/products",
    request_body = CreateProductRequest,
    operation_id = "createProduct",
    responses(
        (status = 201, description = "Created", body = ProductDto),
        (status = 409, description = "The barcode is already used", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "products",
    security(("basic" = []))
)]
async fn create(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreateProductRequest>,
) -> ApiResult<Created<ProductDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let created = state.catalogue.create_product(body.into()).await?;
    Ok(Created(created.revision, created.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/products/{id}",
    operation_id = "getProduct",
    params(("id" = Uuid, Path, description = "Product id")),
    responses(
        (status = 200, description = "The product", body = ProductDto,
         headers(("ETag" = String, description = "The revision to send back as If-Match"))),
        (status = 404, description = "Not found", body = crate::error::Problem),
    ),
    tag = "products",
    security(("basic" = []))
)]
async fn get_one(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Tagged<ProductDto>> {
    principal.require(Permission::CatalogueRead)?;
    let product = state.catalogue.get_product(ProductId::from(id)).await?;
    Ok(Tagged(product.revision, product.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/products/{id}",
    operation_id = "updateProduct",
    params(
        ("id" = Uuid, Path, description = "Product id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = UpdateProductRequest,
    responses(
        (status = 200, description = "Updated", body = ProductDto),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 428, description = "If-Match is required", body = crate::error::Problem),
    ),
    tag = "products",
    security(("basic" = []))
)]
async fn update(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<UpdateProductRequest>,
) -> ApiResult<Tagged<ProductDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let updated = state
        .catalogue
        .update_product(ProductId::from(id), revision, body.into())
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/products/{id}/archive",
    operation_id = "archiveProduct",
    params(
        ("id" = Uuid, Path, description = "Product id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Archived", body = ProductDto)),
    tag = "products",
    security(("basic" = []))
)]
async fn archive(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<ProductDto>> {
    set_archived(state, principal, id, revision, true).await
}

#[utoipa::path(
    post,
    path = "/api/v1/products/{id}/unarchive",
    operation_id = "unarchiveProduct",
    params(
        ("id" = Uuid, Path, description = "Product id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Restored", body = ProductDto)),
    tag = "products",
    security(("basic" = []))
)]
async fn unarchive(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<ProductDto>> {
    set_archived(state, principal, id, revision, false).await
}

async fn set_archived(
    state: AppState,
    principal: Principal,
    id: Uuid,
    revision: Revision,
    archived: bool,
) -> ApiResult<Tagged<ProductDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let updated = state
        .catalogue
        .set_product_archived(ProductId::from(id), revision, archived)
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    put,
    path = "/api/v1/products/{id}/ingredient",
    operation_id = "setProductIngredient",
    params(
        ("id" = Uuid, Path, description = "Product id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = SetMappingRequest,
    responses(
        (status = 200, description = "Mapping set", body = ProductDto),
        (status = 404, description = "The ingredient does not exist", body = crate::error::Problem),
        (status = 422, description = "The ingredient is archived", body = crate::error::Problem),
    ),
    tag = "products",
    security(("basic" = []))
)]
async fn set_mapping(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<SetMappingRequest>,
) -> ApiResult<Tagged<ProductDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let updated = state
        .catalogue
        .set_product_mapping(
            ProductId::from(id),
            revision,
            Some(IngredientId::from(body.ingredient_id)),
        )
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/products/{id}/ingredient",
    operation_id = "clearProductIngredient",
    params(
        ("id" = Uuid, Path, description = "Product id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Mapping cleared", body = ProductDto)),
    tag = "products",
    security(("basic" = []))
)]
async fn clear_mapping(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<ProductDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let updated = state
        .catalogue
        .set_product_mapping(ProductId::from(id), revision, None)
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

fn to_amount(query: ProductNutritionQuery) -> ApiResult<ConsumedAmount> {
    let value = query
        .value
        .ok_or_else(|| ApiError::bad_request("`value` is required."))?;
    match query
        .kind
        .ok_or_else(|| ApiError::bad_request("`kind` is required."))?
    {
        AmountKindDto::Measure => {
            let unit = query.unit.ok_or_else(|| {
                ApiError::bad_request("`unit` is required for a measured amount.")
            })?;
            Ok(ConsumedAmount::Measure(Quantity::new(value, unit)))
        }
        AmountKindDto::Servings => Ok(ConsumedAmount::Servings(value)),
        AmountKindDto::Packs => Ok(ConsumedAmount::Packs(value)),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/products/{id}/nutrition",
    operation_id = "getProductNutrition",
    params(
        ("id" = Uuid, Path, description = "Product id"),
        ProductNutritionQuery,
    ),
    responses(
        (status = 200, description = "Nutrition scaled to the given amount", body = ProductNutritionDto),
        (status = 400, description = "The amount could not be understood", body = crate::error::Problem),
        (status = 404, description = "Not found", body = crate::error::Problem),
    ),
    tag = "products",
    security(("basic" = []))
)]
async fn get_nutrition(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    Query(query): Query<ProductNutritionQuery>,
) -> ApiResult<Json<ProductNutritionDto>> {
    principal.require(Permission::CatalogueRead)?;
    let amount = to_amount(query)?;
    let scaled = state
        .catalogue
        .nutrition_for(ProductId::from(id), &amount)
        .await?;
    Ok(Json(ProductNutritionDto {
        nutrition: scaled.facts.into(),
        quality: scaled.quality,
    }))
}
