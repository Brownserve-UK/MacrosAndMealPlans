use super::*;
use std::cmp::Ordering;

fn q(amount: i64, scale: u32, unit: Unit) -> Quantity {
    Quantity::new(Decimal::new(amount, scale), unit)
}

#[test]
fn converts_within_mass() {
    let converted = q(2, 0, Unit::Kilogram).convert_to(Unit::Gram).unwrap();
    assert_eq!(converted.amount, Decimal::new(2000, 0));
    assert_eq!(converted.unit, Unit::Gram);
}

#[test]
fn converts_within_volume() {
    let converted = q(15, 1, Unit::Litre).convert_to(Unit::Millilitre).unwrap();
    assert_eq!(converted.amount, Decimal::new(1500, 0));
}

#[test]
fn converts_cooking_volumes() {
    assert_eq!(
        q(1, 0, Unit::Tablespoon)
            .convert_to(Unit::Teaspoon)
            .unwrap()
            .amount,
        Decimal::new(3, 0)
    );
}

#[test]
fn refuses_mass_to_volume() {
    let err = q(100, 0, Unit::Gram)
        .convert_to(Unit::Millilitre)
        .unwrap_err();
    assert!(matches!(
        err,
        ConversionError::IncompatibleDimensions { .. }
    ));
}

#[test]
fn refuses_count_to_count_as_contextual() {
    let err = q(1, 0, Unit::Bunch).convert_to(Unit::Item).unwrap_err();
    assert!(matches!(
        err,
        ConversionError::ContextualConversionRequired { .. }
    ));
}

#[test]
fn identical_count_units_need_no_conversion() {
    let same = q(3, 0, Unit::Clove).convert_to(Unit::Clove).unwrap();
    assert_eq!(same.amount, Decimal::new(3, 0));
}

#[test]
fn compares_across_compatible_units() {
    let ordering = q(1, 0, Unit::Kilogram)
        .compare(&q(999, 0, Unit::Gram))
        .unwrap();
    assert_eq!(ordering, Ordering::Greater);
}

#[test]
fn comparison_is_unknown_without_a_safe_conversion() {
    assert!(q(85, 0, Unit::Gram).compare(&q(1, 0, Unit::Bunch)).is_err());
}

#[test]
fn unit_codes_round_trip() {
    for unit in Unit::ALL {
        assert_eq!(Unit::from_str(unit.code()).unwrap(), unit);
    }
}

#[test]
fn serde_representation_matches_the_unit_code() {
    for unit in Unit::ALL {
        let json = serde_json::to_string(&unit).unwrap();
        assert_eq!(json, format!("\"{}\"", unit.code()), "{unit:?}");
        let back: Unit = serde_json::from_str(&json).unwrap();
        assert_eq!(back, unit);
    }
}

#[test]
fn count_units_have_no_base_factor() {
    for unit in Unit::ALL {
        assert_eq!(
            unit.base_factor().is_none(),
            unit.dimension() == Dimension::Count
        );
    }
}
