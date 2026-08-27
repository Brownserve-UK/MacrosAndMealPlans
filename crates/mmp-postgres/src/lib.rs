mod access_grant;
mod consumption_record;
mod error;
mod household_member;
mod household_settings;
mod ingredient;
mod meal_plan;
mod nutrition_target;
mod product;
mod rows;
mod user;

pub use access_grant::PgAccessGrantRepository;
pub use consumption_record::PgConsumptionRecordRepository;
pub use household_member::PgHouseholdMemberRepository;
pub use household_settings::PgHouseholdSettingsRepository;
pub use ingredient::PgIngredientRepository;
pub use meal_plan::PgMealPlanRepository;
pub use nutrition_target::PgNutritionTargetRepository;
pub use product::PgProductRepository;
pub use user::PgUserRepository;

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
