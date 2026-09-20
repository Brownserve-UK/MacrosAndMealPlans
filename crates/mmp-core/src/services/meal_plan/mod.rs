use std::collections::HashSet;
use std::sync::Arc;

use time::{Date, Duration};

use crate::domain::{
    AssumptionRules, ConsumedAmount, ConsumptionRecord, HouseholdMemberId, MEAL_OCCASION,
    MEAL_PLAN_ENTRY, MealItemRef, MealOccasion, MealOccasionId, MealPlanEntry, MealPlanEntryId,
    NewMealPlanComponent, RecipeVisibility, UserId, diners_for, materialise_participants,
};
use crate::error::{CoreError, Result, ValidationErrors};
use crate::ports::{
    Clock, ConsumptionRecordRepository, HouseholdMemberRepository, HouseholdSettingsRepository,
    IngredientRepository, MealPlanRepository, MemberQuery, NutritionTargetRepository, PageRequest,
    PreparedBatchRepository, PreparedMealRepository, ProductRepository, RecipeRepository,
};
use crate::services::PreparationService;

mod catalogue;
mod outcomes;
mod planning;
pub use planning::{GuestChange, GuestMealTarget};
mod view;

pub use view::{
    CookingItemKind, CookingItemView, MealDiner, MealGroupView, MealItem, MealItemSource,
    MealOccasionView, MealParticipantView, MealPlanComponentView, MealPlanDay, MealPlanEntryView,
    MealPlanWeek, MealSlotView, NeedsReview, NutritionSummary, PlannerDay, PlannerMember,
    PlannerWeek,
};

const PRODUCT: &str = "product";
const RECIPE: &str = "recipe";
const DISH: &str = "cooked food";

#[derive(Clone)]
pub struct MealPlanService {
    plans: Arc<dyn MealPlanRepository>,
    products: Arc<dyn ProductRepository>,
    ingredients: Arc<dyn IngredientRepository>,
    prepared_meals: Arc<dyn PreparedMealRepository>,
    recipes: Arc<dyn RecipeRepository>,
    consumption: Arc<dyn ConsumptionRecordRepository>,
    targets: Arc<dyn NutritionTargetRepository>,
    members: Arc<dyn HouseholdMemberRepository>,
    settings: Arc<dyn HouseholdSettingsRepository>,
    batches: Arc<dyn PreparedBatchRepository>,
    preparation: PreparationService,
    stock: crate::services::StockService,
    clock: Arc<dyn Clock>,
}

impl MealPlanService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        plans: Arc<dyn MealPlanRepository>,
        products: Arc<dyn ProductRepository>,
        ingredients: Arc<dyn IngredientRepository>,
        prepared_meals: Arc<dyn PreparedMealRepository>,
        recipes: Arc<dyn RecipeRepository>,
        consumption: Arc<dyn ConsumptionRecordRepository>,
        targets: Arc<dyn NutritionTargetRepository>,
        members: Arc<dyn HouseholdMemberRepository>,
        settings: Arc<dyn HouseholdSettingsRepository>,
        batches: Arc<dyn PreparedBatchRepository>,
        preparation: PreparationService,
        stock: crate::services::StockService,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            plans,
            products,
            ingredients,
            prepared_meals,
            recipes,
            consumption,
            targets,
            members,
            settings,
            batches,
            preparation,
            stock,
            clock,
        }
    }

    pub async fn get(&self, id: MealPlanEntryId) -> Result<MealPlanEntryView> {
        let entry = self.get_entry(id).await?;
        let records = self.records_for_entry(entry.id).await?;
        self.present(entry, &records, None).await
    }

    pub async fn get_occasion(&self, id: MealOccasionId) -> Result<MealOccasionView> {
        let occasion = self.load_occasion(id).await?;
        let rules = self.assumption_rules().await?;
        self.present_occasion(&rules, occasion).await
    }

    async fn get_entry(&self, id: MealPlanEntryId) -> Result<MealPlanEntry> {
        let stored = self
            .plans
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(MEAL_PLAN_ENTRY, id))?;
        let occasion = self.load_occasion(stored.occasion_id).await?;
        occasion
            .groups
            .into_iter()
            .find(|group| group.id == id)
            .ok_or_else(|| CoreError::not_found(MEAL_PLAN_ENTRY, id))
    }

    async fn load_occasion(&self, id: MealOccasionId) -> Result<MealOccasion> {
        let mut occasion = self
            .plans
            .get_occasion(id)
            .await?
            .ok_or_else(|| CoreError::not_found(MEAL_OCCASION, id))?;
        let active = self.active_member_ids().await?;
        self.materialise_occasion(&mut occasion, &active);
        Ok(occasion)
    }

    fn materialise_occasion(&self, occasion: &mut MealOccasion, active: &[HouseholdMemberId]) {
        let now = self.clock.now();
        let diners: Vec<Vec<HouseholdMemberId>> = occasion
            .groups
            .iter()
            .map(|group| diners_for(group, occasion, active))
            .collect();
        for (group, diners) in occasion.groups.iter_mut().zip(diners) {
            materialise_participants(group, &diners, now);
        }
    }

    async fn active_member_ids(&self) -> Result<Vec<HouseholdMemberId>> {
        let page = self
            .members
            .list(&MemberQuery {
                include_archived: false,
                page: PageRequest::new(1, PageRequest::MAX_PER_PAGE),
                ..Default::default()
            })
            .await?;
        Ok(page
            .items
            .into_iter()
            .filter(|member| !member.is_archived())
            .map(|member| member.id)
            .collect())
    }

    fn resolve_subject(
        &self,
        entry: &MealPlanEntry,
        requested: Option<HouseholdMemberId>,
    ) -> Result<HouseholdMemberId> {
        requested
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
                MealItemRef::Dish { recipe_id } => {
                    if self.batches.held_for_recipe(recipe_id).await?.is_empty() {
                        return Err(CoreError::not_found(DISH, recipe_id));
                    }
                    if !matches!(component.amount, ConsumedAmount::Servings(_)) {
                        errors.push(
                            format!("components.{index}.amount"),
                            "Cooked food is measured in servings",
                        );
                    }
                }
                MealItemRef::Ingredient { ingredient_id } => {
                    self.ingredients
                        .get(ingredient_id)
                        .await?
                        .ok_or_else(|| CoreError::not_found("ingredient", ingredient_id))?;
                    if !matches!(component.amount, ConsumedAmount::Measure(_)) {
                        errors.push(
                            format!("components.{index}.amount"),
                            "A food without a brand is measured, not counted",
                        );
                    }
                }
                MealItemRef::PreparedMeal { prepared_meal_id } => {
                    self.prepared_meals
                        .get(prepared_meal_id)
                        .await?
                        .ok_or_else(|| CoreError::not_found("prepared_meal", prepared_meal_id))?;
                    if !matches!(component.amount, ConsumedAmount::Measure(_)) {
                        errors.push(
                            format!("components.{index}.amount"),
                            "A food without a brand is measured, not counted",
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
        let calendar = crate::ports::HouseholdCalendar::new(self.clock.clone(), &settings.timezone);
        Ok(AssumptionRules {
            now: calendar.now(),
            meal_times: settings.meal_times,
            enabled: settings.assume_eaten_when_time_passes,
        })
    }

    async fn records_for_entry(&self, entry_id: MealPlanEntryId) -> Result<Vec<ConsumptionRecord>> {
        self.consumption.list_for_meal_plan_entry(entry_id).await
    }
}

const PLANNING_GRACE_DAYS: i64 = 1;

async fn ensure_not_past(
    clock: &Arc<dyn Clock>,
    settings: &dyn HouseholdSettingsRepository,
    planned_on: Date,
) -> Result<()> {
    let today = super::calendar::household_calendar(settings, clock)
        .await?
        .today();
    let earliest = today - Duration::days(PLANNING_GRACE_DAYS);
    if planned_on < earliest {
        let mut errors = ValidationErrors::new();
        errors.push("planned_on", "Plans cannot be dated in the past");
        return errors.into_result();
    }
    Ok(())
}

async fn ensure_due(
    clock: &Arc<dyn Clock>,
    settings: &dyn HouseholdSettingsRepository,
    planned_on: Date,
) -> Result<()> {
    let today = super::calendar::household_calendar(settings, clock)
        .await?
        .today();
    let latest = today + Duration::days(PLANNING_GRACE_DAYS);
    if planned_on > latest {
        return Err(CoreError::conflict("This meal is not due yet."));
    }
    Ok(())
}

#[cfg(test)]
#[path = "meal_plan_tests.rs"]
mod tests;
