mod access;
mod consumption;
mod coverage;
mod household;
mod household_settings;
mod ids;
mod ingredient;
mod meal_item;
mod meal_plan;
mod nutrition;
mod nutrition_target;
mod patch;
mod product;
mod provenance;
mod quantity;
mod recipe;
mod shopping;
mod stock;
mod str_enum;
mod weight;

pub use access::{AccessScope, Permission, Role, UnknownAccessScope, UnknownRole};
pub use consumption::{
    AmountError, ConsumedAmount, ConsumedNutrition, ConsumptionRecord, ConsumptionRecordPatch,
    NewConsumptionRecord, NutritionQuality, UnknownNutritionQuality, mean_nutrition, nutrition_for,
    recipe_nutrition_for, sum_nutrition,
};
pub use coverage::{Coverage, cover};
pub use household::{
    HouseholdMember, HouseholdMemberPatch, MAX_USERNAME_LEN, MIN_USERNAME_LEN, MemberAccessGrant,
    NewHouseholdMember, NewUser, User, UserPatch,
};
pub use household_settings::{
    HouseholdSettings, HouseholdSettingsPatch, MealTimes, MissingStockInterpretation,
    UnknownMissingStockInterpretation,
};
pub use ids::{ConsumptionRecordId, HouseholdMemberId, IngredientId, ProductId, Revision, UserId};
pub use ids::{
    MealGuestAllocationId, MealGuestGroupId, MealParticipantAllocationId, MealParticipantId,
    MealPlanComponentId, MealPlanEntryId, NutritionTargetId, PurchaseId, RecipeComponentId,
    RecipeId, RecipeInstructionId, ShoppingOpportunityId, StockEffectId, StockEventId, StockItemId,
    WeightGoalId, WeightRecordId,
};
pub use ingredient::{
    Ingredient, IngredientPatch, IngredientSummary, MAX_NAME_LEN, NewIngredient, validate_name,
};
pub use meal_item::{MealItemRef, UnknownMealItemRef};
pub use meal_plan::{
    ActualMealPlanComponent, AllocationOutcome, Assumption, AssumptionRules, ChangedMealOutcome,
    ComponentPreparation, ConfirmMealPlanComponent, ConfirmMealPlanEntry, MealGuestAllocation,
    MealGuestGroup, MealOptOut, MealParticipant, MealParticipantAllocation, MealPlanComponent,
    MealPlanComponentSnapshot, MealPlanEntry, MealPlanEntryPatch, MealPlanScope, MealPlanStatus,
    MealSlot, NewMealGuestAllocation, NewMealGuestGroup, NewMealParticipant,
    NewMealParticipantAllocation, NewMealPlanComponent, NewMealPlanEntry, OutcomeActor,
    ParticipantStatus, Portioning, ReplacementItem, ReviewMealOutcomes, ReviewedGuestOutcome,
    ReviewedMealOutcome, ReviewedMemberOutcome, SetMealParticipants, SlotAttendance,
    UnknownMealPlanScope, UnknownMealPlanStatus, UnknownMealSlot, UnknownParticipantStatus,
    UnknownPortioning, actual_components_for_member, allocated_total, apply_equal_portioning,
    build_guest_results, build_participant, component_still_eaten, derive_component_status,
    derive_entry_status, derive_guest_status, derive_participant_status, effective_consumption,
    equal_split, find_component, has_explicit_allocations, make_components, merge_components,
    merge_guest_group, merge_participant, outcomes_for_component, participant_status_to_meal,
    pending_component_ids, preparation_for, replacements_for, require_allocation_planned,
    require_editable, require_household_attendance, require_planned, require_subject_pending,
    set_allocation, sync_allocations, validate_actual_components, validate_components,
    validate_guest_groups, validate_participants,
};
pub(crate) use meal_plan::{MEAL_PLAN_COMPONENT, MEAL_PLAN_ENTRY};
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
pub use recipe::{
    DerivedNutrition, Fulfilment, MAX_REQUIREMENT_TEXT_LEN, MAX_SERVINGS, MealCategory, NewRecipe,
    NewRecipeComponent, NewRecipeInstruction, Recipe, RecipeComponent, RecipeInstruction,
    RecipePatch, RecipePhoto, RecipePhotoDerivatives, RecipeRequirement, RecipeSummary,
    RecipeVisibility, UnknownMealCategory, UnknownRecipeVisibility, normalise_countries,
    normalise_optional_text, normalise_tags, normalise_unique, recipe_nutrition,
    recipe_nutrition_detailed,
};
pub use shopping::{
    Assignment, Certainty, ExceptionState, NewPurchase, NewShoppingCadence, OpportunityException,
    OpportunityState, Purchase, PurchasePatch, PurchaseState, ShoppingCadence, ShoppingOpportunity,
    ShoppingRequirement, ShoppingSection, SuggestionReason, UnknownExceptionState,
    UnknownOpportunityState, UnknownPurchaseState, UnknownShoppingSection, assign,
    expand_opportunities, week_day_from_number, week_day_number,
};
pub use stock::{
    AppliedDelta, Availability, AvailabilityReport, Confidence, DeductionPlan, DeductionTarget,
    DemandClaim, DemandGap, DemandSubject, IngredientAvailability, MissingStock, NewStockEffect,
    NewStockEvent, NewStockItem, PlannedTake, ProductAvailability, ReleasePlan, Shortfall,
    SourceDate, SourceDateKind, StockEffect, StockEffectSource, StockEffectState, StockEvent,
    StockEventKind, StockEventSource, StockItem, StockItemPatch, StockLevel, StockOutcome,
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
