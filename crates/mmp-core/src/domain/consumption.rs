use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;
use time::{Date, OffsetDateTime};

use super::{
    ConsumptionRecordId, HouseholdMemberId, NutritionFacts, Product, ProductId, Quantity, Revision,
    UserId,
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

    fn value(&self) -> Decimal {
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
    pub amount: Option<ConsumedAmount>,
    pub consumed_on: Option<Date>,
    pub consumed_at: Option<OffsetDateTime>,
}

impl ConsumptionRecordPatch {
    pub fn is_empty(&self) -> bool {
        self.amount.is_none() && self.consumed_on.is_none() && self.consumed_at.is_none()
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
mod tests {
    use super::*;
    use crate::domain::{Provenance, Unit};

    fn product_with(
        package_quantity: Option<Quantity>,
        servings_per_pack: Option<i32>,
        nutrition: NutritionFacts,
    ) -> Product {
        let now = OffsetDateTime::now_utc();
        Product {
            id: ProductId::new(),
            name: "Test product".to_owned(),
            brand: None,
            barcode: None,
            retailer: None,
            shopping_section: None,
            package_quantity,
            servings_per_pack,
            mapped_ingredient_id: None,
            nutrition,
            provenance: Provenance::local(),
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
            archived_at: None,
        }
    }

    fn per_100g(energy_kcal: i64) -> NutritionFacts {
        NutritionFacts {
            basis: Some(Quantity::new(Decimal::new(100, 0), Unit::Gram)),
            energy_kcal: Some(Decimal::new(energy_kcal, 0)),
            protein_g: Some(Decimal::new(10, 0)),
            carbohydrate_g: Some(Decimal::new(20, 0)),
            sugar_g: Some(Decimal::new(5, 0)),
            fat_g: Some(Decimal::new(3, 0)),
            saturated_fat_g: Some(Decimal::new(1, 0)),
            fibre_g: Some(Decimal::new(2, 0)),
            salt_g: Some(Decimal::new(1, 1)),
            cholesterol_mg: Some(Decimal::new(0, 0)),
            extra: Default::default(),
        }
    }

    #[test]
    fn a_measured_amount_scales_by_the_basis_ratio() {
        let product = product_with(
            Some(Quantity::new(Decimal::new(650, 0), Unit::Gram)),
            None,
            per_100g(200),
        );
        let amount = ConsumedAmount::Measure(Quantity::new(Decimal::new(150, 0), Unit::Gram));
        let result = nutrition_for(&product, &amount);

        assert_eq!(result.quality, NutritionQuality::Known);
        assert_eq!(result.facts.energy_kcal, Some(Decimal::new(300, 0)));
        assert_eq!(
            result.facts.basis,
            Some(Quantity::new(Decimal::new(150, 0), Unit::Gram))
        );
    }

    #[test]
    fn two_items_of_a_per_item_basis_doubles_the_nutrients() {
        let mut nutrition = per_100g(80);
        nutrition.basis = Some(Quantity::new(Decimal::ONE, Unit::Item));
        let product = product_with(
            Some(Quantity::new(Decimal::new(6, 0), Unit::Item)),
            None,
            nutrition,
        );
        let amount = ConsumedAmount::Measure(Quantity::new(Decimal::new(2, 0), Unit::Item));
        let result = nutrition_for(&product, &amount);

        assert_eq!(result.facts.energy_kcal, Some(Decimal::new(160, 0)));
    }

    #[test]
    fn a_serving_of_a_pizza_resolves_to_a_quarter_item() {
        let mut nutrition = per_100g(1000);
        nutrition.basis = Some(Quantity::new(Decimal::new(25, 2), Unit::Item));
        let product = product_with(
            Some(Quantity::new(Decimal::ONE, Unit::Item)),
            Some(4),
            nutrition,
        );
        let amount = ConsumedAmount::Servings(Decimal::ONE);
        let result = nutrition_for(&product, &amount);

        assert_eq!(
            result.facts.basis,
            Some(Quantity::new(Decimal::new(25, 2), Unit::Item))
        );
        assert_eq!(result.facts.energy_kcal, Some(Decimal::new(1000, 0)));
    }

    #[test]
    fn half_a_pack_scales_to_the_resolved_weight() {
        let product = product_with(
            Some(Quantity::new(Decimal::new(650, 0), Unit::Gram)),
            None,
            per_100g(200),
        );
        let amount = ConsumedAmount::Packs(Decimal::new(5, 1));
        let result = nutrition_for(&product, &amount);

        assert_eq!(
            result.facts.basis,
            Some(Quantity::new(Decimal::new(325, 0), Unit::Gram))
        );
        assert_eq!(result.facts.energy_kcal, Some(Decimal::new(650, 0)));
    }

    #[test]
    fn a_serving_without_a_servings_count_is_refused() {
        let product = product_with(
            Some(Quantity::new(Decimal::new(650, 0), Unit::Gram)),
            None,
            per_100g(200),
        );
        let amount = ConsumedAmount::Servings(Decimal::ONE);
        assert_eq!(amount.resolve(&product), Err(AmountError::NoServingCount));
    }

    #[test]
    fn a_pack_amount_without_a_pack_size_is_refused() {
        let product = product_with(None, None, per_100g(200));
        let amount = ConsumedAmount::Packs(Decimal::ONE);
        assert_eq!(amount.resolve(&product), Err(AmountError::NoPackSize));
    }

    #[test]
    fn a_mass_amount_against_a_count_basis_is_unknown() {
        let mut nutrition = per_100g(80);
        nutrition.basis = Some(Quantity::new(Decimal::ONE, Unit::Item));
        let product = product_with(None, None, nutrition);
        let amount = ConsumedAmount::Measure(Quantity::new(Decimal::new(150, 0), Unit::Gram));
        let result = nutrition_for(&product, &amount);

        assert_eq!(result.quality, NutritionQuality::Unknown);
        assert!(result.facts.is_unknown());
    }

    #[test]
    fn a_bunch_against_an_item_basis_needs_a_conversion_we_do_not_have() {
        let mut nutrition = per_100g(80);
        nutrition.basis = Some(Quantity::new(Decimal::ONE, Unit::Item));
        let product = product_with(None, None, nutrition);
        let amount = ConsumedAmount::Measure(Quantity::new(Decimal::ONE, Unit::Bunch));
        let result = nutrition_for(&product, &amount);

        assert_eq!(result.quality, NutritionQuality::Unknown);
    }

    #[test]
    fn wholly_unknown_product_nutrition_stays_unknown() {
        let product = product_with(None, None, NutritionFacts::default());
        let amount = ConsumedAmount::Measure(Quantity::new(Decimal::new(150, 0), Unit::Gram));
        let result = nutrition_for(&product, &amount);

        assert_eq!(result.quality, NutritionQuality::Unknown);
    }

    #[test]
    fn a_partly_recorded_product_yields_partial_quality() {
        let mut nutrition = per_100g(200);
        nutrition.fibre_g = None;
        let product = product_with(
            Some(Quantity::new(Decimal::new(650, 0), Unit::Gram)),
            None,
            nutrition,
        );
        let amount = ConsumedAmount::Measure(Quantity::new(Decimal::new(150, 0), Unit::Gram));
        let result = nutrition_for(&product, &amount);

        assert_eq!(result.quality, NutritionQuality::Partial);
    }

    #[test]
    fn summing_leaves_a_wholly_missing_nutrient_unknown() {
        let a = NutritionFacts::default();
        let b = NutritionFacts::default();
        let total = sum_nutrition([&a, &b]);
        assert_eq!(total.energy_kcal, None);
    }

    #[test]
    fn summing_treats_a_missing_value_as_not_contributing() {
        let known = NutritionFacts {
            energy_kcal: Some(Decimal::new(100, 0)),
            ..Default::default()
        };
        let unknown = NutritionFacts::default();
        let total = sum_nutrition([&known, &unknown]);
        assert_eq!(total.energy_kcal, Some(Decimal::new(100, 0)));
    }

    #[test]
    fn summing_adds_present_values_together() {
        let a = NutritionFacts {
            energy_kcal: Some(Decimal::new(100, 0)),
            ..Default::default()
        };
        let b = NutritionFacts {
            energy_kcal: Some(Decimal::new(50, 0)),
            ..Default::default()
        };
        let total = sum_nutrition([&a, &b]);
        assert_eq!(total.energy_kcal, Some(Decimal::new(150, 0)));
    }

    #[test]
    fn quality_codes_round_trip() {
        for quality in NutritionQuality::ALL {
            assert_eq!(NutritionQuality::from_str(quality.code()).unwrap(), quality);
        }
    }

    #[test]
    fn a_zero_amount_is_rejected() {
        let mut errors = ValidationErrors::new();
        validate_amount(
            "amount",
            &ConsumedAmount::Servings(Decimal::ZERO),
            &mut errors,
        );
        assert!(!errors.is_empty());
    }

    #[test]
    fn an_empty_patch_is_detected() {
        assert!(ConsumptionRecordPatch::default().is_empty());
    }
}
