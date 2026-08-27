use super::*;
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
        product_id: ProductId::new(),
        amount: ConsumedAmount::Servings(Decimal::ZERO),
    }];
    assert!(validate_components(&components).is_err());
}
