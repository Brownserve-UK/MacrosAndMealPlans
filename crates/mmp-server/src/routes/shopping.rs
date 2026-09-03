use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use mmp_core::ports::{PageRequest, PurchaseQuery};
use time::{Date, Duration, OffsetDateTime};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::{Permission, Principal};
use crate::dto::{
    CreateOpportunityRequest, CreatePurchaseRequest, FinishShopResponse, MoveOpportunityRequest,
    OpportunityRangeQuery, PageMeta, PurchaseDto, PurchaseListQuery, PurchasePage,
    SetShoppingCadenceRequest, ShoppingCadenceDto, ShoppingListDto, ShoppingListQuery,
    ShoppingOpportunityDto, purchase_id,
};
use crate::error::ApiResult;
use crate::http::Created;
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(requirements))
        .routes(routes!(opportunities, create_opportunity))
        .routes(routes!(move_opportunity, skip_opportunity))
        .routes(routes!(finish_shop))
        .routes(routes!(get_cadence, set_cadence, clear_cadence))
        .routes(routes!(list_purchases, create_purchase))
        .routes(routes!(update_purchase))
}

#[utoipa::path(
    get,
    path = "/api/v1/shopping/requirements",
    params(ShoppingListQuery),
    operation_id = "getShoppingRequirements",
    responses((status = 200, description = "What needs buying, and for which shop",
        body = ShoppingListDto)),
    tag = "shopping",
    security(("basic" = []))
)]
async fn requirements(
    State(state): State<AppState>,
    principal: Principal,
    Query(query): Query<ShoppingListQuery>,
) -> ApiResult<Json<ShoppingListDto>> {
    principal.require(Permission::ShoppingRead)?;
    let list = state.shopping.requirements(query.opportunity_date).await?;
    Ok(Json(list.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/shopping/opportunities",
    params(OpportunityRangeQuery),
    operation_id = "listShoppingOpportunities",
    responses((status = 200, description = "Expected shops in the range",
        body = Vec<ShoppingOpportunityDto>)),
    tag = "shopping",
    security(("basic" = []))
)]
async fn opportunities(
    State(state): State<AppState>,
    principal: Principal,
    Query(query): Query<OpportunityRangeQuery>,
) -> ApiResult<Json<Vec<ShoppingOpportunityDto>>> {
    principal.require(Permission::ShoppingRead)?;
    let today = OffsetDateTime::now_utc().date();
    let from = query.from.unwrap_or(today);
    let to = query.to.unwrap_or(from + Duration::days(56));
    let opportunities = state.shopping.opportunities(from, to).await?;
    Ok(Json(opportunities.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/api/v1/shopping/opportunities",
    request_body = CreateOpportunityRequest,
    operation_id = "createShoppingOpportunity",
    responses(
        (status = 204, description = "The one-off shop was added"),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "shopping",
    security(("basic" = []))
)]
async fn create_opportunity(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreateOpportunityRequest>,
) -> ApiResult<StatusCode> {
    principal.require(Permission::ShoppingWrite)?;
    state.shopping.add_one_off(body.date, body.note).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/api/v1/shopping/opportunities/{date}",
    params(("date" = String, Path, description = "The expected shop being moved, as YYYY-MM-DD")),
    request_body = MoveOpportunityRequest,
    operation_id = "moveShoppingOpportunity",
    responses(
        (status = 204, description = "The shop was moved"),
        (status = 400, description = "The date could not be read", body = crate::error::Problem),
    ),
    tag = "shopping",
    security(("basic" = []))
)]
async fn move_opportunity(
    State(state): State<AppState>,
    principal: Principal,
    Path(date): Path<String>,
    Json(body): Json<MoveOpportunityRequest>,
) -> ApiResult<StatusCode> {
    principal.require(Permission::ShoppingWrite)?;
    state
        .shopping
        .move_opportunity(parse_date(&date)?, body.to)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/api/v1/shopping/opportunities/{date}",
    params(("date" = String, Path, description = "The expected shop being skipped, as YYYY-MM-DD")),
    operation_id = "skipShoppingOpportunity",
    responses(
        (status = 204, description = "The shop was skipped"),
        (status = 400, description = "The date could not be read", body = crate::error::Problem),
    ),
    tag = "shopping",
    security(("basic" = []))
)]
async fn skip_opportunity(
    State(state): State<AppState>,
    principal: Principal,
    Path(date): Path<String>,
) -> ApiResult<StatusCode> {
    principal.require(Permission::ShoppingWrite)?;
    state.shopping.skip_opportunity(parse_date(&date)?).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/v1/shopping/cadence",
    operation_id = "getShoppingCadence",
    responses(
        (status = 200, description = "When the household normally shops",
            body = ShoppingCadenceDto),
        (status = 404, description = "No cadence is configured", body = crate::error::Problem),
    ),
    tag = "shopping",
    security(("basic" = []))
)]
async fn get_cadence(
    State(state): State<AppState>,
    principal: Principal,
) -> ApiResult<Json<ShoppingCadenceDto>> {
    principal.require(Permission::ShoppingRead)?;
    match state.shopping.cadence().await? {
        Some(cadence) => Ok(Json(cadence.into())),
        None => Err(mmp_core::CoreError::not_found("shopping cadence", "singleton").into()),
    }
}

#[utoipa::path(
    put,
    path = "/api/v1/shopping/cadence",
    request_body = SetShoppingCadenceRequest,
    operation_id = "setShoppingCadence",
    responses(
        (status = 200, description = "Saved", body = ShoppingCadenceDto),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "shopping",
    security(("basic" = []))
)]
async fn set_cadence(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<SetShoppingCadenceRequest>,
) -> ApiResult<Json<ShoppingCadenceDto>> {
    principal.require(Permission::HouseholdWrite)?;
    let cadence = state.shopping.set_cadence(body.into_domain()?).await?;
    Ok(Json(cadence.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/shopping/cadence",
    operation_id = "clearShoppingCadence",
    responses(
        (status = 204, description = "Cleared"),
        (status = 404, description = "No cadence is configured", body = crate::error::Problem),
    ),
    tag = "shopping",
    security(("basic" = []))
)]
async fn clear_cadence(
    State(state): State<AppState>,
    principal: Principal,
) -> ApiResult<StatusCode> {
    principal.require(Permission::HouseholdWrite)?;
    state.shopping.clear_cadence().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/v1/purchases",
    params(PurchaseListQuery),
    operation_id = "listPurchases",
    responses((status = 200, description = "A page of purchases", body = PurchasePage)),
    tag = "shopping",
    security(("basic" = []))
)]
async fn list_purchases(
    State(state): State<AppState>,
    principal: Principal,
    Query(query): Query<PurchaseListQuery>,
) -> ApiResult<Json<PurchasePage>> {
    principal.require(Permission::ShoppingRead)?;
    let page = state
        .shopping
        .purchases(&PurchaseQuery {
            state: query.state.map(Into::into),
            opportunity_date: query.opportunity_date,
            page: PageRequest::new(
                query.page.unwrap_or(1),
                query.per_page.unwrap_or(PageRequest::DEFAULT_PER_PAGE),
            ),
            sort: Default::default(),
        })
        .await?;
    Ok(Json(PurchasePage {
        page: PageMeta::of(&page),
        items: page.items.into_iter().map(Into::into).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/purchases",
    request_body = CreatePurchaseRequest,
    operation_id = "createPurchase",
    responses(
        (status = 201, description = "Recorded", body = PurchaseDto),
        (status = 404, description = "The product does not exist", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "shopping",
    security(("basic" = []))
)]
async fn create_purchase(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreatePurchaseRequest>,
) -> ApiResult<Created<PurchaseDto>> {
    principal.require(Permission::ShoppingWrite)?;
    let purchase = state
        .shopping
        .record_purchase(body.into(), principal.user_id)
        .await?;
    Ok(Created(purchase.revision, purchase.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/purchases/{id}",
    params(
        ("id" = Uuid, Path, description = "The purchase"),
        ("If-Match" = String, Header, description = "The revision being updated"),
    ),
    request_body = crate::dto::UpdatePurchaseRequest,
    operation_id = "updatePurchase",
    responses(
        (status = 200, description = "Updated", body = PurchaseDto),
        (status = 404, description = "No such purchase", body = crate::error::Problem),
        (status = 409, description = "Already in stock", body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "shopping",
    security(("basic" = []))
)]
async fn update_purchase(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    crate::http::IfMatch(revision): crate::http::IfMatch,
    Json(body): Json<crate::dto::UpdatePurchaseRequest>,
) -> ApiResult<crate::http::Tagged<PurchaseDto>> {
    principal.require(Permission::ShoppingWrite)?;
    let purchase = state
        .shopping
        .update_purchase(purchase_id(id), revision, body.into())
        .await?;
    Ok(crate::http::Tagged(purchase.revision, purchase.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/shopping/opportunities/{date}/finish",
    params(("date" = String, Path, description = "The shop being finished, as YYYY-MM-DD")),
    operation_id = "finishShop",
    responses(
        (status = 200, description = "The shop was finished", body = FinishShopResponse),
        (status = 400, description = "The date could not be read", body = crate::error::Problem),
    ),
    tag = "shopping",
    security(("basic" = []))
)]
async fn finish_shop(
    State(state): State<AppState>,
    principal: Principal,
    Path(date): Path<String>,
) -> ApiResult<Json<FinishShopResponse>> {
    principal.require(Permission::ShoppingWrite)?;
    let finished = state
        .shopping
        .finish_shop(parse_date(&date)?, principal.user_id)
        .await?;
    Ok(Json(finished.into()))
}

fn parse_date(raw: &str) -> Result<Date, crate::error::ApiError> {
    crate::dto::common::iso_date::parse(raw).map_err(|_| {
        let mut errors = mmp_core::error::ValidationErrors::new();
        errors.push("date", "Use a date like 2026-09-05.");
        mmp_core::CoreError::Validation(errors).into()
    })
}
