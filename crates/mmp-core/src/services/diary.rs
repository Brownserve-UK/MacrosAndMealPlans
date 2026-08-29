use std::sync::Arc;

use time::{Date, Duration};

use crate::domain::{
    ConsumedAmount, ConsumedNutrition, ConsumptionRecord, ConsumptionRecordId,
    ConsumptionRecordPatch, HouseholdMemberId, MealItemRef, NewConsumptionRecord, NutritionFacts,
    NutritionQuality, Product, ProductId, Recipe, RecipeId, Revision, nutrition_for,
    recipe_nutrition, recipe_nutrition_for, sum_nutrition,
};
use crate::error::{CoreError, Result, ValidationErrors};
use crate::ports::{
    Clock, ConsumptionQuery, ConsumptionRecordRepository, PageRequest, ProductRepository,
    RecipeRepository, UpdateOutcome,
};

use super::fulfilment::RecipeFulfilments;

const CONSUMPTION_RECORD: &str = "consumption record";
const PRODUCT: &str = "product";
const RECIPE: &str = "recipe";

#[derive(Debug, Clone)]
pub struct DayTotals {
    pub nutrition: NutritionFacts,
    pub entry_count: i64,
    pub unknown_count: i64,
    pub partial_count: i64,
}

#[derive(Debug, Clone)]
pub struct DiaryEntry {
    pub record: ConsumptionRecord,
    pub product_name: String,
}

#[derive(Debug, Clone)]
pub struct DiaryDay {
    pub member_id: HouseholdMemberId,
    pub date: Date,
    pub entries: Vec<DiaryEntry>,
    pub totals: DayTotals,
}

#[derive(Clone)]
pub struct DiaryService {
    records: Arc<dyn ConsumptionRecordRepository>,
    products: Arc<dyn ProductRepository>,
    recipes: Arc<dyn RecipeRepository>,
    clock: Arc<dyn Clock>,
}

impl DiaryService {
    pub fn new(
        records: Arc<dyn ConsumptionRecordRepository>,
        products: Arc<dyn ProductRepository>,
        recipes: Arc<dyn RecipeRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            records,
            products,
            recipes,
            clock,
        }
    }

    pub async fn record(&self, input: NewConsumptionRecord) -> Result<ConsumptionRecord> {
        ensure_not_future(&*self.clock, input.consumed_on)?;
        self.record_unchecked(input).await
    }

    pub async fn record_unchecked(&self, input: NewConsumptionRecord) -> Result<ConsumptionRecord> {
        input.validate()?;
        let scaled = self
            .resolve_item(input.item, &input.amount, input.recorded_by)
            .await?;
        let now = self.clock.now();
        let record = ConsumptionRecord {
            id: input.id.unwrap_or_default(),
            member_id: input.member_id,
            item: input.item,
            recorded_by: input.recorded_by,
            meal_plan_entry_id: input.meal_plan_entry_id,
            meal_plan_component_id: input.meal_plan_component_id,
            slot: input.slot,
            amount: input.amount,
            consumed_on: input.consumed_on,
            consumed_at: input.consumed_at,
            nutrition: scaled.facts,
            quality: scaled.quality,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
        };

        self.records.insert(&record).await?;
        Ok(record)
    }

    pub async fn get(&self, id: ConsumptionRecordId) -> Result<ConsumptionRecord> {
        self.records
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(CONSUMPTION_RECORD, id))
    }

    pub async fn amend(
        &self,
        id: ConsumptionRecordId,
        expected: Revision,
        patch: ConsumptionRecordPatch,
    ) -> Result<ConsumptionRecord> {
        patch.validate()?;
        let mut current = self.get(id).await?;
        require_revision(id, expected, current.revision)?;

        if patch.is_empty() {
            return Ok(current);
        }

        if patch.slot.is_some() && current.meal_plan_component_id.is_some() {
            return Err(CoreError::conflict(
                "This food came from a planned meal. Reopen the meal in your plan to move it.",
            ));
        }

        if let Some(consumed_on) = patch.consumed_on {
            ensure_not_future(&*self.clock, consumed_on)?;
            current.consumed_on = consumed_on;
        }
        if let Some(consumed_at) = patch.consumed_at {
            current.consumed_at = consumed_at;
        }
        if let Some(slot) = patch.slot {
            current.slot = slot;
        }
        if let Some(amount) = patch.amount {
            let scaled = self
                .resolve_item(current.item, &amount, current.recorded_by)
                .await?;
            current.amount = amount;
            current.nutrition = scaled.facts;
            current.quality = scaled.quality;
        }

        current.revision = current.revision.next();
        current.updated_at = self.clock.now();
        self.commit(&current, expected).await?;
        Ok(current)
    }

    pub async fn remove(&self, id: ConsumptionRecordId, expected: Revision) -> Result<()> {
        let current = self.get(id).await?;
        require_revision(id, expected, current.revision)?;
        if current.meal_plan_component_id.is_some() {
            return Err(CoreError::conflict(
                "This food came from a planned meal. Reopen the meal in your plan to remove it.",
            ));
        }
        if self.records.delete(id).await? {
            Ok(())
        } else {
            Err(CoreError::not_found(CONSUMPTION_RECORD, id))
        }
    }

    pub async fn day(&self, member_id: HouseholdMemberId, date: Date) -> Result<DiaryDay> {
        let query = ConsumptionQuery {
            member_id: Some(member_id),
            from: Some(date),
            to: Some(date),
            page: PageRequest::new(1, PageRequest::MAX_PER_PAGE),
            sort: Default::default(),
        };
        let page = self.records.list(&query).await?;
        let totals = totals_for(&page.items);

        let mut entries = Vec::with_capacity(page.items.len());
        for record in page.items {
            let product_name = self.item_name(record.item).await?;
            entries.push(DiaryEntry {
                record,
                product_name,
            });
        }

        Ok(DiaryDay {
            member_id,
            date,
            entries,
            totals,
        })
    }

    async fn get_product(&self, id: ProductId) -> Result<Product> {
        self.products
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(PRODUCT, id))
    }

    async fn get_recipe(
        &self,
        id: RecipeId,
        actor: Option<crate::domain::UserId>,
    ) -> Result<Recipe> {
        self.recipes
            .get(id)
            .await?
            .filter(|recipe| actor.is_none_or(|actor| recipe.owner_id == actor))
            .ok_or_else(|| CoreError::not_found(RECIPE, id))
    }

    async fn item_name(&self, item: MealItemRef) -> Result<String> {
        match item {
            MealItemRef::Product { product_id } => match self.products.get(product_id).await? {
                Some(product) => Ok(product.name),
                None => Ok("Missing product".to_owned()),
            },
            MealItemRef::Recipe { recipe_id } => match self.recipes.get(recipe_id).await? {
                Some(recipe) => Ok(recipe.name),
                None => Ok("Missing recipe".to_owned()),
            },
        }
    }

    async fn resolve_item(
        &self,
        item: MealItemRef,
        amount: &ConsumedAmount,
        actor: Option<crate::domain::UserId>,
    ) -> Result<ConsumedNutrition> {
        match item {
            MealItemRef::Product { product_id } => {
                let product = self.get_product(product_id).await?;
                ensure_loggable(&product)?;
                ensure_resolvable(&product, amount)?;
                Ok(nutrition_for(&product, amount))
            }
            MealItemRef::Recipe { recipe_id } => {
                let recipe = self.get_recipe(recipe_id, actor).await?;
                if recipe.is_archived() {
                    let mut errors = ValidationErrors::new();
                    errors.push("item", "That recipe is archived");
                    return Err(errors.into());
                }
                if !matches!(amount, ConsumedAmount::Servings(_)) {
                    let mut errors = ValidationErrors::new();
                    errors.push("amount", "Recipes are measured in servings");
                    return Err(errors.into());
                }
                let requirements: Vec<&crate::domain::RecipeRequirement> = recipe
                    .components
                    .iter()
                    .map(|component| &component.requirement)
                    .collect();
                let fulfilments = RecipeFulfilments::load(&*self.products, &requirements).await?;
                let per_serving = recipe_nutrition(
                    recipe.components.iter().map(|component| {
                        (&component.amount, fulfilments.get(&component.requirement))
                    }),
                    recipe.servings,
                );
                Ok(recipe_nutrition_for(&per_serving, amount))
            }
        }
    }

    async fn commit(&self, record: &ConsumptionRecord, expected: Revision) -> Result<()> {
        match self.records.update(record, expected).await? {
            UpdateOutcome::Updated => Ok(()),
            UpdateOutcome::RevisionMismatch { actual } => Err(CoreError::RevisionMismatch {
                resource: CONSUMPTION_RECORD,
                id: record.id.to_string(),
                expected,
                actual,
            }),
            UpdateOutcome::NotFound => Err(CoreError::not_found(CONSUMPTION_RECORD, record.id)),
        }
    }
}

fn ensure_loggable(product: &Product) -> Result<()> {
    if product.is_archived() {
        let mut errors = ValidationErrors::new();
        errors.push("product_id", "That product is archived");
        return errors.into_result();
    }
    Ok(())
}

fn ensure_resolvable(product: &Product, amount: &ConsumedAmount) -> Result<()> {
    if let Err(err) = amount.resolve(product) {
        let mut errors = ValidationErrors::new();
        errors.push("amount", err.to_string());
        return errors.into_result();
    }
    Ok(())
}

fn ensure_not_future(clock: &dyn Clock, consumed_on: Date) -> Result<()> {
    let latest = clock.now().date() + Duration::days(1);
    if consumed_on > latest {
        let mut errors = ValidationErrors::new();
        errors.push("consumed_on", "Food cannot be logged in the future");
        return errors.into_result();
    }
    Ok(())
}

fn totals_for(entries: &[ConsumptionRecord]) -> DayTotals {
    let nutrition = sum_nutrition(entries.iter().map(|e| &e.nutrition));
    let unknown_count = entries
        .iter()
        .filter(|e| e.quality == NutritionQuality::Unknown)
        .count() as i64;
    let partial_count = entries
        .iter()
        .filter(|e| e.quality == NutritionQuality::Partial)
        .count() as i64;
    DayTotals {
        nutrition,
        entry_count: entries.len() as i64,
        unknown_count,
        partial_count,
    }
}

fn require_revision(
    id: impl std::fmt::Display,
    expected: Revision,
    actual: Revision,
) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(CoreError::RevisionMismatch {
            resource: CONSUMPTION_RECORD,
            id: id.to_string(),
            expected,
            actual,
        })
    }
}

#[cfg(test)]
#[path = "diary_tests.rs"]
mod tests;
