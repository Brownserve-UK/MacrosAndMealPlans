use std::sync::Arc;

use mmp_core::services::{
    CatalogueService, ConsumptionService, HouseholdService, HouseholdSettingsService,
    MealPlanService, NutritionTargetService, PreparationService, RecipeService, ShoppingService,
    StockService, WeightService,
};

use crate::auth::AuthProvider;

#[derive(Clone)]
pub struct AppState {
    pub catalogue: CatalogueService,
    pub household: Arc<HouseholdService>,
    pub household_settings: HouseholdSettingsService,
    pub consumption: ConsumptionService,
    pub meal_plan: MealPlanService,
    pub nutrition_targets: NutritionTargetService,
    pub recipes: RecipeService,
    pub stock: StockService,
    pub shopping: ShoppingService,
    pub weight: WeightService,
    pub preparation: PreparationService,
    pub auth: Arc<dyn AuthProvider>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        catalogue: CatalogueService,
        household: Arc<HouseholdService>,
        household_settings: HouseholdSettingsService,
        consumption: ConsumptionService,
        meal_plan: MealPlanService,
        nutrition_targets: NutritionTargetService,
        recipes: RecipeService,
        stock: StockService,
        shopping: ShoppingService,
        weight: WeightService,
        preparation: PreparationService,
        auth: Arc<dyn AuthProvider>,
    ) -> Self {
        Self {
            catalogue,
            household,
            household_settings,
            consumption,
            meal_plan,
            nutrition_targets,
            recipes,
            stock,
            shopping,
            weight,
            preparation,
            auth,
        }
    }
}
