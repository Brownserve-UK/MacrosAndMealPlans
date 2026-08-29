use rust_decimal::Decimal;
use time::macros::datetime;

use crate::domain::{
    ConsumedAmount, Fulfilment, NewRecipe, NewRecipeComponent, NutritionFacts, NutritionQuality,
    Product, ProductId, Provenance, Quantity, RecipeRequirement, Revision, Unit, UserId,
    recipe_nutrition, recipe_nutrition_detailed,
};

fn d(value: i64) -> Decimal {
    Decimal::from(value)
}

fn product_per_100g(energy: i64, protein: i64) -> Product {
    Product {
        id: ProductId::new(),
        name: "Test".to_owned(),
        brand: None,
        barcode: None,
        retailer: None,
        shopping_section: None,
        package_quantity: None,
        servings_per_pack: None,
        mapped_ingredient_id: None,
        nutrition: NutritionFacts {
            basis: Some(Quantity::new(d(100), Unit::Gram)),
            energy_kcal: Some(d(energy)),
            protein_g: Some(d(protein)),
            carbohydrate_g: Some(d(1)),
            sugar_g: Some(d(1)),
            fat_g: Some(d(1)),
            saturated_fat_g: Some(d(1)),
            fibre_g: Some(d(1)),
            salt_g: Some(d(1)),
            cholesterol_mg: Some(d(1)),
            extra: Default::default(),
        },
        provenance: Provenance::local(),
        revision: Revision::INITIAL,
        created_at: datetime!(2026-08-22 09:00 UTC),
        updated_at: datetime!(2026-08-22 09:00 UTC),
        archived_at: None,
    }
}

fn grams(value: i64) -> ConsumedAmount {
    ConsumedAmount::Measure(Quantity::new(d(value), Unit::Gram))
}

fn new_recipe(servings: i32, components: Vec<NewRecipeComponent>) -> NewRecipe {
    NewRecipe {
        id: None,
        name: "Test Recipe".to_owned(),
        description: None,
        servings,
        preparation_minutes: None,
        cooking_minutes: None,
        notes: None,
        components,
        instructions: vec![],
        meal_categories: vec![],
        country_categories: vec![],
        tags: vec![],
        actor_id: UserId::new(),
    }
}

fn component(product_id: ProductId, amount: ConsumedAmount) -> NewRecipeComponent {
    NewRecipeComponent {
        id: None,
        requirement: RecipeRequirement::Product { product_id },
        amount,
    }
}

#[test]
fn aggregates_and_divides_by_servings() {
    let a = product_per_100g(200, 10);
    let b = product_per_100g(100, 5);

    // 200g of A => 400 kcal / 20 protein, 100g of B => 100 kcal / 5 protein.
    // Total 500 kcal / 25 protein across 4 servings => 125 kcal / 6.25 protein per serving.
    let nutrition = recipe_nutrition(
        [
            (&grams(200), Fulfilment::Pinned(&a)),
            (&grams(100), Fulfilment::Pinned(&b)),
        ],
        4,
    );

    assert_eq!(nutrition.facts.energy_kcal, Some(d(125)));
    assert_eq!(nutrition.facts.protein_g, Some(Decimal::new(625, 2)));
    assert_eq!(nutrition.quality, NutritionQuality::Known);
}

#[test]
fn unresolved_line_drags_quality_to_partial() {
    let a = product_per_100g(200, 10);

    let nutrition = recipe_nutrition(
        [
            (&grams(100), Fulfilment::Pinned(&a)),
            (&grams(50), Fulfilment::None),
        ],
        1,
    );

    // The resolved line still contributes; the unresolved one is not guessed.
    assert_eq!(nutrition.facts.energy_kcal, Some(d(200)));
    assert_eq!(nutrition.quality, NutritionQuality::Partial);
}

#[test]
fn all_unresolved_is_unknown() {
    let nutrition = recipe_nutrition([(&grams(100), Fulfilment::None)], 2);
    assert_eq!(nutrition.quality, NutritionQuality::Unknown);
    assert_eq!(nutrition.facts.energy_kcal, None);
}

#[test]
fn detailed_reports_each_line_quality_in_order() {
    let a = product_per_100g(200, 10);

    let detailed = recipe_nutrition_detailed(
        [
            (&grams(100), Fulfilment::Pinned(&a)),
            (&grams(50), Fulfilment::None),
        ],
        1,
    );

    assert_eq!(detailed.consumed.quality, NutritionQuality::Partial);
    assert_eq!(
        detailed.line_qualities,
        vec![NutritionQuality::Known, NutritionQuality::Unknown],
    );
}

#[test]
fn rejects_a_blank_name() {
    let recipe = NewRecipe {
        name: "   ".to_owned(),
        ..new_recipe(2, vec![component(ProductId::new(), grams(100))])
    };
    assert!(recipe.validate().is_err());
}

#[test]
fn rejects_non_positive_servings() {
    let recipe = new_recipe(0, vec![component(ProductId::new(), grams(100))]);
    assert!(recipe.validate().is_err());
}

#[test]
fn rejects_no_components() {
    let recipe = new_recipe(2, vec![]);
    assert!(recipe.validate().is_err());
}

#[test]
fn rejects_a_non_positive_amount() {
    let recipe = new_recipe(2, vec![component(ProductId::new(), grams(0))]);
    assert!(recipe.validate().is_err());
}

#[test]
fn quality_rolls_up_by_worst_case_precedence() {
    use NutritionQuality::{Estimated, Known, Partial, Unknown};

    let cases = [
        (vec![], Unknown),
        (vec![Unknown, Unknown], Unknown),
        (vec![Known, Known], Known),
        (vec![Known, Estimated], Estimated),
        (vec![Estimated, Estimated], Estimated),
        (vec![Known, Partial], Partial),
        (vec![Estimated, Partial], Partial),
        (vec![Estimated, Unknown], Partial),
        (vec![Known, Unknown], Partial),
        (vec![Known, Estimated, Partial], Partial),
    ];

    for (input, expected) in cases {
        assert_eq!(super::rollup_quality(input.clone()), expected, "{input:?}");
    }
}
