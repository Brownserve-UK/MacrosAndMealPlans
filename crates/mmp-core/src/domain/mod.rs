mod access;
mod body_profile;
mod calorie_target;
mod consumption;
mod coverage;
mod household;
mod household_settings;
mod ids;
mod ingredient;
mod macro_target;
mod meal_item;
mod meal_plan;
mod meal_template;
mod nutrition;
mod nutrition_target;
mod patch;
mod prepared;
mod prepared_meal;
mod product;
mod provenance;
mod quantity;
mod recipe;
mod shopping;
mod stock;
mod str_enum;
mod weight;

pub use access::{AccessScope, Permission, Role, UnknownAccessScope, UnknownRole};
pub use body_profile::{
    HabitualActivity, MemberBodyProfile, MemberBodyProfilePatch, Sex, UnknownHabitualActivity,
    UnknownSex, age_on,
};
pub use calorie_target::{
    ACTIVITY_SOURCE_SELF_REPORTED, CALORIE_FORMULA, CalorieCalculation, CalorieCalculationInput,
    Pace, calculate,
};
pub use consumption::{
    AmountError, ConsumedAmount, ConsumedNutrition, ConsumptionRecord, ConsumptionRecordPatch,
    NewConsumptionRecord, NutritionQuality, UnknownNutritionQuality, generic_food_nutrition,
    mean_nutrition, nutrition_for, nutrition_from_draws, recipe_nutrition_for, sum_nutrition,
};
pub use coverage::{Coverage, UncoveredClaim, cover};
pub use household::{
    HouseholdMember, HouseholdMemberPatch, MAX_USERNAME_LEN, MIN_USERNAME_LEN, MemberAccessGrant,
    NewHouseholdMember, NewUser, User, UserPatch,
};
pub use household_settings::{
    HouseholdSettings, HouseholdSettingsPatch, MealTimes, MissingStockInterpretation, SectionOrder,
    UnknownMissingStockInterpretation,
};
pub use ids::{
    CalorieCalculationId, ConsumptionRecordId, HouseholdMemberId, IngredientId, ProductId,
    Revision, UserId,
};
pub use ids::{
    MealGuestAllocationId, MealGuestGroupId, MealParticipantAllocationId, MealParticipantId,
    MealPlanComponentId, MealPlanEntryId, MealTemplateComponentId, MealTemplateId,
    NutritionTargetId, PreparedBatchId, PreparedMealId, PurchaseId, RecipeComponentId, RecipeId,
    RecipeInstructionId, ShoppingListItemId, ShoppingOpportunityId, ShoppingTripId,
    ShoppingTripRowId, StockEffectId, StockEventId, StockItemId, WeightGoalId, WeightRecordId,
};
pub use ingredient::{
    Ingredient, IngredientPatch, IngredientSummary, MAX_NAME_LEN, NewIngredient, validate_name,
};
pub use macro_target::{
    MacroTargets, NutritionEmphasis, UnknownNutritionEmphasis, suggest_macro_targets,
};
pub use meal_item::{MealItemRef, UnknownMealItemRef};
pub use meal_plan::{
    ActualMealPlanComponent, AllocationOutcome, Assumption, AssumptionRules, ChangedMealOutcome,
    ComponentPreparation, ConfirmMealPlanComponent, ConfirmMealPlanEntry, MealGuestAllocation,
    MealGuestGroup, MealOptOut, MealParticipant, MealParticipantAllocation, MealPlanComponent,
    MealPlanComponentSnapshot, MealPlanEntry, MealPlanEntryPatch, MealPlanScope, MealPlanStatus,
    MealSlot, NewMealGuestAllocation, NewMealGuestGroup, NewMealParticipant,
    NewMealParticipantAllocation, NewMealPlanComponent, NewMealPlanEntry, OutcomeActor,
    ParticipantStatus, ReplacementItem, ReviewMealOutcomes, ReviewedGuestOutcome,
    ReviewedMealOutcome, ReviewedMemberOutcome, SetMealParticipants, SlotAttendance,
    UnknownMealPlanScope, UnknownMealPlanStatus, UnknownMealSlot, UnknownParticipantStatus,
    actual_components_for_member, allocated_total, apply_equal_shares, build_guest_results,
    build_participant, component_still_eaten, derive_component_status, derive_entry_status,
    derive_guest_status, derive_participant_status, effective_consumption, equal_split,
    find_component, forecast_remaining, has_explicit_allocations, make_components,
    merge_components, merge_guest_group, merge_participant, outcomes_for_component,
    participant_status_to_meal, pending_component_ids, preparation_for, replacements_for,
    require_allocation_planned, require_editable, require_household_attendance, require_planned,
    require_subject_pending, set_allocation, sync_allocations, validate_actual_components,
    validate_components, validate_guest_groups, validate_participants,
};
pub(crate) use meal_plan::{MEAL_PLAN_COMPONENT, MEAL_PLAN_ENTRY};
pub use meal_template::{
    MAX_NAME_LEN as MEAL_TEMPLATE_MAX_NAME_LEN, MealTemplate, MealTemplateComponent,
    MealTemplatePatch, NewMealTemplate, NewMealTemplateComponent,
    make_components as make_template_components, validate_template_components,
};
pub use nutrition::NutritionFacts;
pub use nutrition_target::{
    NUTRIENT_KEYS, NewNutritionTarget, NutritionGoals, NutritionGoalsPatch, NutritionTarget,
    NutritionTargetPatch, TargetDirection, TargetSource, UnknownTargetSource,
    default_direction_for, direction_for, resolve_on, validate_goals,
};
pub use patch::Patch;
pub use prepared::{
    CHILLED_LEFTOVER_DAYS, FROZEN_LEFTOVER_DAYS, NewPreparedBatch, PortionPlacement,
    PreparationSource, PreparedBatch, cooked_deadline, validate_placements,
};
pub use prepared_meal::{NewPreparedMeal, PreparedMeal, PreparedMealPatch, PreparedMealSummary};
pub use product::{
    MAX_BARCODE_LEN, MAX_SHORT_TEXT_LEN, MIN_BARCODE_LEN, NewProduct, Product, ProductPatch,
};
pub use provenance::{CatalogueOrigin, Provenance, UnknownOrigin};
pub use quantity::{ConversionError, Dimension, Quantity, Unit, UnknownUnit};
pub use recipe::{
    DerivedNutrition, Fulfilment, MAX_REQUIREMENT_TEXT_LEN, MAX_SERVINGS, MealCategory, NewRecipe,
    NewRecipeComponent, NewRecipeInstruction, Recipe, RecipeComponent, RecipeInstruction,
    RecipePatch, RecipePhoto, RecipePhotoDerivatives, RecipeRequirement, RecipeSummary,
    RecipeVisibility, UnknownMealCategory, UnknownRecipeVisibility, normalise_countries,
    normalise_optional_text, normalise_tags, normalise_unique, recipe_nutrition,
    recipe_nutrition_detailed,
};
pub use shopping::{
    Assignment, Certainty, ExceptionState, NewPurchase, NewShoppingCadence, NewShoppingListItem,
    OpportunityException, OpportunityState, Purchase, PurchasePatch, PurchaseState,
    ShoppingCadence, ShoppingListItem, ShoppingListItemPatch, ShoppingOpportunity,
    ShoppingRequirement, ShoppingSection, ShoppingTrip, ShoppingTripRow, SuggestionReason,
    TripState, UnknownExceptionState, UnknownOpportunityState, UnknownPurchaseState,
    UnknownShoppingSection, UnknownTripState, assign, expand_opportunities, week_day_from_number,
    week_day_number,
};
pub use stock::{
    AppliedDelta, Availability, AvailabilityReport, Confidence, CookedFoodAvailability,
    DeductionCandidates, DeductionPlan, DeductionTarget, DemandClaim, DemandGap, DemandSubject,
    IngredientAvailability, MissingStock, NewStockEffect, NewStockEvent, NewStockItem, PlannedTake,
    PreparedMealAvailability, ProductAvailability, ReleasePlan, Shortfall, SourceDate,
    SourceDateKind, StockEffect, StockEffectSource, StockEffectState, StockEvent, StockEventKind,
    StockEventSource, StockItem, StockItemPatch, StockLevel, StockOutcome, StockSubject,
    StorageLocation, TrackingMode, UnknownSourceDateKind, UnknownStockEffectSource,
    UnknownStockEffectState, UnknownStockEventKind, UnknownStorageLocation, UnknownTrackingMode,
    UsabilityDeadline, apply_take, plan_deduction, plan_release,
};
pub use weight::{
    GoalAmounts, GoalProjection, NewWeightGoal, NewWeightRecord, UnknownWeightDisplay,
    UnknownWeightObjective, UnknownWeightSource, WeightDisplay, WeightGoal, WeightGoalPatch,
    WeightObjective, WeightRecord, WeightRecordPatch, WeightSource, current_weight, latest_per_day,
    project_goal,
};
