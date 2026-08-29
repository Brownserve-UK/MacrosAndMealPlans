use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;
use time::{Date, OffsetDateTime};

use super::{
    ConsumptionRecordId, HouseholdMemberId, MealItemRef, MealPlanComponentId, MealPlanEntryId,
    MealSlot, NutritionFacts, Product, Quantity, Revision, UserId,
};
use crate::error::ValidationErrors;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConsumedAmount {
    Measure(Quantity),
    Servings(Decimal),
    Packs(Decimal),
}

impl ConsumedAmount {
    pub const fn kind_code(&self) -> &'static str {
        match self {
            ConsumedAmount::Measure(_) => "measure",
            ConsumedAmount::Servings(_) => "servings",
            ConsumedAmount::Packs(_) => "packs",
        }
    }

    pub fn value(&self) -> Decimal {
        match self {
            ConsumedAmount::Measure(quantity) => quantity.amount,
            ConsumedAmount::Servings(value) | ConsumedAmount::Packs(value) => *value,
        }
    }

    pub fn resolve(&self, product: &Product) -> Result<Quantity, AmountError> {
        match self {
            ConsumedAmount::Measure(quantity) => Ok(*quantity),
            ConsumedAmount::Packs(count) => {
                let pack = product.package_quantity.ok_or(AmountError::NoPackSize)?;
                Ok(Quantity::new(pack.amount * count, pack.unit))
            }
            ConsumedAmount::Servings(count) => {
                let pack = product.package_quantity.ok_or(AmountError::NoPackSize)?;
                let servings = product
                    .servings_per_pack
                    .ok_or(AmountError::NoServingCount)?;
                let per_serving = pack.amount / Decimal::from(servings);
                Ok(Quantity::new(per_serving * count, pack.unit))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AmountError {
    #[error("that product has no pack size to measure packs or servings against")]
    NoPackSize,
    #[error("that product has no servings-per-pack count")]
    NoServingCount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum NutritionQuality {
    Known,
    Estimated,
    Partial,
    Unknown,
}

impl NutritionQuality {
    pub const ALL: [NutritionQuality; 4] = [
        NutritionQuality::Known,
        NutritionQuality::Estimated,
        NutritionQuality::Partial,
        NutritionQuality::Unknown,
    ];

    pub const fn code(&self) -> &'static str {
        match self {
            NutritionQuality::Known => "known",
            NutritionQuality::Estimated => "estimated",
            NutritionQuality::Partial => "partial",
            NutritionQuality::Unknown => "unknown",
        }
    }
}

impl fmt::Display for NutritionQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known nutrition quality")]
pub struct UnknownNutritionQuality(pub String);

impl FromStr for NutritionQuality {
    type Err = UnknownNutritionQuality;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        NutritionQuality::ALL
            .into_iter()
            .find(|q| q.code() == s)
            .ok_or_else(|| UnknownNutritionQuality(s.to_owned()))
    }
}

#[derive(Debug, Clone)]
pub struct ConsumedNutrition {
    pub facts: NutritionFacts,
    pub quality: NutritionQuality,
}

impl ConsumedNutrition {
    pub fn unknown() -> Self {
        Self {
            facts: NutritionFacts::default(),
            quality: NutritionQuality::Unknown,
        }
    }
}

pub fn nutrition_for(product: &Product, amount: &ConsumedAmount) -> ConsumedNutrition {
    let Ok(resolved) = amount.resolve(product) else {
        return ConsumedNutrition::unknown();
    };
    let Some(basis) = product.nutrition.basis else {
        return ConsumedNutrition::unknown();
    };
    let Ok(converted) = resolved.convert_to(basis.unit) else {
        return ConsumedNutrition::unknown();
    };

    let factor = converted.amount / basis.amount;
    let facts = scale_facts(&product.nutrition, factor, converted);
    let quality = quality_of(&facts);
    ConsumedNutrition { facts, quality }
}

pub fn recipe_nutrition_for(
    per_serving: &ConsumedNutrition,
    amount: &ConsumedAmount,
) -> ConsumedNutrition {
    let ConsumedAmount::Servings(servings) = amount else {
        return ConsumedNutrition::unknown();
    };
    let facts = NutritionFacts {
        basis: None,
        ..per_serving.facts.scale(*servings)
    };
    ConsumedNutrition {
        facts,
        quality: per_serving.quality,
    }
}

pub fn sum_nutrition<'a>(facts: impl IntoIterator<Item = &'a NutritionFacts>) -> NutritionFacts {
    let mut total = NutritionFacts::default();
    for f in facts {
        total.energy_kcal = add_optional(total.energy_kcal, f.energy_kcal);
        total.protein_g = add_optional(total.protein_g, f.protein_g);
        total.carbohydrate_g = add_optional(total.carbohydrate_g, f.carbohydrate_g);
        total.sugar_g = add_optional(total.sugar_g, f.sugar_g);
        total.fat_g = add_optional(total.fat_g, f.fat_g);
        total.saturated_fat_g = add_optional(total.saturated_fat_g, f.saturated_fat_g);
        total.fibre_g = add_optional(total.fibre_g, f.fibre_g);
        total.salt_g = add_optional(total.salt_g, f.salt_g);
        total.cholesterol_mg = add_optional(total.cholesterol_mg, f.cholesterol_mg);
        for (key, value) in &f.extra {
            *total.extra.entry(key.clone()).or_insert(Decimal::ZERO) += value;
        }
    }
    total
}

fn add_optional(a: Option<Decimal>, b: Option<Decimal>) -> Option<Decimal> {
    match (a, b) {
        (None, None) => None,
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (Some(x), Some(y)) => Some(x + y),
    }
}

pub fn mean_nutrition<'a>(facts: impl IntoIterator<Item = &'a NutritionFacts>) -> NutritionFacts {
    let facts: Vec<&NutritionFacts> = facts.into_iter().collect();

    fn mean(values: impl Iterator<Item = Option<Decimal>>) -> Option<Decimal> {
        let present: Vec<Decimal> = values.flatten().collect();
        (!present.is_empty())
            .then(|| present.iter().copied().sum::<Decimal>() / Decimal::from(present.len()))
    }

    let mut extra_keys: BTreeSet<&String> = BTreeSet::new();
    for f in &facts {
        extra_keys.extend(f.extra.keys());
    }
    let extra = extra_keys
        .into_iter()
        .filter_map(|key| {
            mean(facts.iter().map(|f| f.extra.get(key).copied())).map(|value| (key.clone(), value))
        })
        .collect();

    NutritionFacts {
        basis: None,
        energy_kcal: mean(facts.iter().map(|f| f.energy_kcal)),
        protein_g: mean(facts.iter().map(|f| f.protein_g)),
        carbohydrate_g: mean(facts.iter().map(|f| f.carbohydrate_g)),
        sugar_g: mean(facts.iter().map(|f| f.sugar_g)),
        fat_g: mean(facts.iter().map(|f| f.fat_g)),
        saturated_fat_g: mean(facts.iter().map(|f| f.saturated_fat_g)),
        fibre_g: mean(facts.iter().map(|f| f.fibre_g)),
        salt_g: mean(facts.iter().map(|f| f.salt_g)),
        cholesterol_mg: mean(facts.iter().map(|f| f.cholesterol_mg)),
        extra,
    }
}

fn scale_facts(source: &NutritionFacts, factor: Decimal, basis: Quantity) -> NutritionFacts {
    NutritionFacts {
        basis: Some(basis),
        energy_kcal: source.energy_kcal.map(|v| v * factor),
        protein_g: source.protein_g.map(|v| v * factor),
        carbohydrate_g: source.carbohydrate_g.map(|v| v * factor),
        sugar_g: source.sugar_g.map(|v| v * factor),
        fat_g: source.fat_g.map(|v| v * factor),
        saturated_fat_g: source.saturated_fat_g.map(|v| v * factor),
        fibre_g: source.fibre_g.map(|v| v * factor),
        salt_g: source.salt_g.map(|v| v * factor),
        cholesterol_mg: source.cholesterol_mg.map(|v| v * factor),
        extra: source
            .extra
            .iter()
            .map(|(k, v)| (k.clone(), v * factor))
            .collect(),
    }
}

fn quality_of(facts: &NutritionFacts) -> NutritionQuality {
    if facts.is_unknown() {
        NutritionQuality::Unknown
    } else if facts.named_values().all(|(_, v)| v.is_some()) {
        NutritionQuality::Known
    } else {
        NutritionQuality::Partial
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConsumptionRecord {
    pub id: ConsumptionRecordId,
    pub member_id: HouseholdMemberId,
    pub item: MealItemRef,
    pub recorded_by: Option<UserId>,
    pub meal_plan_entry_id: Option<MealPlanEntryId>,
    pub meal_plan_component_id: Option<MealPlanComponentId>,
    pub slot: MealSlot,
    pub amount: ConsumedAmount,
    pub consumed_on: Date,
    pub consumed_at: Option<OffsetDateTime>,
    pub nutrition: NutritionFacts,
    pub quality: NutritionQuality,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct NewConsumptionRecord {
    pub id: Option<ConsumptionRecordId>,
    pub member_id: HouseholdMemberId,
    pub item: MealItemRef,
    pub recorded_by: Option<UserId>,
    pub meal_plan_entry_id: Option<MealPlanEntryId>,
    pub meal_plan_component_id: Option<MealPlanComponentId>,
    pub slot: MealSlot,
    pub amount: ConsumedAmount,
    pub consumed_on: Date,
    pub consumed_at: Option<OffsetDateTime>,
}

impl NewConsumptionRecord {
    pub fn validate(&self) -> crate::error::Result<()> {
        let mut errors = ValidationErrors::new();
        validate_amount("amount", &self.amount, &mut errors);
        validate_recipe_amount("amount", self.item, &self.amount, &mut errors);
        errors.into_result()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConsumptionRecordPatch {
    pub slot: Option<MealSlot>,
    pub amount: Option<ConsumedAmount>,
    pub consumed_on: Option<Date>,
    pub consumed_at: Option<Option<OffsetDateTime>>,
}

impl ConsumptionRecordPatch {
    pub fn is_empty(&self) -> bool {
        self.slot.is_none()
            && self.amount.is_none()
            && self.consumed_on.is_none()
            && self.consumed_at.is_none()
    }

    pub fn validate(&self) -> crate::error::Result<()> {
        let mut errors = ValidationErrors::new();
        if let Some(amount) = &self.amount {
            validate_amount("amount", amount, &mut errors);
        }
        errors.into_result()
    }
}

fn validate_amount(field: &str, amount: &ConsumedAmount, errors: &mut ValidationErrors) {
    if amount.value() <= Decimal::ZERO {
        errors.push(field, "Must be more than zero");
    }
}

pub(crate) fn validate_recipe_amount(
    field: &str,
    item: MealItemRef,
    amount: &ConsumedAmount,
    errors: &mut ValidationErrors,
) {
    if item.is_recipe() && !matches!(amount, ConsumedAmount::Servings(_)) {
        errors.push(field, "Recipes are measured in servings");
    }
}

#[cfg(test)]
#[path = "consumption_tests.rs"]
mod tests;
