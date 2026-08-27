use std::sync::Arc;

use mmp_core::services::{
    CatalogueService, DiaryService, HouseholdService, HouseholdSettingsService, MealPlanService,
    NutritionTargetService, RecipeService,
};

use crate::auth::AuthProvider;

#[derive(Clone)]
pub struct AppState {
    pub catalogue: CatalogueService,
    pub household: Arc<HouseholdService>,
    pub household_settings: HouseholdSettingsService,
    pub diary: DiaryService,
    pub meal_plan: MealPlanService,
    pub nutrition_targets: NutritionTargetService,
    pub recipes: RecipeService,
    pub auth: Arc<dyn AuthProvider>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        catalogue: CatalogueService,
        household: Arc<HouseholdService>,
        household_settings: HouseholdSettingsService,
        diary: DiaryService,
        meal_plan: MealPlanService,
        nutrition_targets: NutritionTargetService,
        recipes: RecipeService,
        auth: Arc<dyn AuthProvider>,
    ) -> Self {
        Self {
            catalogue,
            household,
            household_settings,
            diary,
            meal_plan,
            nutrition_targets,
            recipes,
            auth,
        }
    }
}
