use axum::Json;
use axum::extract::{Path, Query, State};
use mmp_core::domain::{PreparedBatchId, RecipeId};
use mmp_core::services::{MoveCookedFood, RecordPreparation};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::{Permission, Principal};
use crate::dto::{
    MoveCookedFoodRequest, PlacePortionsRequest, PreparationRangeQuery, PreparationResponse,
    PreparedBatchDto, RecordPreparationRequest, StockItemDto,
};
use crate::error::ApiResult;
use crate::http::{Created, IfMatch, Tagged};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(record, list))
        .routes(routes!(get))
        .routes(routes!(place))
        .routes(routes!(move_cooked))
}

#[utoipa::path(
    post,
    path = "/api/v1/cooked-food/{recipe_id}/move",
    operation_id = "moveCookedFood",
    params(("recipe_id" = Uuid, Path, description = "The recipe the cooked food came from")),
    request_body = MoveCookedFoodRequest,
    responses(
        (status = 200, description = "Where the moved servings now live", body = Vec<StockItemDto>),
        (status = 409, description = "There is not that much there, or it is already there",
         body = crate::error::Problem),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "preparation",
    security(("basic" = []))
)]
async fn move_cooked(
    State(state): State<AppState>,
    principal: Principal,
    Path(recipe_id): Path<Uuid>,
    Json(body): Json<MoveCookedFoodRequest>,
) -> ApiResult<Json<Vec<StockItemDto>>> {
    principal.require(Permission::StockWrite)?;
    let landed = state
        .preparation
        .move_cooked(MoveCookedFood {
            recipe_id: RecipeId::from(recipe_id),
            from: body.from.into(),
            to: body.to.into(),
            servings: body.servings,
            actor: principal.user_id,
        })
        .await?;
    Ok(Json(landed.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    put,
    path = "/api/v1/preparations/{id}/placements",
    operation_id = "placePortions",
    params(
        ("id" = Uuid, Path, description = "Prepared batch id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = PlacePortionsRequest,
    responses(
        (status = 200, description = "Where the cook's remaining portions now live",
         body = PreparationResponse),
        (status = 404, description = "No such preparation", body = crate::error::Problem),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 422, description = "That does not add up to what is left",
         body = crate::error::Problem),
        (status = 428, description = "If-Match is required", body = crate::error::Problem),
    ),
    tag = "preparation",
    security(("basic" = []))
)]
async fn place(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(expected): IfMatch,
    Json(body): Json<PlacePortionsRequest>,
) -> ApiResult<Tagged<PreparationResponse>> {
    let placed = state
        .preparation
        .place(
            PreparedBatchId::from(id),
            expected,
            body.placements.into_iter().map(Into::into).collect(),
            principal.user_id,
        )
        .await?;
    let stock_outcomes = placed.stock.iter().cloned().map(Into::into).collect();
    let batch = placed.into_value();
    let revision = batch.revision;
    Ok(Tagged(
        revision,
        PreparationResponse {
            batch: batch.into(),
            stock_outcomes,
        },
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/preparations",
    operation_id = "listPreparations",
    params(PreparationRangeQuery),
    responses(
        (status = 200, description = "Cooking events in the range, newest first",
         body = Vec<PreparedBatchDto>),
        (status = 409, description = "That date range runs backwards", body = crate::error::Problem),
    ),
    tag = "preparation",
    security(("basic" = []))
)]
async fn list(
    State(state): State<AppState>,
    _principal: Principal,
    Query(range): Query<PreparationRangeQuery>,
) -> ApiResult<Json<Vec<PreparedBatchDto>>> {
    let batches = state
        .preparation
        .list_in_range(range.from, range.to)
        .await?;
    Ok(Json(batches.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/api/v1/preparations",
    operation_id = "recordPreparation",
    request_body = RecordPreparationRequest,
    responses(
        (status = 201, description = "Cooked, with any stock shortfalls it ran into",
         body = PreparationResponse),
        (status = 404, description = "No such recipe", body = crate::error::Problem),
        (status = 409, description = "That recipe is archived", body = crate::error::Problem),
    ),
    tag = "preparation",
    security(("basic" = []))
)]
async fn record(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<RecordPreparationRequest>,
) -> ApiResult<Created<PreparationResponse>> {
    let source = body.source();
    let prepared = state
        .preparation
        .record(RecordPreparation {
            recipe_id: RecipeId::from(body.recipe_id),
            source,
            servings_produced: body.servings_produced,
            placements: body.placements.into_iter().map(Into::into).collect(),
            prepared_at: None,
            actor: principal.user_id,
        })
        .await?;
    let stock_outcomes = prepared.stock.iter().cloned().map(Into::into).collect();
    let batch = prepared.into_value();
    let revision = batch.revision;
    Ok(Created(
        revision,
        PreparationResponse {
            batch: batch.into(),
            stock_outcomes,
        },
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/preparations/{id}",
    operation_id = "getPreparation",
    params(("id" = Uuid, Path, description = "Prepared batch id")),
    responses(
        (status = 200, description = "The cooking event", body = PreparedBatchDto,
         headers(("ETag" = String, description = "The revision to send back as If-Match"))),
        (status = 404, description = "No such preparation", body = crate::error::Problem),
    ),
    tag = "preparation",
    security(("basic" = []))
)]
async fn get(
    State(state): State<AppState>,
    _principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Tagged<PreparedBatchDto>> {
    let batch = state.preparation.get(PreparedBatchId::from(id)).await?;
    let revision = batch.revision;
    Ok(Tagged(revision, batch.into()))
}
