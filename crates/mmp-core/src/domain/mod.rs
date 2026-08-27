mod access;
mod consumption;
mod household;
mod ids;
mod ingredient;
mod meal_plan;
mod nutrition;
mod nutrition_target;
mod patch;
mod product;
mod provenance;
mod quantity;

pub use access::{AccessScope, Permission, Role, UnknownAccessScope, UnknownRole};
pub use consumption::{
    AmountError, ConsumedAmount, ConsumedNutrition, ConsumptionRecord, ConsumptionRecordPatch,
    NewConsumptionRecord, NutritionQuality, UnknownNutritionQuality, nutrition_for, sum_nutrition,
};
pub use household::{
    HouseholdMember, HouseholdMemberPatch, MAX_USERNAME_LEN, MIN_USERNAME_LEN, MemberAccessGrant,
    NewHouseholdMember, NewUser, User, UserPatch,
};
pub use ids::{ConsumptionRecordId, HouseholdMemberId, IngredientId, ProductId, Revision, UserId};
pub use ids::{MealPlanComponentId, MealPlanEntryId, NutritionTargetId};
pub use ingredient::{
    Ingredient, IngredientPatch, IngredientSummary, MAX_NAME_LEN, NewIngredient, validate_name,
};
pub use meal_plan::{
    ActualMealPlanComponent, ConfirmMealPlanComponent, ConfirmMealPlanEntry, MealPlanComponent,
    MealPlanComponentSnapshot, MealPlanEntry, MealPlanEntryPatch, MealPlanStatus, MealSlot,
    NewMealPlanComponent, NewMealPlanEntry, UnknownMealPlanStatus, UnknownMealSlot,
    validate_components,
};
pub use nutrition::NutritionFacts;
pub use nutrition_target::{
    NUTRIENT_KEYS, NewNutritionTarget, NutritionGoals, NutritionGoalsPatch, NutritionTarget,
    NutritionTargetPatch, TargetDirection, direction_for, resolve_on, validate_goals,
};
pub use patch::Patch;
pub use product::{
    MAX_BARCODE_LEN, MAX_SHORT_TEXT_LEN, MIN_BARCODE_LEN, NewProduct, Product, ProductPatch,
};
pub use provenance::{CatalogueOrigin, Provenance, UnknownOrigin};
pub use quantity::{ConversionError, Dimension, Quantity, Unit, UnknownUnit};
