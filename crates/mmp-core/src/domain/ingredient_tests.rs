use super::*;
fn new_ingredient(name: &str) -> NewIngredient {
    NewIngredient {
        id: None,
        name: name.to_owned(),
        default_unit: Unit::Gram,
        provenance: Provenance::local(),
    }
}

#[test]
fn a_blank_name_is_rejected() {
    assert!(new_ingredient("   ").validate().is_err());
}

#[test]
fn an_over_long_name_is_rejected() {
    assert!(
        new_ingredient(&"a".repeat(MAX_NAME_LEN + 1))
            .validate()
            .is_err()
    );
}

#[test]
fn a_minimal_ingredient_is_valid() {
    assert!(new_ingredient("Coriander").validate().is_ok());
}

#[test]
fn an_empty_patch_is_detected() {
    assert!(IngredientPatch::default().is_empty());
}
