use std::sync::Arc;

use mmp_core::services::{
    CatalogueService, ConsumptionService, HouseholdService, HouseholdSettingsService,
    MealPlanService, MealTemplateService, NutritionPlanService, NutritionTargetService,
    PreparationService, RecipeService, ShoppingService, StockService, WeightService,
};

use crate::auth::AuthProvider;

#[derive(Clone)]
pub struct AppState {
    pub catalogue: CatalogueService,
    pub household: Arc<HouseholdService>,
    pub household_settings: HouseholdSettingsService,
    pub consumption: ConsumptionService,
    pub meal_plan: MealPlanService,
    pub meal_templates: MealTemplateService,
    pub nutrition_targets: NutritionTargetService,
    pub nutrition_plan: NutritionPlanService,
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
        meal_templates: MealTemplateService,
        nutrition_targets: NutritionTargetService,
        nutrition_plan: NutritionPlanService,
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
            meal_templates,
            nutrition_targets,
            nutrition_plan,
            recipes,
            stock,
            shopping,
            weight,
            preparation,
            auth,
        }
    }
}
