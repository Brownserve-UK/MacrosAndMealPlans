mod access_grant;
mod body_profile;
mod calorie_calculation;
mod consumption_record;
mod error;
mod household_member;
mod household_settings;
mod ingredient;
mod meal_plan;
mod meal_template;
mod nutrition_target;
mod prepared;
mod prepared_meal;
mod product;
mod recipe;
mod rows;
mod shopping;
mod stock;
mod user;
mod weight;

pub use access_grant::PgAccessGrantRepository;
pub use body_profile::PgMemberBodyProfileRepository;
pub use calorie_calculation::PgCalorieCalculationRepository;
pub use consumption_record::PgConsumptionRecordRepository;
pub use household_member::PgHouseholdMemberRepository;
pub use household_settings::PgHouseholdSettingsRepository;
pub use ingredient::PgIngredientRepository;
pub use meal_plan::PgMealPlanRepository;
pub use meal_template::PgMealTemplateRepository;
pub use nutrition_target::PgNutritionTargetRepository;
pub use prepared::PgPreparedBatchRepository;
pub use prepared_meal::PgPreparedMealRepository;
pub use product::PgProductRepository;
pub use recipe::PgRecipeRepository;
pub use shopping::{
    PgFinishShopRepository, PgPurchaseRepository, PgShoppingCadenceRepository,
    PgShoppingListItemRepository, PgShoppingOpportunityRepository,
    PgShoppingSuggestionDismissalRepository, PgShoppingTripRepository,
};
pub use stock::PgStockRepository;
pub use user::PgUserRepository;
pub use weight::{PgWeightGoalRepository, PgWeightRecordRepository};

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

pub use sqlx::PgPool;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);

pub type MigrateError = sqlx::migrate::MigrateError;

pub async fn connect(database_url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(CONNECT_TIMEOUT)
        .connect(database_url)
        .await
}

pub async fn migrate(pool: &PgPool) -> Result<(), MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
