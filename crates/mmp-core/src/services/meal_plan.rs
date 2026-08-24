use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use time::{Date, Duration};

use crate::domain::{
    ConfirmMealPlanEntry, ConsumedAmount, ConsumptionRecord, MealPlanComponent,
    MealPlanComponentSnapshot, MealPlanEntry, MealPlanEntryId, MealPlanEntryPatch, MealPlanStatus,
    NewMealPlanComponent, NewMealPlanEntry, NutritionFacts, NutritionQuality, Product, ProductId,
    Revision, nutrition_for, sum_nutrition, validate_components,
};
use crate::error::{CoreError, Result, ValidationErrors};
use crate::ports::{
    Clock, ConsumptionRecordRepository, MealPlanQuery, MealPlanRepository, ProductRepository,
    UpdateOutcome,
};

const MEAL_PLAN_ENTRY: &str = "meal plan entry";
const PRODUCT: &str = "product";

#[derive(Debug, Clone, Default)]
pub struct NutritionSummary {
    pub nutrition: NutritionFacts,
    pub unknown_count: i64,
    pub partial_count: i64,
}

#[derive(Debug, Clone)]
pub struct MealPlanComponentView {
    pub component: MealPlanComponent,
    pub product_name: String,
    pub nutrition: NutritionFacts,
    pub quality: NutritionQuality,
    pub consumption_record: Option<ConsumptionRecord>,
}

#[derive(Debug, Clone)]
pub struct MealPlanEntryView {
    pub entry: MealPlanEntry,
    pub components: Vec<MealPlanComponentView>,
    pub planned: NutritionSummary,
    pub actual: Option<NutritionSummary>,
    pub needs_attention: bool,
}

#[derive(Debug, Clone)]
pub struct MealPlanDay {
    pub date: Date,
    pub entries: Vec<MealPlanEntryView>,
    pub actual: NutritionSummary,
    pub remaining_planned: NutritionSummary,
    pub projected: NutritionSummary,
}

#[derive(Debug, Clone)]
pub struct MealPlanWeek {
    pub member_id: crate::domain::HouseholdMemberId,
    pub week_start: Date,
    pub week_end: Date,
    pub days: Vec<MealPlanDay>,
    pub actual: NutritionSummary,
    pub remaining_planned: NutritionSummary,
    pub projected: NutritionSummary,
}

#[derive(Clone)]
pub struct MealPlanService {
    plans: Arc<dyn MealPlanRepository>,
    products: Arc<dyn ProductRepository>,
    consumption: Arc<dyn ConsumptionRecordRepository>,
    clock: Arc<dyn Clock>,
}

impl MealPlanService {
    pub fn new(
        plans: Arc<dyn MealPlanRepository>,
        products: Arc<dyn ProductRepository>,
        consumption: Arc<dyn ConsumptionRecordRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            plans,
            products,
            consumption,
            clock,
        }
    }

    pub async fn create(&self, input: NewMealPlanEntry) -> Result<MealPlanEntryView> {
        validate_components(&input.components)?;
        self.validate_products(&input.components, &HashSet::new())
            .await?;
        let now = self.clock.now();
        let entry = MealPlanEntry {
            id: input.id.unwrap_or_default(),
            member_id: input.member_id,
            planned_on: input.planned_on,
            planned_time: input.planned_time,
            slot: input.slot,
            status: MealPlanStatus::Planned,
            components: make_components(input.components),
            created_by: input.actor_id,
            updated_by: input.actor_id,
            resolved_by: None,
            resolved_at: None,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
        };
        self.plans.insert(&entry).await?;
        self.present(entry, &[]).await
    }

    pub async fn get(&self, id: MealPlanEntryId) -> Result<MealPlanEntryView> {
        let entry = self.get_entry(id).await?;
        let records = self.records_for_entry(entry.id).await?;
        self.present(entry, &records).await
    }

    pub async fn update(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        patch: MealPlanEntryPatch,
        actor_id: crate::domain::UserId,
    ) -> Result<MealPlanEntryView> {
        let mut entry = self.get_entry(id).await?;
        require_revision(id, expected, entry.revision)?;
        require_planned(&entry)?;

        if let Some(components) = patch.components {
            validate_components(&components)?;
            let existing_products = entry
                .components
                .iter()
                .map(|component| component.product_id)
                .collect();
            self.validate_products(&components, &existing_products)
                .await?;
            entry.components = make_components(components);
        }
        if let Some(planned_on) = patch.planned_on {
            entry.planned_on = planned_on;
        }
        if let Some(planned_time) = patch.planned_time {
            entry.planned_time = planned_time;
        }
        if let Some(slot) = patch.slot {
            entry.slot = slot;
        }
        entry.updated_by = actor_id;
        entry.updated_at = self.clock.now();
        entry.revision = entry.revision.next();
        commit_outcome(self.plans.update(&entry, expected).await?, id, expected)?;
        self.present(entry, &[]).await
    }

    pub async fn delete(&self, id: MealPlanEntryId, expected: Revision) -> Result<()> {
        let entry = self.get_entry(id).await?;
        require_revision(id, expected, entry.revision)?;
        require_planned(&entry)?;
        commit_outcome(self.plans.delete(id, expected).await?, id, expected)
    }

    pub async fn mark_not_eaten(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        actor_id: crate::domain::UserId,
    ) -> Result<MealPlanEntryView> {
        let mut entry = self.get_entry(id).await?;
        require_revision(id, expected, entry.revision)?;
        require_planned(&entry)?;
        self.freeze(&mut entry).await?;
        let now = self.clock.now();
        entry.status = MealPlanStatus::NotEaten;
        entry.resolved_by = Some(actor_id);
        entry.resolved_at = Some(now);
        entry.updated_by = actor_id;
        entry.updated_at = now;
        entry.revision = entry.revision.next();
        commit_outcome(
            self.plans.resolve(&entry, expected, &[]).await?,
            id,
            expected,
        )?;
        self.present(entry, &[]).await
    }

    pub async fn mark_eaten(
        &self,
        id: MealPlanEntryId,
        expected: Revision,
        input: ConfirmMealPlanEntry,
    ) -> Result<MealPlanEntryView> {
        let mut entry = self.get_entry(id).await?;
        require_revision(id, expected, entry.revision)?;
        require_planned(&entry)?;
        validate_actual_components(&entry, &input)?;
        self.freeze(&mut entry).await?;

        let products = self.products_for_entry(&entry).await?;
        let actual_by_component: HashMap<_, _> = input
            .components
            .iter()
            .map(|actual| (actual.component_id, actual.amount))
            .collect();
        let now = self.clock.now();
        let mut records = Vec::with_capacity(entry.components.len());
        for component in &entry.components {
            let product = products
                .get(&component.product_id)
                .ok_or_else(|| CoreError::not_found(PRODUCT, component.product_id))?;
            let amount = actual_by_component[&component.id];
            ensure_resolvable(
                product,
                &amount,
                &format!("components.{}.amount", component.id),
            )?;
            let scaled = nutrition_for(product, &amount);
            records.push(ConsumptionRecord {
                id: Default::default(),
                member_id: entry.member_id,
                product_id: component.product_id,
                recorded_by: Some(input.actor_id),
                meal_plan_entry_id: Some(entry.id),
                meal_plan_component_id: Some(component.id),
                amount,
                consumed_on: input.consumed_on,
                consumed_at: input.consumed_at,
                nutrition: scaled.facts,
                quality: scaled.quality,
                revision: Revision::INITIAL,
                created_at: now,
                updated_at: now,
            });
        }

        entry.status = MealPlanStatus::Eaten;
        entry.resolved_by = Some(input.actor_id);
        entry.resolved_at = Some(now);
        entry.updated_by = input.actor_id;
        entry.updated_at = now;
        entry.revision = entry.revision.next();
        commit_outcome(
            self.plans.resolve(&entry, expected, &records).await?,
            id,
            expected,
        )?;
        self.present(entry, &records).await
    }

    pub async fn week(
        &self,
        member_id: crate::domain::HouseholdMemberId,
        week_start: Date,
    ) -> Result<MealPlanWeek> {
        let week_end = week_start + Duration::days(6);
        let entries = self
            .plans
            .list(&MealPlanQuery {
                member_id,
                from: week_start,
                to: week_end,
            })
            .await?;
        let records = self
            .consumption
            .list_period(member_id, week_start, week_end)
            .await?;
        let mut records_by_entry: HashMap<MealPlanEntryId, Vec<ConsumptionRecord>> = HashMap::new();
        for record in &records {
            if let Some(entry_id) = record.meal_plan_entry_id {
                records_by_entry
                    .entry(entry_id)
                    .or_default()
                    .push(record.clone());
            }
        }

        let mut presented_by_date: BTreeMap<Date, Vec<MealPlanEntryView>> = BTreeMap::new();
        for entry in entries {
            let linked = records_by_entry
                .get(&entry.id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let date = entry.planned_on;
            presented_by_date
                .entry(date)
                .or_default()
                .push(self.present(entry, linked).await?);
        }

        let mut days = Vec::with_capacity(7);
        for offset in 0..7 {
            let date = week_start + Duration::days(offset);
            let mut day_entries = presented_by_date.remove(&date).unwrap_or_default();
            day_entries.sort_by_key(|view| {
                (
                    view.entry.slot.order(),
                    view.entry.planned_time,
                    view.entry.created_at,
                )
            });
            let actual = summary(records.iter().filter(|record| record.consumed_on == date));
            let remaining_planned = summary_from_views(
                day_entries
                    .iter()
                    .filter(|view| view.entry.status == MealPlanStatus::Planned),
            );
            let projected = combine_summaries(&actual, &remaining_planned);
            days.push(MealPlanDay {
                date,
                entries: day_entries,
                actual,
                remaining_planned,
                projected,
            });
        }

        let actual = combine_many(days.iter().map(|day| &day.actual));
        let remaining_planned = combine_many(days.iter().map(|day| &day.remaining_planned));
        let projected = combine_summaries(&actual, &remaining_planned);
        Ok(MealPlanWeek {
            member_id,
            week_start,
            week_end,
            days,
            actual,
            remaining_planned,
            projected,
        })
    }

    async fn get_entry(&self, id: MealPlanEntryId) -> Result<MealPlanEntry> {
        self.plans
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(MEAL_PLAN_ENTRY, id))
    }

    async fn validate_products(
        &self,
        components: &[NewMealPlanComponent],
        archived_allowed: &HashSet<ProductId>,
    ) -> Result<()> {
        for (index, component) in components.iter().enumerate() {
            let product = self
                .products
                .get(component.product_id)
                .await?
                .ok_or_else(|| CoreError::not_found(PRODUCT, component.product_id))?;
            let mut errors = ValidationErrors::new();
            if product.is_archived() && !archived_allowed.contains(&product.id) {
                errors.push(
                    format!("components.{index}.product_id"),
                    "That product is archived",
                );
            }
            if let Err(error) = component.amount.resolve(&product) {
                errors.push(format!("components.{index}.amount"), error.to_string());
            }
            errors.into_result()?;
        }
        Ok(())
    }

    async fn freeze(&self, entry: &mut MealPlanEntry) -> Result<()> {
        let products = self.products_for_entry(entry).await?;
        for component in &mut entry.components {
            let product = products
                .get(&component.product_id)
                .ok_or_else(|| CoreError::not_found(PRODUCT, component.product_id))?;
            ensure_resolvable(product, &component.amount, "components.amount")?;
            let scaled = nutrition_for(product, &component.amount);
            component.snapshot = Some(MealPlanComponentSnapshot {
                product_name: product.name.clone(),
                nutrition: scaled.facts,
                quality: scaled.quality,
            });
        }
        Ok(())
    }

    async fn products_for_entry(
        &self,
        entry: &MealPlanEntry,
    ) -> Result<HashMap<ProductId, Product>> {
        let ids: Vec<_> = entry
            .components
            .iter()
            .map(|component| component.product_id)
            .collect();
        Ok(self
            .products
            .get_many(&ids)
            .await?
            .into_iter()
            .map(|product| (product.id, product))
            .collect())
    }

    async fn present(
        &self,
        entry: MealPlanEntry,
        records: &[ConsumptionRecord],
    ) -> Result<MealPlanEntryView> {
        let products = if entry.status == MealPlanStatus::Planned {
            self.products_for_entry(&entry).await?
        } else {
            HashMap::new()
        };
        let records_by_component: HashMap<_, _> = records
            .iter()
            .filter_map(|record| record.meal_plan_component_id.map(|id| (id, record.clone())))
            .collect();
        let mut needs_attention = false;
        let mut components = Vec::with_capacity(entry.components.len());
        for component in &entry.components {
            let (product_name, nutrition, quality) = match &component.snapshot {
                Some(snapshot) => (
                    snapshot.product_name.clone(),
                    snapshot.nutrition.clone(),
                    snapshot.quality,
                ),
                None => match products.get(&component.product_id) {
                    Some(product) => {
                        let scaled = nutrition_for(product, &component.amount);
                        if component.amount.resolve(product).is_err() {
                            needs_attention = true;
                        }
                        (product.name.clone(), scaled.facts, scaled.quality)
                    }
                    None => {
                        needs_attention = true;
                        (
                            "Missing product".to_owned(),
                            NutritionFacts::default(),
                            NutritionQuality::Unknown,
                        )
                    }
                },
            };
            components.push(MealPlanComponentView {
                component: component.clone(),
                product_name,
                nutrition,
                quality,
                consumption_record: records_by_component.get(&component.id).cloned(),
            });
        }
        let planned = summary_components(&components);
        let actual = if records.is_empty() {
            None
        } else {
            Some(summary(records.iter()))
        };
        Ok(MealPlanEntryView {
            entry,
            components,
            planned,
            actual,
            needs_attention,
        })
    }

    async fn records_for_entry(&self, entry_id: MealPlanEntryId) -> Result<Vec<ConsumptionRecord>> {
        self.consumption.list_for_meal_plan_entry(entry_id).await
    }
}

fn make_components(input: Vec<NewMealPlanComponent>) -> Vec<MealPlanComponent> {
    input
        .into_iter()
        .enumerate()
        .map(|(position, component)| MealPlanComponent {
            id: Default::default(),
            product_id: component.product_id,
            amount: component.amount,
            position: i32::try_from(position).unwrap_or(i32::MAX),
            snapshot: None,
        })
        .collect()
}

fn validate_actual_components(entry: &MealPlanEntry, input: &ConfirmMealPlanEntry) -> Result<()> {
    let expected: HashSet<_> = entry
        .components
        .iter()
        .map(|component| component.id)
        .collect();
    let actual: HashSet<_> = input
        .components
        .iter()
        .map(|component| component.component_id)
        .collect();
    let mut errors = ValidationErrors::new();
    if input.components.len() != expected.len() || actual != expected {
        errors.push("components", "Confirm every planned product exactly once");
    }
    for (index, component) in input.components.iter().enumerate() {
        if component.amount.value() <= rust_decimal::Decimal::ZERO {
            errors.push(
                format!("components.{index}.amount"),
                "Must be more than zero",
            );
        }
    }
    errors.into_result()
}

fn require_planned(entry: &MealPlanEntry) -> Result<()> {
    if entry.status == MealPlanStatus::Planned {
        Ok(())
    } else {
        Err(CoreError::conflict(
            "Resolved meal plans cannot be changed.",
        ))
    }
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

fn ensure_resolvable(product: &Product, amount: &ConsumedAmount, field: &str) -> Result<()> {
    if let Err(error) = amount.resolve(product) {
        let mut errors = ValidationErrors::new();
        errors.push(field, error.to_string());
        return errors.into_result();
    }
    Ok(())
}

fn summary_components(components: &[MealPlanComponentView]) -> NutritionSummary {
    NutritionSummary {
        nutrition: sum_nutrition(components.iter().map(|component| &component.nutrition)),
        unknown_count: components
            .iter()
            .filter(|component| component.quality == NutritionQuality::Unknown)
            .count() as i64,
        partial_count: components
            .iter()
            .filter(|component| component.quality == NutritionQuality::Partial)
            .count() as i64,
    }
}

fn summary<'a>(records: impl Iterator<Item = &'a ConsumptionRecord>) -> NutritionSummary {
    let records: Vec<_> = records.collect();
    NutritionSummary {
        nutrition: sum_nutrition(records.iter().map(|record| &record.nutrition)),
        unknown_count: records
            .iter()
            .filter(|record| record.quality == NutritionQuality::Unknown)
            .count() as i64,
        partial_count: records
            .iter()
            .filter(|record| record.quality == NutritionQuality::Partial)
            .count() as i64,
    }
}

fn summary_from_views<'a>(views: impl Iterator<Item = &'a MealPlanEntryView>) -> NutritionSummary {
    combine_many(views.map(|view| &view.planned))
}

fn combine_many<'a>(summaries: impl Iterator<Item = &'a NutritionSummary>) -> NutritionSummary {
    let summaries: Vec<_> = summaries.collect();
    NutritionSummary {
        nutrition: sum_nutrition(summaries.iter().map(|summary| &summary.nutrition)),
        unknown_count: summaries.iter().map(|summary| summary.unknown_count).sum(),
        partial_count: summaries.iter().map(|summary| summary.partial_count).sum(),
    }
}

fn combine_summaries(left: &NutritionSummary, right: &NutritionSummary) -> NutritionSummary {
    combine_many([left, right].into_iter())
}

#[cfg(test)]
#[path = "meal_plan_tests.rs"]
mod tests;
