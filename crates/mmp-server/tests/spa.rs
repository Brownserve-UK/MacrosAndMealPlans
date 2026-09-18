use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use mmp_core::ports::SystemClock;
use mmp_core::services::{
    CatalogueService, ConsumptionService, HouseholdService, HouseholdSettingsService,
    MealPlanService, MealTemplateService, NutritionPlanService, NutritionTargetService,
    PreparationService, RecipeService, ShoppingService, StockService, WeightService,
};
use mmp_core::testing::{
    InMemoryAccessGrantRepository, InMemoryCalorieCalculationRepository,
    InMemoryConsumptionRecordRepository, InMemoryFinishShopRepository,
    InMemoryHouseholdMemberRepository, InMemoryHouseholdSettingsRepository,
    InMemoryIngredientRepository, InMemoryMealPlanRepository, InMemoryMealTemplateRepository,
    InMemoryMemberBodyProfileRepository, InMemoryNutritionTargetRepository,
    InMemoryPreparedBatchRepository, InMemoryPreparedMealRepository, InMemoryProductRepository,
    InMemoryPurchaseRepository, InMemoryRecipeRepository, InMemoryShoppingCadenceRepository,
    InMemoryShoppingListItemRepository, InMemoryShoppingOpportunityRepository,
    InMemoryShoppingSuggestionDismissalRepository, InMemoryShoppingTripRepository,
    InMemoryStockRepository, InMemoryUserRepository, InMemoryWeightGoalRepository,
    InMemoryWeightRecordRepository,
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

fn finish_shop_repository() -> Arc<InMemoryFinishShopRepository> {
    Arc::new(InMemoryFinishShopRepository::new(
        InMemoryPurchaseRepository::new(),
        InMemoryShoppingListItemRepository::new(),
        InMemoryShoppingTripRepository::new(),
    ))
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
    let ingredients = Arc::new(InMemoryIngredientRepository::new());
    let recipes_repo = Arc::new(InMemoryRecipeRepository::new());
    let recipes = RecipeService::new(
        recipes_repo.clone(),
        Arc::new(products.clone()),
        ingredients.clone(),
        Arc::new(InMemoryPreparedMealRepository::new()),
        Arc::new(SystemClock),
    );
    let stock_service = StockService::new(
        Arc::new(InMemoryStockRepository::new()),
        Arc::new(InMemoryProductRepository::new()),
        Arc::new(InMemoryIngredientRepository::new()),
        Arc::new(InMemoryPreparedMealRepository::new()),
        Arc::new(InMemoryMealPlanRepository::default()),
        recipes_repo.clone(),
        Arc::new(InMemoryPreparedBatchRepository::new()),
        Arc::new(InMemoryHouseholdSettingsRepository::new()),
        Arc::new(SystemClock),
    );
    let meal_templates = MealTemplateService::new(
        Arc::new(InMemoryMealTemplateRepository::new()),
        Arc::new(InMemoryMealPlanRepository::default()),
        Arc::new(SystemClock),
    );
    let nutrition_plan = NutritionPlanService::new(
        Arc::new(InMemoryMemberBodyProfileRepository::new()),
        Arc::new(InMemoryCalorieCalculationRepository::new()),
        NutritionTargetService::new(Arc::new(targets.clone()), Arc::new(SystemClock)),
        WeightService::new(
            Arc::new(InMemoryWeightRecordRepository::new()),
            Arc::new(InMemoryWeightGoalRepository::new()),
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            Arc::new(SystemClock),
        ),
        Arc::new(InMemoryHouseholdSettingsRepository::new()),
        Arc::new(SystemClock),
    );
    let state = AppState::new(
        CatalogueService::new(
            ingredients.clone(),
            Arc::new(InMemoryPreparedMealRepository::new()),
            Arc::new(products.clone()),
            Arc::new(SystemClock),
        ),
        household.clone(),
        HouseholdSettingsService::new(
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            Arc::new(SystemClock),
        ),
        ConsumptionService::new(
            Arc::new(consumption.clone()),
            Arc::new(products.clone()),
            ingredients.clone(),
            Arc::new(InMemoryPreparedMealRepository::new()),
            recipes_repo.clone(),
            Arc::new(InMemoryPreparedBatchRepository::new()),
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            Arc::new(SystemClock),
        ),
        MealPlanService::new(
            Arc::new(InMemoryMealPlanRepository::new(consumption.clone())),
            Arc::new(products),
            ingredients.clone(),
            Arc::new(InMemoryPreparedMealRepository::new()),
            recipes_repo.clone(),
            Arc::new(consumption),
            Arc::new(targets.clone()),
            Arc::new(InMemoryHouseholdMemberRepository::new()),
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            Arc::new(InMemoryPreparedBatchRepository::new()),
            PreparationService::new(
                Arc::new(InMemoryPreparedBatchRepository::new()),
                recipes_repo.clone(),
                Arc::new(InMemoryProductRepository::new()),
                Arc::new(InMemoryIngredientRepository::new()),
                Arc::new(InMemoryPreparedMealRepository::new()),
                Arc::new(InMemoryHouseholdSettingsRepository::new()),
                Arc::new(SystemClock),
            ),
            stock_service.clone(),
            Arc::new(SystemClock),
        ),
        meal_templates,
        NutritionTargetService::new(Arc::new(targets), Arc::new(SystemClock)),
        nutrition_plan,
        recipes,
        stock_service.clone(),
        ShoppingService::new(
            Arc::new(InMemoryShoppingCadenceRepository::new()),
            Arc::new(InMemoryShoppingOpportunityRepository::new()),
            Arc::new(InMemoryPurchaseRepository::new()),
            Arc::new(InMemoryShoppingListItemRepository::new()),
            Arc::new(InMemoryShoppingTripRepository::new()),
            finish_shop_repository(),
            Arc::new(InMemoryShoppingSuggestionDismissalRepository::new()),
            Arc::new(InMemoryIngredientRepository::new()),
            Arc::new(InMemoryPreparedMealRepository::new()),
            Arc::new(InMemoryProductRepository::new()),
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            stock_service,
            Arc::new(SystemClock),
        ),
        WeightService::new(
            Arc::new(InMemoryWeightRecordRepository::new()),
            Arc::new(InMemoryWeightGoalRepository::new()),
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            Arc::new(SystemClock),
        ),
        PreparationService::new(
            Arc::new(InMemoryPreparedBatchRepository::new()),
            recipes_repo.clone(),
            Arc::new(InMemoryProductRepository::new()),
            Arc::new(InMemoryIngredientRepository::new()),
            Arc::new(InMemoryPreparedMealRepository::new()),
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            Arc::new(SystemClock),
        ),
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
    let ingredients = Arc::new(InMemoryIngredientRepository::new());
    let recipes_repo = Arc::new(InMemoryRecipeRepository::new());
    let recipes = RecipeService::new(
        recipes_repo.clone(),
        Arc::new(products.clone()),
        ingredients.clone(),
        Arc::new(InMemoryPreparedMealRepository::new()),
        Arc::new(SystemClock),
    );
    let stock_service = StockService::new(
        Arc::new(InMemoryStockRepository::new()),
        Arc::new(InMemoryProductRepository::new()),
        Arc::new(InMemoryIngredientRepository::new()),
        Arc::new(InMemoryPreparedMealRepository::new()),
        Arc::new(InMemoryMealPlanRepository::default()),
        recipes_repo.clone(),
        Arc::new(InMemoryPreparedBatchRepository::new()),
        Arc::new(InMemoryHouseholdSettingsRepository::new()),
        Arc::new(SystemClock),
    );
    let meal_templates = MealTemplateService::new(
        Arc::new(InMemoryMealTemplateRepository::new()),
        Arc::new(InMemoryMealPlanRepository::default()),
        Arc::new(SystemClock),
    );
    let nutrition_plan = NutritionPlanService::new(
        Arc::new(InMemoryMemberBodyProfileRepository::new()),
        Arc::new(InMemoryCalorieCalculationRepository::new()),
        NutritionTargetService::new(Arc::new(targets.clone()), Arc::new(SystemClock)),
        WeightService::new(
            Arc::new(InMemoryWeightRecordRepository::new()),
            Arc::new(InMemoryWeightGoalRepository::new()),
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            Arc::new(SystemClock),
        ),
        Arc::new(InMemoryHouseholdSettingsRepository::new()),
        Arc::new(SystemClock),
    );
    let state = AppState::new(
        CatalogueService::new(
            ingredients.clone(),
            Arc::new(InMemoryPreparedMealRepository::new()),
            Arc::new(products.clone()),
            Arc::new(SystemClock),
        ),
        household.clone(),
        HouseholdSettingsService::new(
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            Arc::new(SystemClock),
        ),
        ConsumptionService::new(
            Arc::new(consumption.clone()),
            Arc::new(products.clone()),
            ingredients.clone(),
            Arc::new(InMemoryPreparedMealRepository::new()),
            recipes_repo.clone(),
            Arc::new(InMemoryPreparedBatchRepository::new()),
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            Arc::new(SystemClock),
        ),
        MealPlanService::new(
            Arc::new(InMemoryMealPlanRepository::new(consumption.clone())),
            Arc::new(products),
            ingredients.clone(),
            Arc::new(InMemoryPreparedMealRepository::new()),
            recipes_repo.clone(),
            Arc::new(consumption),
            Arc::new(targets.clone()),
            Arc::new(InMemoryHouseholdMemberRepository::new()),
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            Arc::new(InMemoryPreparedBatchRepository::new()),
            PreparationService::new(
                Arc::new(InMemoryPreparedBatchRepository::new()),
                recipes_repo.clone(),
                Arc::new(InMemoryProductRepository::new()),
                Arc::new(InMemoryIngredientRepository::new()),
                Arc::new(InMemoryPreparedMealRepository::new()),
                Arc::new(InMemoryHouseholdSettingsRepository::new()),
                Arc::new(SystemClock),
            ),
            stock_service.clone(),
            Arc::new(SystemClock),
        ),
        meal_templates,
        NutritionTargetService::new(Arc::new(targets), Arc::new(SystemClock)),
        nutrition_plan,
        recipes,
        stock_service.clone(),
        ShoppingService::new(
            Arc::new(InMemoryShoppingCadenceRepository::new()),
            Arc::new(InMemoryShoppingOpportunityRepository::new()),
            Arc::new(InMemoryPurchaseRepository::new()),
            Arc::new(InMemoryShoppingListItemRepository::new()),
            Arc::new(InMemoryShoppingTripRepository::new()),
            finish_shop_repository(),
            Arc::new(InMemoryShoppingSuggestionDismissalRepository::new()),
            Arc::new(InMemoryIngredientRepository::new()),
            Arc::new(InMemoryPreparedMealRepository::new()),
            Arc::new(InMemoryProductRepository::new()),
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            stock_service,
            Arc::new(SystemClock),
        ),
        WeightService::new(
            Arc::new(InMemoryWeightRecordRepository::new()),
            Arc::new(InMemoryWeightGoalRepository::new()),
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            Arc::new(SystemClock),
        ),
        PreparationService::new(
            Arc::new(InMemoryPreparedBatchRepository::new()),
            recipes_repo.clone(),
            Arc::new(InMemoryProductRepository::new()),
            Arc::new(InMemoryIngredientRepository::new()),
            Arc::new(InMemoryPreparedMealRepository::new()),
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            Arc::new(SystemClock),
        ),
        Arc::new(DevBasicAuthProvider::new(household, "changeme")),
    );
    let (router, _) = app::build(state);
    let router = app::with_web_client(router, None);

    let (status, _) = get(&router, "/api/v1/meta").await;
    assert_eq!(status, StatusCode::OK);
}
