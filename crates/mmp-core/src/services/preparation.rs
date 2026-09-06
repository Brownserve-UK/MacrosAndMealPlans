use std::sync::Arc;

use rust_decimal::Decimal;
use time::OffsetDateTime;

use super::fulfilment::{RecipeFulfilments, expand_recipe};
use super::stock_effects::{StockAffected, name_outcomes, requirement_deduction};
use crate::domain::{
    ConsumedNutrition, MealPlanComponentId, NewPreparedBatch, NewStockEvent, PortionPlacement,
    PreparationSource, PreparedBatch, PreparedBatchId, Quantity, Recipe, RecipeId,
    RecipeRequirement, Revision, StockEffectSource, StockEventKind, StockEventSource, StockItem,
    StockItemId, StockLevel, StockSubject, Unit, UserId, recipe_nutrition, validate_placements,
};
use crate::error::{CoreError, Result};
use crate::ports::{
    Clock, IngredientRepository, PreparedBatchRepository, ProductRepository, RecipeRepository,
    StockDeduction, StockWrite,
};

const PREPARED_BATCH: &str = "prepared batch";

#[derive(Debug, Clone)]
pub struct RecordPreparation {
    pub recipe_id: RecipeId,
    pub source: PreparationSource,
    pub servings_produced: Decimal,
    pub placements: Vec<PortionPlacement>,
    pub prepared_at: Option<OffsetDateTime>,
    pub actor: UserId,
}

#[derive(Clone)]
pub struct PreparationService {
    batches: Arc<dyn PreparedBatchRepository>,
    recipes: Arc<dyn RecipeRepository>,
    products: Arc<dyn ProductRepository>,
    ingredients: Arc<dyn IngredientRepository>,
    clock: Arc<dyn Clock>,
}

impl PreparationService {
    pub fn new(
        batches: Arc<dyn PreparedBatchRepository>,
        recipes: Arc<dyn RecipeRepository>,
        products: Arc<dyn ProductRepository>,
        ingredients: Arc<dyn IngredientRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            batches,
            recipes,
            products,
            ingredients,
            clock,
        }
    }

    pub async fn get(&self, id: PreparedBatchId) -> Result<PreparedBatch> {
        self.batches
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(PREPARED_BATCH, id))
    }

    pub async fn names_for(
        &self,
        ids: &[PreparedBatchId],
    ) -> Result<std::collections::HashMap<PreparedBatchId, String>> {
        Ok(self
            .batches
            .get_many(ids)
            .await?
            .into_iter()
            .map(|batch| (batch.id, batch.item_name))
            .collect())
    }

    pub async fn for_component(
        &self,
        component_id: MealPlanComponentId,
    ) -> Result<Option<PreparedBatch>> {
        self.batches.for_component(component_id).await
    }

    pub async fn list_in_range(
        &self,
        from: time::Date,
        to: time::Date,
    ) -> Result<Vec<PreparedBatch>> {
        if to < from {
            return Err(CoreError::conflict("That date range runs backwards."));
        }
        self.batches.list_in_range(from, to).await
    }

    pub async fn place(
        &self,
        batch_id: PreparedBatchId,
        placements: Vec<PortionPlacement>,
        actor: UserId,
    ) -> Result<StockAffected<PreparedBatch>> {
        let batch = self.get(batch_id).await?;
        let held = self.batches.portions(batch_id).await?;
        let remaining: Decimal = held
            .iter()
            .filter_map(|item| item.level.conservative_quantity())
            .map(|quantity| quantity.amount)
            .sum();
        validate_placements(&placements, remaining)?;

        let now = self.clock.now();
        let mut portions = Vec::with_capacity(placements.len());
        for (index, placement) in placements.iter().enumerate() {
            let level = StockLevel::Exact {
                quantity: Quantity::new(placement.servings, Unit::Serving),
            };
            let existing = held.get(index);
            let item = StockItem {
                id: existing.map(|item| item.id).unwrap_or_default(),
                subject: StockSubject::prepared_portion(batch_id),
                level,
                storage_location: placement.storage_location,
                source_date: existing.and_then(|item| item.source_date),
                usability_deadline: placement.usability_deadline.clone(),
                note: placement.note.clone(),
                revision: existing
                    .map(|item| item.revision)
                    .unwrap_or(Revision::INITIAL),
                created_at: existing.map(|item| item.created_at).unwrap_or(now),
                updated_at: now,
                archived_at: None,
            };
            let event = NewStockEvent {
                kind: StockEventKind::Moved,
                quantity_delta: None,
                actor_user_id: Some(actor),
                subject_member_id: None,
                source: None,
                reverses_event_id: None,
                note: None,
            };
            portions.push((item, event));
        }

        let archive: Vec<_> = held
            .iter()
            .skip(placements.len())
            .map(|item| item.id)
            .collect();
        let outcomes = self.batches.place_portions(&portions, &archive).await?;
        let named = name_outcomes(
            &*self.products,
            &*self.ingredients,
            &*self.batches,
            outcomes,
        )
        .await?;
        Ok(StockAffected::new(batch, named))
    }

    pub async fn record(&self, input: RecordPreparation) -> Result<StockAffected<PreparedBatch>> {
        let recipe = self
            .recipes
            .get(input.recipe_id)
            .await?
            .ok_or_else(|| CoreError::not_found("recipe", input.recipe_id))?;
        if recipe.is_archived() {
            return Err(CoreError::conflict("That recipe is archived."));
        }

        let (batch, portions, write) = self.plan(&recipe, &input).await?;
        let outcomes = self.batches.insert(&batch, &portions, &write).await?;
        let named = name_outcomes(
            &*self.products,
            &*self.ingredients,
            &*self.batches,
            outcomes,
        )
        .await?;
        Ok(StockAffected::new(batch, named))
    }

    async fn plan(
        &self,
        recipe: &Recipe,
        input: &RecordPreparation,
    ) -> Result<(PreparedBatch, Vec<(StockItem, NewStockEvent)>, StockWrite)> {
        let servings = input.servings_produced;
        validate_placements(&input.placements, servings)?;
        let new_batch = NewPreparedBatch {
            recipe_id: Some(recipe.id),
            source: input.source,
            prepared_at: input.prepared_at.unwrap_or_else(|| self.clock.now()),
            servings_produced: servings,
            item_name: recipe.name.clone(),
            nutrition: self.per_serving(recipe).await?,
        };
        new_batch.validate()?;

        let now = self.clock.now();
        let batch = PreparedBatch {
            id: PreparedBatchId::new(),
            recipe_id: new_batch.recipe_id,
            source: new_batch.source,
            prepared_at: new_batch.prepared_at,
            servings_produced: new_batch.servings_produced,
            item_name: new_batch.item_name,
            nutrition: new_batch.nutrition,
            created_by: input.actor,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
        };

        let portions = input
            .placements
            .iter()
            .map(|placement| {
                let level = StockLevel::Exact {
                    quantity: Quantity::new(placement.servings, Unit::Serving),
                };
                let item = StockItem {
                    id: StockItemId::new(),
                    subject: StockSubject::prepared_portion(batch.id),
                    level,
                    storage_location: placement.storage_location,
                    source_date: None,
                    usability_deadline: placement.usability_deadline.clone(),
                    note: placement.note.clone(),
                    revision: Revision::INITIAL,
                    created_at: now,
                    updated_at: now,
                    archived_at: None,
                };
                let event = NewStockEvent {
                    kind: StockEventKind::Added,
                    quantity_delta: level.conservative_quantity(),
                    actor_user_id: Some(input.actor),
                    subject_member_id: None,
                    source: Some(StockEventSource {
                        kind: StockEffectSource::PreparedBatch,
                        id: batch.id.as_uuid(),
                        label: batch.item_name.clone(),
                    }),
                    reverses_event_id: None,
                    note: None,
                };
                (item, event)
            })
            .collect();

        let write = StockWrite {
            deductions: self.raw_deductions(recipe, &batch, servings, input).await?,
            releases: Vec::new(),
        };
        Ok((batch, portions, write))
    }

    async fn raw_deductions(
        &self,
        recipe: &Recipe,
        batch: &PreparedBatch,
        servings: Decimal,
        input: &RecordPreparation,
    ) -> Result<Vec<StockDeduction>> {
        let requirements: Vec<&RecipeRequirement> = recipe
            .components
            .iter()
            .map(|component| &component.requirement)
            .collect();
        let fulfilments = RecipeFulfilments::load(&*self.products, &requirements).await?;
        Ok(expand_recipe(recipe, servings, &fulfilments)
            .wants
            .into_iter()
            .map(|want| {
                requirement_deduction(
                    StockEffectSource::PreparedBatch,
                    batch.id.as_uuid(),
                    want.recipe_component_id.as_uuid(),
                    want.target,
                    want.want,
                    format!("Cooked {}", batch.item_name),
                    Some(input.actor),
                    None,
                )
            })
            .collect())
    }

    async fn per_serving(&self, recipe: &Recipe) -> Result<ConsumedNutrition> {
        let requirements: Vec<&RecipeRequirement> = recipe
            .components
            .iter()
            .map(|component| &component.requirement)
            .collect();
        let fulfilments = RecipeFulfilments::load(&*self.products, &requirements).await?;
        Ok(recipe_nutrition(
            recipe
                .components
                .iter()
                .map(|component| (&component.amount, fulfilments.get(&component.requirement))),
            recipe.servings,
        ))
    }
}

#[cfg(test)]
#[path = "preparation_tests.rs"]
mod tests;
