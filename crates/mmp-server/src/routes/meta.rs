use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;

pub const PROTOCOL_VERSION: u32 = 1;
pub const MIN_SUPPORTED_CLIENT_PROTOCOL: u32 = 1;
pub const MAX_SUPPORTED_CLIENT_PROTOCOL: u32 = 1;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(meta))
        .routes(routes!(health))
        .routes(routes!(units))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MetaDto {
    #[schema(example = "0.1.0")]
    pub server_version: &'static str,
    #[schema(example = 1)]
    pub protocol_version: u32,
    pub supported_client_protocol_versions: ProtocolRange,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProtocolRange {
    #[schema(example = 1)]
    pub min: u32,
    #[schema(example = 1)]
    pub max: u32,
}

#[utoipa::path(
    get,
    path = "/api/v1/meta",
    operation_id = "getMeta",
    responses((status = 200, description = "Server and protocol compatibility", body = MetaDto)),
    tag = "meta"
)]
async fn meta() -> Json<MetaDto> {
    Json(MetaDto {
        server_version: env!("CARGO_PKG_VERSION"),
        protocol_version: PROTOCOL_VERSION,
        supported_client_protocol_versions: ProtocolRange {
            min: MIN_SUPPORTED_CLIENT_PROTOCOL,
            max: MAX_SUPPORTED_CLIENT_PROTOCOL,
        },
    })
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthDto {
    pub status: &'static str,
}

#[utoipa::path(
    get,
    path = "/api/v1/health",
    operation_id = "getHealth",
    responses((status = 200, description = "The server is up", body = HealthDto)),
    tag = "meta"
)]
async fn health() -> Json<HealthDto> {
    Json(HealthDto { status: "ok" })
}

#[utoipa::path(
    get,
    path = "/api/v1/units",
    operation_id = "listUnits",
    responses((status = 200, description = "Supported quantity units", body = Vec<crate::dto::UnitDto>)),
    tag = "meta"
)]
async fn units() -> Json<Vec<crate::dto::UnitDto>> {
    Json(crate::dto::UnitDto::all())
}
