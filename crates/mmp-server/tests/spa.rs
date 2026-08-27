use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use mmp_core::ports::SystemClock;
use mmp_core::services::{
    CatalogueService, DiaryService, HouseholdService, HouseholdSettingsService, MealPlanService,
    NutritionTargetService,
};
use mmp_core::testing::{
    InMemoryAccessGrantRepository, InMemoryConsumptionRecordRepository,
    InMemoryHouseholdMemberRepository, InMemoryHouseholdSettingsRepository,
    InMemoryIngredientRepository, InMemoryMealPlanRepository, InMemoryNutritionTargetRepository,
    InMemoryProductRepository, InMemoryUserRepository,
};
use mmp_server::auth::DevBasicAuthProvider;
use mmp_server::{AppState, app};
use tower::ServiceExt;

fn web_dist() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("mmp-spa-test-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::write(
        dir.join("index.html"),
        "<!doctype html><title>app shell</title>",
    )
    .unwrap();
    std::fs::write(dir.join("assets/app.js"), "console.log('bundle');").unwrap();
    dir
}

fn app_with_web(dist: &std::path::Path) -> axum::Router {
    let household = Arc::new(HouseholdService::new(
        Arc::new(InMemoryHouseholdMemberRepository::new()),
        Arc::new(InMemoryUserRepository::new()),
        Arc::new(InMemoryAccessGrantRepository::new()),
        Arc::new(SystemClock),
    ));
    let products = InMemoryProductRepository::new();
    let consumption = InMemoryConsumptionRecordRepository::new();
    let targets = InMemoryNutritionTargetRepository::new();
    let state = AppState::new(
        CatalogueService::new(
            Arc::new(InMemoryIngredientRepository::new()),
            Arc::new(products.clone()),
            Arc::new(SystemClock),
        ),
        household.clone(),
        HouseholdSettingsService::new(
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            Arc::new(SystemClock),
        ),
        DiaryService::new(
            Arc::new(consumption.clone()),
            Arc::new(products.clone()),
            Arc::new(SystemClock),
        ),
        MealPlanService::new(
            Arc::new(InMemoryMealPlanRepository::new(consumption.clone())),
            Arc::new(products),
            Arc::new(consumption),
            Arc::new(targets.clone()),
            Arc::new(SystemClock),
        ),
        NutritionTargetService::new(Arc::new(targets), Arc::new(SystemClock)),
        Arc::new(DevBasicAuthProvider::new(household, "changeme")),
    );
    let (router, _) = app::build(state);
    app::with_web_client(router, dist.to_str())
}

async fn get(app: &axum::Router, path: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn serves_the_built_index() {
    let dist = web_dist();
    let (status, body) = get(&app_with_web(&dist), "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("app shell"));
    std::fs::remove_dir_all(&dist).ok();
}

#[tokio::test]
async fn serves_built_assets() {
    let dist = web_dist();
    let (status, body) = get(&app_with_web(&dist), "/assets/app.js").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("bundle"));
    std::fs::remove_dir_all(&dist).ok();
}

#[tokio::test]
async fn an_unknown_path_falls_back_to_the_index_for_client_routing() {
    let dist = web_dist();
    let (status, body) = get(&app_with_web(&dist), "/ingredients/some-id").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("app shell"),
        "deep links must reach the client router"
    );
    std::fs::remove_dir_all(&dist).ok();
}

#[tokio::test]
async fn the_api_is_not_swallowed_by_the_spa_fallback() {
    let dist = web_dist();
    let app = app_with_web(&dist);

    let (status, body) = get(&app, "/api/v1/meta").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("protocol_version"),
        "got the SPA instead of the API: {body}"
    );

    let (status, _) = get(&app, "/api/v1/ingredients").await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "a protected route must still authenticate rather than fall through to index.html"
    );

    std::fs::remove_dir_all(&dist).ok();
}

#[tokio::test]
async fn the_openapi_document_is_still_reachable() {
    let dist = web_dist();
    let response = app_with_web(&dist)
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/json"
    );
    std::fs::remove_dir_all(&dist).ok();
}

#[tokio::test]
async fn without_a_web_build_the_api_still_works() {
    let household = Arc::new(HouseholdService::new(
        Arc::new(InMemoryHouseholdMemberRepository::new()),
        Arc::new(InMemoryUserRepository::new()),
        Arc::new(InMemoryAccessGrantRepository::new()),
        Arc::new(SystemClock),
    ));
    let products = InMemoryProductRepository::new();
    let consumption = InMemoryConsumptionRecordRepository::new();
    let targets = InMemoryNutritionTargetRepository::new();
    let state = AppState::new(
        CatalogueService::new(
            Arc::new(InMemoryIngredientRepository::new()),
            Arc::new(products.clone()),
            Arc::new(SystemClock),
        ),
        household.clone(),
        HouseholdSettingsService::new(
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            Arc::new(SystemClock),
        ),
        DiaryService::new(
            Arc::new(consumption.clone()),
            Arc::new(products.clone()),
            Arc::new(SystemClock),
        ),
        MealPlanService::new(
            Arc::new(InMemoryMealPlanRepository::new(consumption.clone())),
            Arc::new(products),
            Arc::new(consumption),
            Arc::new(targets.clone()),
            Arc::new(SystemClock),
        ),
        NutritionTargetService::new(Arc::new(targets), Arc::new(SystemClock)),
        Arc::new(DevBasicAuthProvider::new(household, "changeme")),
    );
    let (router, _) = app::build(state);
    let router = app::with_web_client(router, None);

    let (status, _) = get(&router, "/api/v1/meta").await;
    assert_eq!(status, StatusCode::OK);
}
