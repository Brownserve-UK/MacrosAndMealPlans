use super::*;
use crate::domain::{MealItemRef, ProductId, RecipeId};
use rust_decimal::Decimal;

#[test]
fn slot_codes_round_trip() {
    for slot in MealSlot::ALL {
        assert_eq!(MealSlot::from_str(slot.code()).unwrap(), slot);
    }
}

#[test]
fn status_codes_round_trip() {
    for status in MealPlanStatus::ALL {
        assert_eq!(MealPlanStatus::from_str(status.code()).unwrap(), status);
    }
}

#[test]
fn a_meal_needs_a_component() {
    assert!(validate_components(&[]).is_err());
}

#[test]
fn a_component_needs_a_positive_amount() {
    let components = vec![NewMealPlanComponent {
        id: None,
        item: MealItemRef::product(ProductId::new()),
        amount: ConsumedAmount::Servings(Decimal::ZERO),
    }];
    assert!(validate_components(&components).is_err());
}

#[test]
fn a_recipe_component_must_be_measured_in_servings() {
    let grams = ConsumedAmount::Measure(crate::domain::Quantity::new(
        Decimal::new(100, 0),
        crate::domain::Unit::Gram,
    ));
    let recipe_component = vec![NewMealPlanComponent {
        id: None,
        item: MealItemRef::recipe(RecipeId::new()),
        amount: grams,
    }];
    assert!(validate_components(&recipe_component).is_err());

    let product_component = vec![NewMealPlanComponent {
        id: None,
        item: MealItemRef::product(ProductId::new()),
        amount: grams,
    }];
    assert!(validate_components(&product_component).is_ok());
}
