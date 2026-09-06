use std::sync::Arc;

use rust_decimal::Decimal;
use time::OffsetDateTime;

use super::fulfilment::{RecipeFulfilments, expand_recipe};
use super::stock_effects::{StockAffected, name_outcomes, requirement_deduction};
use crate::domain::{
    ConsumedNutrition, MealPlanComponentId, NewPreparedBatch, NewStockEvent, PreparationSource,
    PreparedBatch, PreparedBatchId, Quantity, Recipe, RecipeId, RecipeRequirement, Revision,
    StockEffectSource, StockEventKind, StockEventSource, StockItem, StockItemId, StockLevel,
    StockSubject, StorageLocation, Unit, UsabilityDeadline, UserId, recipe_nutrition,
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
    pub storage_location: StorageLocation,
    pub usability_deadline: Option<UsabilityDeadline>,
    pub note: Option<String>,
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

    pub async fn record(&self, input: RecordPreparation) -> Result<StockAffected<PreparedBatch>> {
        let recipe = self
            .recipes
            .get(input.recipe_id)
            .await?
            .ok_or_else(|| CoreError::not_found("recipe", input.recipe_id))?;
        if recipe.is_archived() {
            return Err(CoreError::conflict("That recipe is archived."));
        }

        let (batch, portion, event, write) = self.plan(&recipe, &input).await?;
        let outcomes = self
            .batches
            .insert(&batch, &portion, &event, &write)
            .await?;
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
    ) -> Result<(PreparedBatch, StockItem, NewStockEvent, StockWrite)> {
        let servings = input.servings_produced;
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

        let level = StockLevel::Exact {
            quantity: Quantity::new(servings, Unit::Serving),
        };
        let portion = StockItem {
            id: StockItemId::new(),
            subject: StockSubject::prepared_portion(batch.id),
            level,
            storage_location: input.storage_location,
            source_date: None,
            usability_deadline: input.usability_deadline.clone(),
            note: input.note.clone(),
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

        let write = StockWrite {
            deductions: self.raw_deductions(recipe, &batch, servings, input).await?,
            releases: Vec::new(),
        };
        Ok((batch, portion, event, write))
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
