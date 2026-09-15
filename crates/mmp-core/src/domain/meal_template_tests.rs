use super::*;
use crate::domain::{ProductId, Quantity, RecipeId, Unit};

fn new_template(components: Vec<NewMealTemplateComponent>) -> NewMealTemplate {
    NewMealTemplate {
        id: None,
        owner_id: UserId::new(),
        name: "Fish fingers, chips and peas".to_owned(),
        components,
    }
}

fn measured(product_id: ProductId, grams: i64) -> NewMealTemplateComponent {
    NewMealTemplateComponent {
        item: MealItemRef::product(product_id),
        amount: ConsumedAmount::Measure(Quantity::new(
            rust_decimal::Decimal::new(grams, 0),
            Unit::Gram,
        )),
    }
}

#[test]
fn a_blank_name_is_rejected() {
    let mut template = new_template(vec![measured(ProductId::new(), 100)]);
    template.name = "   ".to_owned();
    assert!(template.validate().is_err());
}

#[test]
fn at_least_one_component_is_required() {
    let template = new_template(vec![]);
    assert!(template.validate().is_err());
}

#[test]
fn a_well_formed_template_validates() {
    let template = new_template(vec![measured(ProductId::new(), 100)]);
    assert!(template.validate().is_ok());
}

#[test]
fn a_recipe_component_must_be_measured_in_servings() {
    let template = new_template(vec![NewMealTemplateComponent {
        item: MealItemRef::recipe(RecipeId::new()),
        amount: ConsumedAmount::Measure(Quantity::new(
            rust_decimal::Decimal::new(100, 0),
            Unit::Gram,
        )),
    }]);
    assert!(template.validate().is_err());
}

#[test]
fn a_dish_component_is_rejected() {
    let template = new_template(vec![NewMealTemplateComponent {
        item: MealItemRef::dish(RecipeId::new()),
        amount: ConsumedAmount::Servings(rust_decimal::Decimal::ONE),
    }]);
    assert!(
        template.validate().is_err(),
        "a saved meal cannot hold cooked food already in the house"
    );
}

#[test]
fn a_generic_ingredient_cannot_be_measured_in_servings() {
    let template = new_template(vec![NewMealTemplateComponent {
        item: MealItemRef::ingredient(crate::domain::IngredientId::new()),
        amount: ConsumedAmount::Servings(rust_decimal::Decimal::ONE),
    }]);
    assert!(template.validate().is_err());
}

#[test]
fn make_components_assigns_positions_in_order() {
    let a = ProductId::new();
    let b = ProductId::new();
    let components = make_components(vec![measured(a, 100), measured(b, 50)]);
    assert_eq!(components[0].position, 0);
    assert_eq!(components[1].position, 1);
    assert_eq!(components[0].item, MealItemRef::product(a));
    assert_eq!(components[1].item, MealItemRef::product(b));
}
