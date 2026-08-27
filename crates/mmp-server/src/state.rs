use std::sync::Arc;

use mmp_core::services::{
    CatalogueService, DiaryService, HouseholdService, HouseholdSettingsService, MealPlanService,
    NutritionTargetService,
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
    pub auth: Arc<dyn AuthProvider>,
}

impl AppState {
    pub fn new(
        catalogue: CatalogueService,
        household: Arc<HouseholdService>,
        household_settings: HouseholdSettingsService,
        diary: DiaryService,
        meal_plan: MealPlanService,
        nutrition_targets: NutritionTargetService,
        auth: Arc<dyn AuthProvider>,
    ) -> Self {
        Self {
            catalogue,
            household,
            household_settings,
            diary,
            meal_plan,
            nutrition_targets,
            auth,
        }
    }
}
