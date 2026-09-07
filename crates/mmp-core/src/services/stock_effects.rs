use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::domain::{
    ConsumedAmount, ConsumptionRecord, DeductionTarget, DemandSubject, HouseholdMemberId,
    IngredientId, PreparedBatchId, Product, ProductId, Quantity, Shortfall, StockEffectSource,
    StockOutcome, UserId,
};
use crate::error::Result;
use crate::ports::{
    IngredientRepository, PreparedBatchRepository, ProductRepository, StockDeduction, StockRelease,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StockOutcomeView {
    pub subject: DemandSubject,
    pub name: String,
    pub wanted: Quantity,
    pub deducted: Quantity,
    pub shortfall: Shortfall,
    pub unresolved_release: bool,
}

#[derive(Debug, Clone)]
pub struct StockAffected<T> {
    pub value: T,
    pub stock: Vec<StockOutcomeView>,
}

impl<T> StockAffected<T> {
    pub fn new(value: T, stock: Vec<StockOutcomeView>) -> Self {
        Self { value, stock }
    }

    pub fn bare(value: T) -> Self {
        Self {
            value,
            stock: Vec::new(),
        }
    }

    pub fn into_value(self) -> T {
        self.value
    }
}

impl<T> Deref for StockAffected<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for StockAffected<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

#[allow(clippy::too_many_arguments)]
pub fn product_deduction(
    source_kind: StockEffectSource,
    source_id: uuid::Uuid,
    product: &Product,
    amount: &ConsumedAmount,
    source_label: String,
    actor: Option<UserId>,
    subject: Option<HouseholdMemberId>,
) -> Option<StockDeduction> {
    let want = amount.resolve(product).ok()?;
    Some(StockDeduction {
        source_kind,
        source_id,
        source_detail_id: None,
        target: DeductionTarget::product(product.id),
        want,
        actor_user_id: actor,
        subject_member_id: subject,
        source_label,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn requirement_deduction(
    source_kind: StockEffectSource,
    source_id: uuid::Uuid,
    source_detail_id: uuid::Uuid,
    target: DeductionTarget,
    want: Quantity,
    source_label: String,
    actor: Option<UserId>,
    subject: Option<HouseholdMemberId>,
) -> StockDeduction {
    StockDeduction {
        source_kind,
        source_id,
        source_detail_id: Some(source_detail_id),
        target,
        want,
        actor_user_id: actor,
        subject_member_id: subject,
        source_label,
    }
}

pub fn portion_deduction(
    component_id: uuid::Uuid,
    eater_id: uuid::Uuid,
    prepared_batch_id: PreparedBatchId,
    want: Quantity,
    source_label: String,
    actor: Option<UserId>,
    subject: Option<HouseholdMemberId>,
) -> StockDeduction {
    StockDeduction {
        source_kind: StockEffectSource::MealPlanComponent,
        source_id: component_id,
        source_detail_id: Some(eater_id),
        target: DeductionTarget::prepared_portion(prepared_batch_id),
        want,
        actor_user_id: actor,
        subject_member_id: subject,
        source_label,
    }
}

pub fn cooked_food_deduction(
    source_id: uuid::Uuid,
    eater_id: uuid::Uuid,
    recipe_id: crate::domain::RecipeId,
    want: Quantity,
    source_label: String,
    actor: Option<UserId>,
    subject: Option<HouseholdMemberId>,
) -> StockDeduction {
    StockDeduction {
        source_kind: StockEffectSource::MealPlanComponent,
        source_id,
        source_detail_id: Some(eater_id),
        target: DeductionTarget::cooked_food(recipe_id),
        want,
        actor_user_id: actor,
        subject_member_id: subject,
        source_label,
    }
}

pub fn component_release(
    component_id: uuid::Uuid,
    eater_id: Option<uuid::Uuid>,
    source_label: String,
    actor: Option<UserId>,
    subject: Option<HouseholdMemberId>,
) -> StockRelease {
    StockRelease {
        source_kind: StockEffectSource::MealPlanComponent,
        source_id: component_id,
        source_detail_id: eater_id,
        actor_user_id: actor,
        subject_member_id: subject,
        source_label,
    }
}

pub fn record_deduction(
    record: &ConsumptionRecord,
    product: &Product,
    source_label: String,
) -> Option<StockDeduction> {
    product_deduction(
        StockEffectSource::ConsumptionRecord,
        record.id.as_uuid(),
        product,
        &record.amount,
        source_label,
        record.recorded_by,
        Some(record.member_id),
    )
}

pub fn record_release(record: &ConsumptionRecord, source_label: String) -> StockRelease {
    StockRelease {
        source_kind: StockEffectSource::ConsumptionRecord,
        source_id: record.id.as_uuid(),
        source_detail_id: None,
        actor_user_id: record.recorded_by,
        subject_member_id: Some(record.member_id),
        source_label,
    }
}

pub async fn name_outcomes(
    products: &dyn ProductRepository,
    ingredients: &dyn IngredientRepository,
    batches: &dyn PreparedBatchRepository,
    outcomes: Vec<StockOutcome>,
) -> Result<Vec<StockOutcomeView>> {
    if outcomes.is_empty() {
        return Ok(Vec::new());
    }
    let mut product_ids: Vec<ProductId> = outcomes
        .iter()
        .filter_map(|o| o.subject.product_id())
        .collect();
    product_ids.sort_unstable_by_key(|id| id.as_uuid());
    product_ids.dedup();
    let mut ingredient_ids: Vec<IngredientId> = outcomes
        .iter()
        .filter_map(|o| o.subject.ingredient_id())
        .collect();
    ingredient_ids.sort_unstable_by_key(|id| id.as_uuid());
    ingredient_ids.dedup();
    let mut batch_ids: Vec<PreparedBatchId> = outcomes
        .iter()
        .filter_map(|o| o.subject.prepared_batch_id())
        .collect();
    batch_ids.sort_unstable_by_key(|id| id.as_uuid());
    batch_ids.dedup();
    let mut cooked_ids: Vec<crate::domain::RecipeId> = outcomes
        .iter()
        .filter_map(|o| o.subject.cooked_recipe_id())
        .collect();
    cooked_ids.sort_unstable_by_key(|id| id.as_uuid());
    cooked_ids.dedup();

    let mut cooked_names: HashMap<crate::domain::RecipeId, String> = HashMap::new();
    for recipe_id in cooked_ids {
        if let Some(batch) = batches.held_for_recipe(recipe_id).await?.first() {
            cooked_names.insert(recipe_id, batch.item_name.clone());
        }
    }

    let product_names: HashMap<ProductId, String> = products
        .get_many(&product_ids)
        .await?
        .into_iter()
        .map(|p| (p.id, p.name))
        .collect();
    let ingredient_names: HashMap<IngredientId, String> = ingredients
        .get_many(&ingredient_ids)
        .await?
        .into_iter()
        .map(|i| (i.id, i.name))
        .collect();
    let batch_names: HashMap<PreparedBatchId, String> = batches
        .get_many(&batch_ids)
        .await?
        .into_iter()
        .map(|b| (b.id, b.item_name))
        .collect();

    Ok(outcomes
        .into_iter()
        .map(|o| StockOutcomeView {
            subject: o.subject,
            name: match o.subject {
                DemandSubject::Product { product_id } => product_names
                    .get(&product_id)
                    .cloned()
                    .unwrap_or_else(|| "Unknown product".to_owned()),
                DemandSubject::Ingredient { ingredient_id } => ingredient_names
                    .get(&ingredient_id)
                    .cloned()
                    .unwrap_or_else(|| "Unknown ingredient".to_owned()),
                DemandSubject::PreparedPortion { prepared_batch_id } => batch_names
                    .get(&prepared_batch_id)
                    .cloned()
                    .unwrap_or_else(|| "Unknown prepared portion".to_owned()),
                DemandSubject::CookedFood { recipe_id } => cooked_names
                    .get(&recipe_id)
                    .cloned()
                    .unwrap_or_else(|| "Cooked food".to_owned()),
            },
            wanted: o.wanted,
            deducted: o.deducted,
            shortfall: o.shortfall,
            unresolved_release: o.unresolved_release,
        })
        .collect())
}

#[cfg(test)]
#[path = "stock_effects_tests.rs"]
mod tests;
