use super::*;
use crate::domain::Unit;

fn per_100g() -> Quantity {
    Quantity::new(Decimal::new(100, 0), Unit::Gram)
}

#[test]
fn empty_facts_are_unknown() {
    assert!(NutritionFacts::default().is_unknown());
}

#[test]
fn a_zero_value_is_known_not_unknown() {
    let facts = NutritionFacts {
        basis: Some(per_100g()),
        fat_g: Some(Decimal::ZERO),
        ..Default::default()
    };
    assert!(!facts.is_unknown());
}

#[test]
fn negative_nutrients_are_rejected() {
    let facts = NutritionFacts {
        basis: Some(per_100g()),
        protein_g: Some(Decimal::new(-1, 0)),
        ..Default::default()
    };
    let mut errors = ValidationErrors::new();
    facts.validate("nutrition", &mut errors);
    assert!(!errors.is_empty());
}

#[test]
fn a_basis_is_required_once_any_value_is_supplied() {
    let facts = NutritionFacts {
        energy_kcal: Some(Decimal::new(250, 0)),
        ..Default::default()
    };
    let mut errors = ValidationErrors::new();
    facts.validate("nutrition", &mut errors);
    assert!(errors.iter().any(|e| e.field == "nutrition.basis"));
}

#[test]
fn a_basis_of_zero_is_rejected() {
    let facts = NutritionFacts {
        basis: Some(Quantity::new(Decimal::ZERO, Unit::Gram)),
        energy_kcal: Some(Decimal::new(250, 0)),
        ..Default::default()
    };
    let mut errors = ValidationErrors::new();
    facts.validate("nutrition", &mut errors);
    assert!(errors.iter().any(|e| e.field == "nutrition.basis.amount"));
}

#[test]
fn a_serving_sized_basis_is_expressible() {
    let facts = NutritionFacts {
        basis: Some(Quantity::new(Decimal::new(30, 0), Unit::Gram)),
        energy_kcal: Some(Decimal::new(120, 0)),
        ..Default::default()
    };
    let mut errors = ValidationErrors::new();
    facts.validate("nutrition", &mut errors);
    assert!(
        errors.is_empty(),
        "a per-serving label must be representable"
    );
}

#[test]
fn wholly_unknown_nutrition_needs_no_basis() {
    let mut errors = ValidationErrors::new();
    NutritionFacts::default().validate("nutrition", &mut errors);
    assert!(errors.is_empty());
}

#[test]
fn named_nutrient_list_matches_the_accessors() {
    let facts = NutritionFacts::default();
    let names: Vec<&str> = facts.named_values().map(|(name, _)| name).collect();
    assert_eq!(names, NutritionFacts::NAMED_NUTRIENTS.to_vec());
}
