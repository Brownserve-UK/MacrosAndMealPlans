use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

use crate::openapi::ApiDoc;
use crate::routes;
use crate::state::AppState;

fn routes() -> (Router<AppState>, utoipa::openapi::OpenApi) {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(routes::meta::router())
        .merge(routes::auth::router())
        .merge(routes::ingredients::router())
        .merge(routes::prepared_meals::router())
        .merge(routes::products::router())
        .merge(routes::members::router())
        .merge(routes::users::router())
        .merge(routes::consumption::router())
        .merge(routes::meal_plan::router())
        .merge(routes::meal_templates::router())
        .merge(routes::nutrition_target::router())
        .merge(routes::nutrition_plan::router())
        .merge(routes::recipes::router())
        .merge(routes::review::router())
        .merge(routes::settings::router())
        .merge(routes::shopping::router())
        .merge(routes::stock::router())
        .merge(routes::weight::router())
        .merge(routes::preparation::router())
        .split_for_parts();

    let router = router
        .merge(Scalar::with_url("/docs", api.clone()))
        .route(
            "/openapi.json",
            axum::routing::get({
                let api = api.clone();
                move || async move { axum::Json(api) }
            }),
        )
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());

    (router, api)
}

pub fn build(state: AppState) -> (Router, utoipa::openapi::OpenApi) {
    let (router, api) = routes();
    (router.with_state(state), api)
}

pub fn api_document() -> utoipa::openapi::OpenApi {
    routes().1
}

pub fn with_web_client(router: Router, web_dist: Option<&str>) -> Router {
    let Some(dist) = web_dist else {
        return router;
    };

    let index = std::path::Path::new(dist).join("index.html");
    router.fallback_service(
        tower_http::services::ServeDir::new(dist)
            .fallback(tower_http::services::ServeFile::new(index)),
    )
}
