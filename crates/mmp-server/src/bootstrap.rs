use std::sync::Arc;

use anyhow::Context;
use mmp_core::domain::{NewHouseholdMember, NewUser, Role};
use mmp_core::ports::SystemClock;
use mmp_core::services::{
    CatalogueService, DiaryService, HouseholdService, HouseholdSettingsService, MealPlanService,
    NutritionTargetService, RecipeService, SeedIngredient, SeedReport,
};
use mmp_postgres::PgPool;
use mmp_postgres::{
    PgAccessGrantRepository, PgConsumptionRecordRepository, PgHouseholdMemberRepository,
    PgHouseholdSettingsRepository, PgIngredientRepository, PgMealPlanRepository,
    PgNutritionTargetRepository, PgProductRepository, PgRecipeRepository, PgUserRepository,
};

use crate::auth::DevBasicAuthProvider;
use crate::config::Config;
use crate::state::AppState;

const SEED_INGREDIENTS: &str = include_str!("../seed/ingredients.json");

pub fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,mmp_server=debug,sqlx=warn"));
    fmt().with_env_filter(filter).init();
}

pub fn load_config() -> anyhow::Result<Config> {
    let _ = dotenvy::dotenv();
    Config::from_env().context("reading configuration from the environment")
}

pub async fn connect_and_migrate(config: &Config) -> anyhow::Result<PgPool> {
    let pool = mmp_postgres::connect(&config.database_url, 10)
        .await
        .with_context(|| {
            format!(
                "connecting to PostgreSQL at `{}`. Is the database running? \
                 Try `just db-up`, and check DATABASE_URL in your .env.",
                redact(&config.database_url)
            )
        })?;

    mmp_postgres::migrate(&pool)
        .await
        .context("applying database migrations")?;

    Ok(pool)
}

// IMPORTANT: This strips the password so connection strings can be logged without leaking secrets for error reporting.
fn redact(database_url: &str) -> String {
    match (database_url.find("://"), database_url.rfind('@')) {
        (Some(scheme_end), Some(at)) if at > scheme_end => {
            format!(
                "{}://***@{}",
                &database_url[..scheme_end],
                &database_url[at + 1..]
            )
        }
        _ => database_url.to_owned(),
    }
}

pub fn catalogue_service(pool: &PgPool) -> CatalogueService {
    CatalogueService::new(
        Arc::new(PgIngredientRepository::new(pool.clone())),
        Arc::new(PgProductRepository::new(pool.clone())),
        Arc::new(SystemClock),
    )
}

pub fn household_service(pool: &PgPool) -> Arc<HouseholdService> {
    Arc::new(HouseholdService::new(
        Arc::new(PgHouseholdMemberRepository::new(pool.clone())),
        Arc::new(PgUserRepository::new(pool.clone())),
        Arc::new(PgAccessGrantRepository::new(pool.clone())),
        Arc::new(SystemClock),
    ))
}

pub fn diary_service(pool: &PgPool) -> DiaryService {
    DiaryService::new(
        Arc::new(PgConsumptionRecordRepository::new(pool.clone())),
        Arc::new(PgProductRepository::new(pool.clone())),
        Arc::new(PgRecipeRepository::new(pool.clone())),
        Arc::new(SystemClock),
    )
}

pub fn app_state(config: &Config, pool: &PgPool) -> AppState {
    let household = household_service(pool);
    let products = Arc::new(PgProductRepository::new(pool.clone()));
    let recipes_repo = Arc::new(PgRecipeRepository::new(pool.clone()));
    let consumption = Arc::new(PgConsumptionRecordRepository::new(pool.clone()));
    let diary = DiaryService::new(
        consumption.clone(),
        products.clone(),
        recipes_repo.clone(),
        Arc::new(SystemClock),
    );
    let targets = Arc::new(PgNutritionTargetRepository::new(pool.clone()));
    let meal_plan = MealPlanService::new(
        Arc::new(PgMealPlanRepository::new(pool.clone())),
        products,
        recipes_repo,
        consumption,
        targets.clone(),
        Arc::new(SystemClock),
    );
    let nutrition_targets = NutritionTargetService::new(targets, Arc::new(SystemClock));
    let household_settings = HouseholdSettingsService::new(
        Arc::new(PgHouseholdSettingsRepository::new(pool.clone())),
        Arc::new(SystemClock),
    );
    let recipes = RecipeService::new(
        Arc::new(PgRecipeRepository::new(pool.clone())),
        Arc::new(PgProductRepository::new(pool.clone())),
        Arc::new(PgIngredientRepository::new(pool.clone())),
        Arc::new(SystemClock),
    );
    AppState::new(
        catalogue_service(pool),
        household.clone(),
        household_settings,
        diary,
        meal_plan,
        nutrition_targets,
        recipes,
        Arc::new(DevBasicAuthProvider::new(
            household,
            config.dev_password.clone(),
        )),
    )
}

pub async fn ensure_bootstrap_user(
    household: &HouseholdService,
    username: &str,
) -> anyhow::Result<()> {
    let existing = household
        .list_users(&Default::default())
        .await
        .context("checking whether any account exists")?;

    if existing.total > 0 {
        return Ok(());
    }

    let user = household
        .create_user(NewUser {
            id: None,
            username: username.to_owned(),
            display_name: None,
            roles: vec![Role::Admin],
        })
        .await
        .context("creating the bootstrap admin account")?;

    let member = household
        .create_member(NewHouseholdMember {
            id: None,
            display_name: username.to_owned(),
            linked_user_id: Some(user.id),
        })
        .await
        .context("creating the bootstrap household member")?;

    tracing::info!(
        username = %user.username,
        member = %member.display_name,
        "created the first admin account and linked a household member"
    );
    Ok(())
}

pub fn seed_ingredients() -> anyhow::Result<Vec<SeedIngredient>> {
    serde_json::from_str(SEED_INGREDIENTS).context("parsing the bundled ingredient seed data")
}

pub async fn apply_seed(catalogue: &CatalogueService) -> anyhow::Result<SeedReport> {
    let seeds = seed_ingredients()?;
    catalogue
        .apply_seed_ingredients(&seeds)
        .await
        .context("applying the ingredient seed catalogue")
}

#[cfg(test)]
#[path = "bootstrap_tests.rs"]
mod tests;
