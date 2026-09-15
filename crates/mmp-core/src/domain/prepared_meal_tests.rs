use super::*;
use crate::domain::ingredient::MAX_NAME_LEN;

fn new_prepared_meal(name: &str) -> NewPreparedMeal {
    NewPreparedMeal {
        id: None,
        name: name.to_owned(),
        default_unit: Unit::Item,
        shopping_section: None,
        track_stock: None,
        provenance: Provenance::local(),
    }
}

#[test]
fn a_blank_name_is_rejected() {
    assert!(new_prepared_meal("   ").validate().is_err());
}

#[test]
fn an_over_long_name_is_rejected() {
    assert!(
        new_prepared_meal(&"a".repeat(MAX_NAME_LEN + 1))
            .validate()
            .is_err()
    );
}

#[test]
fn a_minimal_prepared_meal_is_valid() {
    assert!(new_prepared_meal("Frozen lasagne").validate().is_ok());
}

#[test]
fn an_empty_patch_is_detected() {
    assert!(PreparedMealPatch::default().is_empty());
}
