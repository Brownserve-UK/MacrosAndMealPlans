use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::domain::{
    ConsumedAmount, ConsumptionRecord, HouseholdMemberId, Product, ProductId, Quantity, Shortfall,
    StockEffectSource, StockOutcome, UserId,
};
use crate::error::Result;
use crate::ports::{ProductRepository, StockDeduction, StockRelease};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StockOutcomeView {
    pub product_id: ProductId,
    pub product_name: String,
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
        product_id: product.id,
        want,
        actor_user_id: actor,
        subject_member_id: subject,
        source_label,
    })
}

pub fn component_release(
    component_id: uuid::Uuid,
    source_label: String,
    actor: Option<UserId>,
    subject: Option<HouseholdMemberId>,
) -> StockRelease {
    StockRelease {
        source_kind: StockEffectSource::MealPlanComponent,
        source_id: component_id,
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
        actor_user_id: record.recorded_by,
        subject_member_id: Some(record.member_id),
        source_label,
    }
}

pub async fn name_outcomes(
    products: &dyn ProductRepository,
    outcomes: Vec<StockOutcome>,
) -> Result<Vec<StockOutcomeView>> {
    if outcomes.is_empty() {
        return Ok(Vec::new());
    }
    let mut ids: Vec<ProductId> = outcomes.iter().map(|o| o.product_id).collect();
    ids.sort_unstable_by_key(|id| id.as_uuid());
    ids.dedup();
    let names: HashMap<ProductId, String> = products
        .get_many(&ids)
        .await?
        .into_iter()
        .map(|p| (p.id, p.name))
        .collect();
    Ok(outcomes
        .into_iter()
        .map(|o| StockOutcomeView {
            product_id: o.product_id,
            product_name: names
                .get(&o.product_id)
                .cloned()
                .unwrap_or_else(|| "Unknown product".to_owned()),
            wanted: o.wanted,
            deducted: o.deducted,
            shortfall: o.shortfall,
            unresolved_release: o.unresolved_release,
        })
        .collect())
}
