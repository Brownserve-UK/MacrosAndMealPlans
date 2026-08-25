use super::*;
use mmp_core::domain::{Quantity, Unit};

fn per_100g() -> Quantity {
    Quantity::new(Decimal::new(100, 0), Unit::Gram)
}

#[test]
fn unknown_nutrition_round_trips_as_null_not_zero() {
    let dto = NutritionDto::from(NutritionFacts::default());
    let json = serde_json::to_value(&dto).unwrap();
    assert!(json.get("energy_kcal").unwrap().is_null());
    assert!(json.get("basis").is_none());
}

#[test]
fn values_are_json_numbers_not_strings() {
    let facts = NutritionFacts {
        basis: Some(per_100g()),
        energy_kcal: Some(Decimal::new(645, 1)),
        ..Default::default()
    };
    let json = serde_json::to_value(NutritionDto::from(facts)).unwrap();
    assert_eq!(json["energy_kcal"], serde_json::json!(64.5));
}

#[test]
fn the_basis_survives_the_round_trip() {
    let facts = NutritionFacts {
        basis: Some(Quantity::new(Decimal::new(30, 0), Unit::Gram)),
        energy_kcal: Some(Decimal::new(120, 0)),
        ..Default::default()
    };
    let back: NutritionFacts = NutritionDto::from(facts).into();
    assert_eq!(
        back.basis,
        Some(Quantity::new(Decimal::new(30, 0), Unit::Gram))
    );
}

#[test]
fn a_zero_survives_the_round_trip_as_a_known_value() {
    let facts = NutritionFacts {
        basis: Some(per_100g()),
        fat_g: Some(Decimal::ZERO),
        ..Default::default()
    };
    let back: NutritionFacts = NutritionDto::from(facts).into();
    assert_eq!(back.fat_g, Some(Decimal::ZERO));
    assert!(!back.is_unknown());
}

#[test]
fn extra_nutrients_survive_the_round_trip() {
    let mut facts = NutritionFacts {
        basis: Some(per_100g()),
        ..Default::default()
    };
    facts
        .extra
        .insert("vitamin_c_mg".to_owned(), Decimal::new(125, 1));
    let back: NutritionFacts = NutritionDto::from(facts).into();
    assert_eq!(back.extra.get("vitamin_c_mg"), Some(&Decimal::new(125, 1)));
}

#[test]
fn absent_fields_deserialise_as_unknown() {
    let dto: NutritionDto = serde_json::from_str("{}").unwrap();
    assert_eq!(dto.energy_kcal, None);
    let facts: NutritionFacts = dto.into();
    assert!(facts.is_unknown());
}
