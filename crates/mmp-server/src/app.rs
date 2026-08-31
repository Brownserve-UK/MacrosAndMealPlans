use std::sync::Arc;

use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

use crate::openapi::ApiDoc;
use crate::routes;
use crate::state::AppState;

pub fn build(state: AppState) -> (Router, utoipa::openapi::OpenApi) {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(routes::meta::router())
        .merge(routes::auth::router())
        .merge(routes::ingredients::router())
        .merge(routes::products::router())
        .merge(routes::members::router())
        .merge(routes::users::router())
        .merge(routes::diary::router())
        .merge(routes::meal_plan::router())
        .merge(routes::nutrition_target::router())
        .merge(routes::recipes::router())
        .merge(routes::settings::router())
        .merge(routes::stock::router())
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
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    (router, api)
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

pub fn stub_state() -> AppState {
    use mmp_core::ports::SystemClock;

    let household = Arc::new(mmp_core::services::HouseholdService::new(
        Arc::new(NoopMembers),
        Arc::new(NoopUsers),
        Arc::new(NoopGrants),
        Arc::new(SystemClock),
    ));
    let consumption = Arc::new(NoopConsumptionRecords);
    let products = Arc::new(NoopProducts);
    let targets = Arc::new(NoopNutritionTargets);

    AppState::new(
        mmp_core::services::CatalogueService::new(
            Arc::new(NoopIngredients),
            products.clone(),
            Arc::new(SystemClock),
        ),
        household.clone(),
        mmp_core::services::HouseholdSettingsService::new(
            Arc::new(NoopHouseholdSettings),
            Arc::new(SystemClock),
        ),
        mmp_core::services::DiaryService::new(
            consumption.clone(),
            products.clone(),
            Arc::new(NoopRecipes),
            Arc::new(SystemClock),
        ),
        mmp_core::services::MealPlanService::new(
            Arc::new(NoopMealPlans),
            products,
            Arc::new(NoopRecipes),
            consumption,
            targets.clone(),
            Arc::new(NoopMembers),
            Arc::new(NoopHouseholdSettings),
            Arc::new(SystemClock),
        ),
        mmp_core::services::NutritionTargetService::new(targets, Arc::new(SystemClock)),
        mmp_core::services::RecipeService::new(
            Arc::new(NoopRecipes),
            Arc::new(NoopProducts),
            Arc::new(NoopIngredients),
            Arc::new(SystemClock),
        ),
        mmp_core::services::StockService::new(
            Arc::new(NoopStock),
            Arc::new(NoopProducts),
            Arc::new(NoopMealPlans),
            Arc::new(NoopMembers),
            Arc::new(NoopHouseholdSettings),
            Arc::new(SystemClock),
        ),
        Arc::new(crate::auth::DevBasicAuthProvider::new(household, "")),
    )
}

struct NoopStock;

#[async_trait::async_trait]
impl mmp_core::ports::StockRepository for NoopStock {
    async fn get(
        &self,
        _: mmp_core::domain::StockItemId,
    ) -> mmp_core::Result<Option<mmp_core::domain::StockItem>> {
        Ok(None)
    }
    async fn list(
        &self,
        q: &mmp_core::ports::StockQuery,
    ) -> mmp_core::Result<mmp_core::ports::Paginated<mmp_core::domain::StockItem>> {
        Ok(mmp_core::ports::Paginated::new(vec![], 0, q.page))
    }
    async fn list_for_products(
        &self,
        _: &[mmp_core::domain::ProductId],
    ) -> mmp_core::Result<Vec<mmp_core::domain::StockItem>> {
        Ok(vec![])
    }
    async fn insert(
        &self,
        _: &mmp_core::domain::StockItem,
        _: &mmp_core::domain::NewStockEvent,
    ) -> mmp_core::Result<()> {
        Ok(())
    }
    async fn update(
        &self,
        _: &mmp_core::domain::StockItem,
        _: mmp_core::domain::Revision,
        _: &mmp_core::domain::NewStockEvent,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
    async fn list_events(
        &self,
        _: mmp_core::domain::StockItemId,
    ) -> mmp_core::Result<Vec<mmp_core::domain::StockEvent>> {
        Ok(vec![])
    }
    async fn effects_for_source(
        &self,
        _: mmp_core::domain::StockEffectSource,
        _: uuid::Uuid,
    ) -> mmp_core::Result<Vec<mmp_core::domain::StockEffect>> {
        Ok(vec![])
    }
}

#[async_trait::async_trait]
impl mmp_core::ports::RecipeRepository for NoopRecipes {
    async fn get(
        &self,
        _: mmp_core::domain::RecipeId,
    ) -> mmp_core::Result<Option<mmp_core::domain::Recipe>> {
        Ok(None)
    }
    async fn list(
        &self,
        q: &mmp_core::ports::RecipeQuery,
    ) -> mmp_core::Result<mmp_core::ports::Paginated<mmp_core::domain::RecipeSummary>> {
        Ok(mmp_core::ports::Paginated::new(vec![], 0, q.page))
    }
    async fn insert(&self, _: &mmp_core::domain::Recipe) -> mmp_core::Result<()> {
        Ok(())
    }
    async fn update(
        &self,
        _: &mmp_core::domain::Recipe,
        _: mmp_core::domain::Revision,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
    async fn get_photo(
        &self,
        _: mmp_core::domain::RecipeId,
    ) -> mmp_core::Result<Option<mmp_core::domain::RecipePhoto>> {
        Ok(None)
    }
    async fn update_photo(
        &self,
        _: &mmp_core::domain::Recipe,
        _: mmp_core::domain::Revision,
        _: Option<&mmp_core::domain::RecipePhoto>,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
}

struct NoopIngredients;
struct NoopHouseholdSettings;
struct NoopProducts;

#[async_trait::async_trait]
impl mmp_core::ports::HouseholdSettingsRepository for NoopHouseholdSettings {
    async fn get(&self) -> mmp_core::Result<mmp_core::domain::HouseholdSettings> {
        Ok(mmp_core::domain::HouseholdSettings {
            meal_times: mmp_core::domain::MealTimes {
                breakfast: time::macros::time!(08:00),
                lunch: time::macros::time!(12:30),
                dinner: time::macros::time!(18:00),
            },
            missing_stock_interpretation: mmp_core::domain::MissingStockInterpretation::Unknown,
            default_all_members_participate: true,
            revision: mmp_core::domain::Revision::INITIAL,
            created_at: time::OffsetDateTime::UNIX_EPOCH,
            updated_at: time::OffsetDateTime::UNIX_EPOCH,
        })
    }
    async fn update(
        &self,
        _: &mmp_core::domain::HouseholdSettings,
        _: mmp_core::domain::Revision,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
}
struct NoopMembers;
struct NoopUsers;
struct NoopGrants;
struct NoopConsumptionRecords;
struct NoopMealPlans;
struct NoopNutritionTargets;
struct NoopRecipes;

#[async_trait::async_trait]
impl mmp_core::ports::NutritionTargetRepository for NoopNutritionTargets {
    async fn get(
        &self,
        _: mmp_core::domain::NutritionTargetId,
    ) -> mmp_core::Result<Option<mmp_core::domain::NutritionTarget>> {
        Ok(None)
    }
    async fn list_for_member(
        &self,
        _: mmp_core::domain::HouseholdMemberId,
    ) -> mmp_core::Result<Vec<mmp_core::domain::NutritionTarget>> {
        Ok(vec![])
    }
    async fn insert(&self, _: &mmp_core::domain::NutritionTarget) -> mmp_core::Result<()> {
        Ok(())
    }
    async fn update(
        &self,
        _: &mmp_core::domain::NutritionTarget,
        _: mmp_core::domain::Revision,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
    async fn delete(
        &self,
        _: mmp_core::domain::NutritionTargetId,
        _: mmp_core::domain::Revision,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
}

#[async_trait::async_trait]
impl mmp_core::ports::IngredientRepository for NoopIngredients {
    async fn get(
        &self,
        _: mmp_core::domain::IngredientId,
    ) -> mmp_core::Result<Option<mmp_core::domain::Ingredient>> {
        Ok(None)
    }
    async fn find_by_name(
        &self,
        _: &str,
    ) -> mmp_core::Result<Option<mmp_core::domain::Ingredient>> {
        Ok(None)
    }
    async fn find_by_seed_key(
        &self,
        _: &str,
    ) -> mmp_core::Result<Option<mmp_core::domain::Ingredient>> {
        Ok(None)
    }
    async fn list(
        &self,
        q: &mmp_core::ports::IngredientQuery,
    ) -> mmp_core::Result<mmp_core::ports::Paginated<mmp_core::domain::Ingredient>> {
        Ok(mmp_core::ports::Paginated::new(vec![], 0, q.page))
    }
    async fn insert(&self, _: &mmp_core::domain::Ingredient) -> mmp_core::Result<()> {
        Ok(())
    }
    async fn update(
        &self,
        _: &mmp_core::domain::Ingredient,
        _: mmp_core::domain::Revision,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
}

#[async_trait::async_trait]
impl mmp_core::ports::ProductRepository for NoopProducts {
    async fn count_by_ingredient(
        &self,
        _: &[mmp_core::domain::IngredientId],
    ) -> mmp_core::Result<std::collections::HashMap<mmp_core::domain::IngredientId, i64>> {
        Ok(Default::default())
    }
    async fn list_by_ingredient(
        &self,
        _: &[mmp_core::domain::IngredientId],
    ) -> mmp_core::Result<
        std::collections::HashMap<mmp_core::domain::IngredientId, Vec<mmp_core::domain::Product>>,
    > {
        Ok(Default::default())
    }
    async fn get(
        &self,
        _: mmp_core::domain::ProductId,
    ) -> mmp_core::Result<Option<mmp_core::domain::Product>> {
        Ok(None)
    }
    async fn find_by_barcode(
        &self,
        _: &str,
    ) -> mmp_core::Result<Option<mmp_core::domain::Product>> {
        Ok(None)
    }
    async fn find_by_seed_key(
        &self,
        _: &str,
    ) -> mmp_core::Result<Option<mmp_core::domain::Product>> {
        Ok(None)
    }
    async fn list(
        &self,
        q: &mmp_core::ports::ProductQuery,
    ) -> mmp_core::Result<mmp_core::ports::Paginated<mmp_core::domain::Product>> {
        Ok(mmp_core::ports::Paginated::new(vec![], 0, q.page))
    }
    async fn insert(&self, _: &mmp_core::domain::Product) -> mmp_core::Result<()> {
        Ok(())
    }
    async fn update(
        &self,
        _: &mmp_core::domain::Product,
        _: mmp_core::domain::Revision,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
}

#[async_trait::async_trait]
impl mmp_core::ports::HouseholdMemberRepository for NoopMembers {
    async fn get(
        &self,
        _: mmp_core::domain::HouseholdMemberId,
    ) -> mmp_core::Result<Option<mmp_core::domain::HouseholdMember>> {
        Ok(None)
    }
    async fn find_by_display_name(
        &self,
        _: &str,
    ) -> mmp_core::Result<Option<mmp_core::domain::HouseholdMember>> {
        Ok(None)
    }
    async fn find_by_linked_user(
        &self,
        _: mmp_core::domain::UserId,
    ) -> mmp_core::Result<Option<mmp_core::domain::HouseholdMember>> {
        Ok(None)
    }
    async fn list(
        &self,
        q: &mmp_core::ports::MemberQuery,
    ) -> mmp_core::Result<mmp_core::ports::Paginated<mmp_core::domain::HouseholdMember>> {
        Ok(mmp_core::ports::Paginated::new(vec![], 0, q.page))
    }
    async fn insert(&self, _: &mmp_core::domain::HouseholdMember) -> mmp_core::Result<()> {
        Ok(())
    }
    async fn update(
        &self,
        _: &mmp_core::domain::HouseholdMember,
        _: mmp_core::domain::Revision,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
}

#[async_trait::async_trait]
impl mmp_core::ports::UserRepository for NoopUsers {
    async fn get(
        &self,
        _: mmp_core::domain::UserId,
    ) -> mmp_core::Result<Option<mmp_core::domain::User>> {
        Ok(None)
    }
    async fn find_by_username(&self, _: &str) -> mmp_core::Result<Option<mmp_core::domain::User>> {
        Ok(None)
    }
    async fn list(
        &self,
        q: &mmp_core::ports::UserQuery,
    ) -> mmp_core::Result<mmp_core::ports::Paginated<mmp_core::domain::User>> {
        Ok(mmp_core::ports::Paginated::new(vec![], 0, q.page))
    }
    async fn count_with_role(&self, _: mmp_core::domain::Role, _: bool) -> mmp_core::Result<i64> {
        Ok(0)
    }
    async fn insert(&self, _: &mmp_core::domain::User) -> mmp_core::Result<()> {
        Ok(())
    }
    async fn update(
        &self,
        _: &mmp_core::domain::User,
        _: mmp_core::domain::Revision,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
}

#[async_trait::async_trait]
impl mmp_core::ports::ConsumptionRecordRepository for NoopConsumptionRecords {
    async fn get(
        &self,
        _: mmp_core::domain::ConsumptionRecordId,
    ) -> mmp_core::Result<Option<mmp_core::domain::ConsumptionRecord>> {
        Ok(None)
    }
    async fn list(
        &self,
        q: &mmp_core::ports::ConsumptionQuery,
    ) -> mmp_core::Result<mmp_core::ports::Paginated<mmp_core::domain::ConsumptionRecord>> {
        Ok(mmp_core::ports::Paginated::new(vec![], 0, q.page))
    }
    async fn list_period(
        &self,
        _: mmp_core::domain::HouseholdMemberId,
        _: time::Date,
        _: time::Date,
    ) -> mmp_core::Result<Vec<mmp_core::domain::ConsumptionRecord>> {
        Ok(vec![])
    }
    async fn list_for_meal_plan_entry(
        &self,
        _: mmp_core::domain::MealPlanEntryId,
    ) -> mmp_core::Result<Vec<mmp_core::domain::ConsumptionRecord>> {
        Ok(vec![])
    }
    async fn insert(
        &self,
        _: &mmp_core::domain::ConsumptionRecord,
        _: &mmp_core::ports::StockWrite,
    ) -> mmp_core::Result<Vec<mmp_core::domain::StockOutcome>> {
        Ok(vec![])
    }
    async fn update(
        &self,
        _: &mmp_core::domain::ConsumptionRecord,
        _: mmp_core::domain::Revision,
        _: &mmp_core::ports::StockWrite,
    ) -> mmp_core::Result<(
        mmp_core::ports::UpdateOutcome,
        Vec<mmp_core::domain::StockOutcome>,
    )> {
        Ok((mmp_core::ports::UpdateOutcome::NotFound, vec![]))
    }
    async fn delete(
        &self,
        _: mmp_core::domain::ConsumptionRecordId,
        _: mmp_core::domain::Revision,
        _: &mmp_core::ports::StockWrite,
    ) -> mmp_core::Result<(
        mmp_core::ports::UpdateOutcome,
        Vec<mmp_core::domain::StockOutcome>,
    )> {
        Ok((mmp_core::ports::UpdateOutcome::NotFound, vec![]))
    }
}

#[async_trait::async_trait]
impl mmp_core::ports::MealPlanRepository for NoopMealPlans {
    async fn get(
        &self,
        _: mmp_core::domain::MealPlanEntryId,
    ) -> mmp_core::Result<Option<mmp_core::domain::MealPlanEntry>> {
        Ok(None)
    }
    async fn list(
        &self,
        _: &mmp_core::ports::MealPlanQuery,
    ) -> mmp_core::Result<Vec<mmp_core::domain::MealPlanEntry>> {
        Ok(vec![])
    }
    async fn insert(&self, _: &mmp_core::domain::MealPlanEntry) -> mmp_core::Result<()> {
        Ok(())
    }
    async fn update(
        &self,
        _: &mmp_core::domain::MealPlanEntry,
        _: mmp_core::domain::Revision,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
    async fn delete(
        &self,
        _: mmp_core::domain::MealPlanEntryId,
        _: mmp_core::domain::Revision,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
    async fn resolve(
        &self,
        _: &mmp_core::domain::MealPlanEntry,
        _: mmp_core::domain::Revision,
        _: &[mmp_core::domain::ConsumptionRecord],
        _: &mmp_core::ports::StockWrite,
    ) -> mmp_core::Result<(
        mmp_core::ports::UpdateOutcome,
        Vec<mmp_core::domain::StockOutcome>,
    )> {
        Ok((mmp_core::ports::UpdateOutcome::NotFound, vec![]))
    }
    async fn reopen(
        &self,
        _: &mmp_core::domain::MealPlanEntry,
        _: mmp_core::domain::Revision,
        _: &[mmp_core::domain::ConsumptionRecordId],
        _: &mmp_core::ports::StockWrite,
    ) -> mmp_core::Result<(
        mmp_core::ports::UpdateOutcome,
        Vec<mmp_core::domain::StockOutcome>,
    )> {
        Ok((mmp_core::ports::UpdateOutcome::NotFound, vec![]))
    }
    async fn set_participants(
        &self,
        _: &mmp_core::domain::MealPlanEntry,
        _: mmp_core::domain::Revision,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
    async fn resolve_component(
        &self,
        _: mmp_core::domain::MealPlanEntryId,
        _: &mmp_core::ports::MealPlanComponentUpdate<'_>,
        _: &[mmp_core::domain::MealParticipant],
        _: mmp_core::domain::Revision,
        _: Option<&mmp_core::domain::ConsumptionRecord>,
        _: &mmp_core::ports::StockWrite,
    ) -> mmp_core::Result<(
        mmp_core::ports::UpdateOutcome,
        Vec<mmp_core::domain::StockOutcome>,
    )> {
        Ok((mmp_core::ports::UpdateOutcome::NotFound, vec![]))
    }
    async fn reopen_component(
        &self,
        _: mmp_core::domain::MealPlanEntryId,
        _: &mmp_core::ports::MealPlanComponentUpdate<'_>,
        _: &[mmp_core::domain::MealParticipant],
        _: mmp_core::domain::Revision,
        _: Option<mmp_core::domain::ConsumptionRecordId>,
        _: &mmp_core::ports::StockWrite,
    ) -> mmp_core::Result<(
        mmp_core::ports::UpdateOutcome,
        Vec<mmp_core::domain::StockOutcome>,
    )> {
        Ok((mmp_core::ports::UpdateOutcome::NotFound, vec![]))
    }
}

#[async_trait::async_trait]
impl mmp_core::ports::AccessGrantRepository for NoopGrants {
    async fn list_for_member(
        &self,
        _: mmp_core::domain::HouseholdMemberId,
    ) -> mmp_core::Result<Vec<mmp_core::domain::MemberAccessGrant>> {
        Ok(vec![])
    }
    async fn exists(
        &self,
        _: mmp_core::domain::UserId,
        _: mmp_core::domain::HouseholdMemberId,
        _: mmp_core::domain::AccessScope,
    ) -> mmp_core::Result<bool> {
        Ok(false)
    }
    async fn upsert(&self, _: &mmp_core::domain::MemberAccessGrant) -> mmp_core::Result<()> {
        Ok(())
    }
    async fn revoke(
        &self,
        _: mmp_core::domain::UserId,
        _: mmp_core::domain::HouseholdMemberId,
        _: mmp_core::domain::AccessScope,
    ) -> mmp_core::Result<bool> {
        Ok(false)
    }
}
