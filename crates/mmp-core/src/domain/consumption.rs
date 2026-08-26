use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;
use time::{Date, OffsetDateTime};

use super::{
    ConsumptionRecordId, HouseholdMemberId, MealPlanComponentId, MealPlanEntryId, MealSlot,
    NutritionFacts, Product, ProductId, Quantity, Revision, UserId,
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
    Partial,
    Unknown,
}

impl NutritionQuality {
    pub const ALL: [NutritionQuality; 3] = [
        NutritionQuality::Known,
        NutritionQuality::Partial,
        NutritionQuality::Unknown,
    ];

    pub const fn code(&self) -> &'static str {
        match self {
            NutritionQuality::Known => "known",
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

pub struct ConsumedNutrition {
    pub facts: NutritionFacts,
    pub quality: NutritionQuality,
}

impl ConsumedNutrition {
    fn unknown() -> Self {
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
    pub product_id: ProductId,
    pub recorded_by: Option<UserId>,
    pub meal_plan_entry_id: Option<MealPlanEntryId>,
    pub meal_plan_component_id: Option<MealPlanComponentId>,
    pub slot: MealSlot,
    pub amount: ConsumedAmount,
    pub consumed_on: Date,
    pub consumed_at: OffsetDateTime,
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
    pub product_id: ProductId,
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
        errors.into_result()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConsumptionRecordPatch {
    pub slot: Option<MealSlot>,
    pub amount: Option<ConsumedAmount>,
    pub consumed_on: Option<Date>,
    pub consumed_at: Option<OffsetDateTime>,
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

#[cfg(test)]
#[path = "consumption_tests.rs"]
mod tests;
