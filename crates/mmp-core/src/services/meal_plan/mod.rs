use std::collections::HashSet;
use std::sync::Arc;

use time::{Date, Duration};

use crate::domain::{
    AssumptionRules, ConsumptionRecord, HouseholdMemberId, MEAL_PLAN_COMPONENT, MEAL_PLAN_ENTRY,
    MealItemRef, MealPlanComponentId, MealPlanEntry, MealPlanEntryId, NewMealPlanComponent,
    RecipeVisibility, Revision, UserId, require_editable,
};
use crate::error::{CoreError, Result, ValidationErrors};
use crate::ports::{
    Clock, ConsumptionRecordRepository, HouseholdMemberRepository, HouseholdSettingsRepository,
    IngredientRepository, MealPlanRepository, NutritionTargetRepository, ProductRepository,
    RecipeRepository, UpdateOutcome,
};

mod catalogue;
mod outcomes;
mod planning;
mod view;

pub use view::{
    MealItem, MealItemSource, MealParticipantView, MealPlanComponentView, MealPlanDay,
    MealPlanEntryView, MealPlanWeek, MealSlotView, NeedsReview, NutritionSummary,
};

const PRODUCT: &str = "product";
const RECIPE: &str = "recipe";

#[derive(Clone)]
pub struct MealPlanService {
    plans: Arc<dyn MealPlanRepository>,
    products: Arc<dyn ProductRepository>,
    ingredients: Arc<dyn IngredientRepository>,
    recipes: Arc<dyn RecipeRepository>,
    consumption: Arc<dyn ConsumptionRecordRepository>,
    targets: Arc<dyn NutritionTargetRepository>,
    members: Arc<dyn HouseholdMemberRepository>,
    settings: Arc<dyn HouseholdSettingsRepository>,
    clock: Arc<dyn Clock>,
}

impl MealPlanService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        plans: Arc<dyn MealPlanRepository>,
        products: Arc<dyn ProductRepository>,
        ingredients: Arc<dyn IngredientRepository>,
        recipes: Arc<dyn RecipeRepository>,
        consumption: Arc<dyn ConsumptionRecordRepository>,
        targets: Arc<dyn NutritionTargetRepository>,
        members: Arc<dyn HouseholdMemberRepository>,
        settings: Arc<dyn HouseholdSettingsRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            plans,
            products,
            ingredients,
            recipes,
            consumption,
            targets,
            members,
            settings,
            clock,
        }
    }

    pub async fn get(&self, id: MealPlanEntryId) -> Result<MealPlanEntryView> {
        let entry = self.get_entry(id).await?;
        let records = self.records_for_entry(entry.id).await?;
        self.present(entry, &records, None).await
    }

    async fn get_entry(&self, id: MealPlanEntryId) -> Result<MealPlanEntry> {
        self.plans
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(MEAL_PLAN_ENTRY, id))
    }

    fn resolve_subject(
        &self,
        entry: &MealPlanEntry,
        requested: Option<HouseholdMemberId>,
    ) -> Result<HouseholdMemberId> {
        requested
            .or(entry.member_id)
            .or_else(|| entry.participants.first().map(|p| p.member_id))
            .ok_or_else(|| {
                CoreError::conflict("This meal has no participant to record an outcome against.")
            })
    }

    async fn validate_component_items(
        &self,
        components: &[NewMealPlanComponent],
        archived_allowed: &HashSet<MealItemRef>,
        actor_id: UserId,
    ) -> Result<()> {
        for (index, component) in components.iter().enumerate() {
            let mut errors = ValidationErrors::new();
            match component.item {
                MealItemRef::Product { product_id } => {
                    let product = self
                        .products
                        .get(product_id)
                        .await?
                        .ok_or_else(|| CoreError::not_found(PRODUCT, product_id))?;
                    if product.is_archived() && !archived_allowed.contains(&component.item) {
                        errors.push(
                            format!("components.{index}.item"),
                            "That product is archived",
                        );
                    }
                    if let Err(error) = component.amount.resolve(&product) {
                        errors.push(format!("components.{index}.amount"), error.to_string());
                    }
                }
                MealItemRef::Recipe { recipe_id } => {
                    let recipe = self
                        .recipes
                        .get(recipe_id)
                        .await?
                        .filter(|recipe| {
                            recipe.owner_id == actor_id
                                || recipe.visibility == RecipeVisibility::Shared
                        })
                        .ok_or_else(|| CoreError::not_found(RECIPE, recipe_id))?;
                    if recipe.is_archived() && !archived_allowed.contains(&component.item) {
                        errors.push(
                            format!("components.{index}.item"),
                            "That recipe is archived",
                        );
                    }
                }
            }
            errors.into_result()?;
        }
        Ok(())
    }

    async fn assumption_rules(&self) -> Result<AssumptionRules> {
        let settings = self.settings.get().await?;
        Ok(AssumptionRules {
            now: self.clock.now(),
            meal_times: settings.meal_times,
            enabled: settings.assume_eaten_when_time_passes,
        })
    }

    async fn records_for_entry(&self, entry_id: MealPlanEntryId) -> Result<Vec<ConsumptionRecord>> {
        self.consumption.list_for_meal_plan_entry(entry_id).await
    }
}

fn ensure_not_past(clock: &dyn Clock, planned_on: Date) -> Result<()> {
    let earliest = clock.now().date() - Duration::days(1);
    if planned_on < earliest {
        let mut errors = ValidationErrors::new();
        errors.push("planned_on", "Plans cannot be dated in the past");
        return errors.into_result();
    }
    Ok(())
}

fn ensure_due(clock: &dyn Clock, planned_on: Date) -> Result<()> {
    let latest = clock.now().date() + Duration::days(1);
    if planned_on > latest {
        return Err(CoreError::conflict("This meal is not due yet."));
    }
    Ok(())
}

fn require_revision(id: MealPlanEntryId, expected: Revision, actual: Revision) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(CoreError::RevisionMismatch {
            resource: MEAL_PLAN_ENTRY,
            id: id.to_string(),
            expected,
            actual,
        })
    }
}

fn require_component_revision(
    id: MealPlanComponentId,
    expected: Revision,
    actual: Revision,
) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(CoreError::RevisionMismatch {
            resource: MEAL_PLAN_COMPONENT,
            id: id.to_string(),
            expected,
            actual,
        })
    }
}

fn commit_component_outcome(
    outcome: UpdateOutcome,
    id: MealPlanComponentId,
    expected: Revision,
) -> Result<()> {
    match outcome {
        UpdateOutcome::Updated => Ok(()),
        UpdateOutcome::RevisionMismatch { actual } => Err(CoreError::RevisionMismatch {
            resource: MEAL_PLAN_COMPONENT,
            id: id.to_string(),
            expected,
            actual,
        }),
        UpdateOutcome::NotFound => Err(CoreError::not_found(MEAL_PLAN_COMPONENT, id)),
    }
}

fn commit_outcome(outcome: UpdateOutcome, id: MealPlanEntryId, expected: Revision) -> Result<()> {
    match outcome {
        UpdateOutcome::Updated => Ok(()),
        UpdateOutcome::RevisionMismatch { actual } => Err(CoreError::RevisionMismatch {
            resource: MEAL_PLAN_ENTRY,
            id: id.to_string(),
            expected,
            actual,
        }),
        UpdateOutcome::NotFound => Err(CoreError::not_found(MEAL_PLAN_ENTRY, id)),
    }
}

#[cfg(test)]
#[path = "meal_plan_tests.rs"]
mod tests;
