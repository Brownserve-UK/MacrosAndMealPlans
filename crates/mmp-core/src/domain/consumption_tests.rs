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
