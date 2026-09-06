use axum::Json;
use axum::extract::{Path, State};
use mmp_core::domain::{PreparedBatchId, RecipeId};
use mmp_core::services::RecordPreparation;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::Principal;
use crate::dto::{PreparationResponse, PreparedBatchDto, RecordPreparationRequest};
use crate::error::ApiResult;
use crate::http::Created;
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(record))
        .routes(routes!(get))
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
            storage_location: body.storage_location.into(),
            usability_deadline: body.usability_deadline.map(Into::into),
            note: body.note,
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
        (status = 200, description = "The cooking event", body = PreparedBatchDto),
        (status = 404, description = "No such preparation", body = crate::error::Problem),
    ),
    tag = "preparation",
    security(("basic" = []))
)]
async fn get(
    State(state): State<AppState>,
    _principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<PreparedBatchDto>> {
    let batch = state.preparation.get(PreparedBatchId::from(id)).await?;
    Ok(Json(batch.into()))
}
