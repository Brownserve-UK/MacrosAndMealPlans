use axum::Json;
use axum::extract::{Path, Query, State};
use mmp_core::domain::{HouseholdMemberId, ProductId};
use mmp_core::ports::{PageRequest, StockQuery};
use time::{Duration, OffsetDateTime};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::{Permission, Principal};
use crate::dto::{
    AvailabilityReportDto, CreateStockItemRequest, PageMeta, StockAvailabilityQuery, StockEventDto,
    StockItemDto, StockListQuery, StockPage, UpdateStockItemRequest, stock_item_id,
};
use crate::error::ApiResult;
use crate::http::{Created, IfMatch, Tagged};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(availability))
        .routes(routes!(get_one, update))
        .routes(routes!(archive))
        .routes(routes!(events))
}

fn to_query(query: StockListQuery) -> StockQuery {
    StockQuery {
        product_id: query.product_id.map(ProductId::from),
        include_archived: query.include_archived.unwrap_or(false),
        page: PageRequest::new(
            query.page.unwrap_or(1),
            query.per_page.unwrap_or(PageRequest::DEFAULT_PER_PAGE),
        ),
        sort: query.sort.map(Into::into).unwrap_or_default(),
    }
}

fn subject_for(principal: &Principal, requested: Option<Uuid>) -> Option<HouseholdMemberId> {
    requested
        .map(HouseholdMemberId::from)
        .or(principal.member_id)
}

#[utoipa::path(
    get,
    path = "/api/v1/stock",
    params(StockListQuery),
    operation_id = "listStock",
    responses((status = 200, description = "A page of stock items", body = StockPage)),
    tag = "stock",
    security(("basic" = []))
)]
async fn list(
    State(state): State<AppState>,
    principal: Principal,
    Query(query): Query<StockListQuery>,
) -> ApiResult<Json<StockPage>> {
    principal.require(Permission::StockRead)?;
    let page = state.stock.list(&to_query(query)).await?;
    Ok(Json(StockPage {
        page: PageMeta::of(&page),
        items: page.items.into_iter().map(Into::into).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/stock",
    request_body = CreateStockItemRequest,
    operation_id = "createStockItem",
    responses(
        (status = 201, description = "Created", body = StockItemDto),
        (status = 404, description = "The product does not exist", body = crate::error::Problem),
        (status = 409, description = "The product is archived", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "stock",
    security(("basic" = []))
)]
async fn create(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreateStockItemRequest>,
) -> ApiResult<Created<StockItemDto>> {
    principal.require(Permission::StockWrite)?;
    let subject = subject_for(&principal, body.subject_member_id);
    let created = state
        .stock
        .create(body.into_domain(), principal.user_id, subject)
        .await?;
    Ok(Created(created.revision, created.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/stock/{id}",
    operation_id = "getStockItem",
    params(("id" = Uuid, Path, description = "Stock item id")),
    responses(
        (status = 200, description = "The stock item", body = StockItemDto,
         headers(("ETag" = String, description = "The revision to send back as If-Match"))),
        (status = 404, description = "Not found", body = crate::error::Problem),
    ),
    tag = "stock",
    security(("basic" = []))
)]
async fn get_one(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Tagged<StockItemDto>> {
    principal.require(Permission::StockRead)?;
    let item = state.stock.get(stock_item_id(id)).await?;
    Ok(Tagged(item.revision, item.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/stock/{id}",
    operation_id = "updateStockItem",
    params(
        ("id" = Uuid, Path, description = "Stock item id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = UpdateStockItemRequest,
    responses(
        (status = 200, description = "Updated", body = StockItemDto),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
        (status = 428, description = "If-Match is required", body = crate::error::Problem),
    ),
    tag = "stock",
    security(("basic" = []))
)]
async fn update(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<UpdateStockItemRequest>,
) -> ApiResult<Tagged<StockItemDto>> {
    principal.require(Permission::StockWrite)?;
    let subject = subject_for(&principal, body.subject_member_id);
    let updated = state
        .stock
        .update(
            stock_item_id(id),
            revision,
            body.into(),
            principal.user_id,
            subject,
        )
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/stock/{id}/archive",
    operation_id = "archiveStockItem",
    params(
        ("id" = Uuid, Path, description = "Stock item id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Archived", body = StockItemDto)),
    tag = "stock",
    security(("basic" = []))
)]
async fn archive(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<StockItemDto>> {
    principal.require(Permission::StockWrite)?;
    let updated = state
        .stock
        .set_archived(stock_item_id(id), revision, principal.user_id)
        .await?;
    Ok(Tagged(updated.revision, updated.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/stock/{id}/events",
    operation_id = "listStockEvents",
    params(("id" = Uuid, Path, description = "Stock item id")),
    responses(
        (status = 200, description = "The stock item's audit history, newest first",
         body = Vec<StockEventDto>),
        (status = 403, description = "Not permitted", body = crate::error::Problem),
    ),
    tag = "stock",
    security(("basic" = []))
)]
async fn events(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<StockEventDto>>> {
    principal.require(Permission::StockHistory)?;
    let events = state.stock.events(stock_item_id(id)).await?;
    Ok(Json(events.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    get,
    path = "/api/v1/stock/availability",
    params(StockAvailabilityQuery),
    operation_id = "getStockAvailability",
    responses((status = 200,
        description = "Availability per product and per ingredient, netting off planned demand",
        body = AvailabilityReportDto)),
    tag = "stock",
    security(("basic" = []))
)]
async fn availability(
    State(state): State<AppState>,
    principal: Principal,
    Query(query): Query<StockAvailabilityQuery>,
) -> ApiResult<Json<AvailabilityReportDto>> {
    principal.require(Permission::StockRead)?;
    let today = OffsetDateTime::now_utc().date();
    let from = query.from.unwrap_or(today);
    let to = query.to.unwrap_or(today + Duration::days(14));

    let report = match query.product_id {
        Some(id) => {
            state
                .stock
                .availability(&[ProductId::from(id)], from, to)
                .await?
        }
        None => state.stock.availability_overview(from, to).await?,
    };
    Ok(Json(report.into()))
}
