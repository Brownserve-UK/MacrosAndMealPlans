use std::collections::HashMap;

use rust_decimal::Decimal;
use time::macros::datetime;

use super::{RecipeFulfilments, expand_recipe};
use crate::domain::{
    ConsumedAmount, DeductionCandidates, DemandGap, DemandSubject, IngredientId, NutritionFacts,
    Product, ProductId, Provenance, Quantity, Recipe, RecipeComponent, RecipeComponentId, RecipeId,
    RecipeRequirement, RecipeVisibility, Revision, Unit, UserId,
};

fn d(value: i64) -> Decimal {
    Decimal::from(value)
}

fn product(name: &str) -> Product {
    Product {
        id: ProductId::new(),
        name: name.to_owned(),
        brand: None,
        barcode: None,
        retailer: None,
        shopping_section: None,
        track_stock: None,
        package_quantity: None,
        servings_per_pack: None,
        mapped_ingredient_id: None,
        nutrition: NutritionFacts::default(),
        provenance: Provenance::local(),
        revision: Revision::INITIAL,
        created_at: datetime!(2026-09-04 09:00 UTC),
        updated_at: datetime!(2026-09-04 09:00 UTC),
        archived_at: None,
    }
}

fn packaged(name: &str, pack: Quantity, servings_per_pack: Option<i32>) -> Product {
    Product {
        package_quantity: Some(pack),
        servings_per_pack,
        ..product(name)
    }
}

fn grams(value: i64) -> ConsumedAmount {
    ConsumedAmount::Measure(Quantity::new(d(value), Unit::Gram))
}

fn component(requirement: RecipeRequirement, amount: ConsumedAmount) -> RecipeComponent {
    RecipeComponent {
        id: RecipeComponentId::new(),
        requirement,
        source_text: None,
        amount,
        position: 0,
    }
}

fn recipe(servings: i32, components: Vec<RecipeComponent>) -> Recipe {
    let owner = UserId::new();
    Recipe {
        id: RecipeId::new(),
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
        photo_version: None,
        owner_id: owner,
        visibility: RecipeVisibility::Private,
        created_by: owner,
        updated_by: owner,
        revision: Revision::INITIAL,
        created_at: datetime!(2026-09-04 09:00 UTC),
        updated_at: datetime!(2026-09-04 09:00 UTC),
        archived_at: None,
    }
}

fn fulfilments(
    pinned: Vec<Product>,
    pools: Vec<(IngredientId, Vec<Product>)>,
) -> RecipeFulfilments {
    RecipeFulfilments {
        pinned: pinned
            .into_iter()
            .map(|product| (product.id, product))
            .collect(),
        candidates: pools.into_iter().collect::<HashMap<_, _>>(),
        empty: Vec::new(),
    }
}

fn nothing() -> RecipeFulfilments {
    fulfilments(vec![], vec![])
}

#[test]
fn a_pinned_product_becomes_a_want_against_that_product() {
    let milk = product("Milk");
    let recipe = recipe(
        1,
        vec![component(
            RecipeRequirement::Product {
                product_id: milk.id,
            },
            grams(200),
        )],
    );

    let out = expand_recipe(&recipe, d(1), &fulfilments(vec![milk.clone()], vec![]));

    assert!(out.subject_gaps.is_empty());
    assert!(out.loose_gaps.is_empty());
    assert_eq!(out.wants.len(), 1);
    assert_eq!(out.wants[0].want, Quantity::new(d(200), Unit::Gram));
    assert_eq!(out.wants[0].target.subject, DemandSubject::product(milk.id));
    assert_eq!(
        out.wants[0].target.candidates,
        DeductionCandidates::Products(vec![milk.id])
    );
}

#[test]
fn servings_scale_the_amounts_the_recipe_asks_for() {
    let milk = product("Milk");
    let recipe = recipe(
        2,
        vec![component(
            RecipeRequirement::Product {
                product_id: milk.id,
            },
            grams(200),
        )],
    );

    let out = expand_recipe(&recipe, d(3), &fulfilments(vec![milk.clone()], vec![]));

    assert_eq!(out.wants[0].want, Quantity::new(d(300), Unit::Gram));
}

#[test]
fn a_recipe_claiming_no_servings_is_scaled_as_though_it_made_one() {
    let milk = product("Milk");
    let recipe = recipe(
        0,
        vec![component(
            RecipeRequirement::Product {
                product_id: milk.id,
            },
            grams(200),
        )],
    );

    let out = expand_recipe(&recipe, d(2), &fulfilments(vec![milk.clone()], vec![]));

    assert_eq!(out.wants[0].want, Quantity::new(d(400), Unit::Gram));
}

#[test]
fn an_ingredient_becomes_one_want_against_the_whole_pool() {
    let ingredient = IngredientId::new();
    let first = product("Own Brand Milk");
    let second = product("Value Milk");
    let recipe = recipe(
        1,
        vec![component(
            RecipeRequirement::Ingredient {
                ingredient_id: ingredient,
            },
            grams(500),
        )],
    );

    let out = expand_recipe(
        &recipe,
        d(1),
        &fulfilments(
            vec![],
            vec![(ingredient, vec![first.clone(), second.clone()])],
        ),
    );

    assert_eq!(out.wants.len(), 1);
    assert_eq!(
        out.wants[0].target.subject,
        DemandSubject::ingredient(ingredient)
    );
    assert_eq!(
        out.wants[0].target.candidates,
        DeductionCandidates::Products(vec![first.id, second.id])
    );
}

#[test]
fn an_unresolved_line_is_a_gap_that_belongs_to_no_subject() {
    let recipe = recipe(
        1,
        vec![component(
            RecipeRequirement::Unresolved {
                text: "a handful of parsley".to_owned(),
            },
            grams(10),
        )],
    );

    let out = expand_recipe(&recipe, d(1), &nothing());

    assert!(out.wants.is_empty());
    assert!(out.subject_gaps.is_empty());
    assert_eq!(out.loose_gaps, vec![DemandGap::UnresolvedRecipeLine]);
}

#[test]
fn a_pinned_product_we_could_not_load_reports_a_missing_product() {
    let missing = ProductId::new();
    let recipe = recipe(
        1,
        vec![component(
            RecipeRequirement::Product {
                product_id: missing,
            },
            grams(200),
        )],
    );

    let out = expand_recipe(&recipe, d(1), &nothing());

    assert!(out.wants.is_empty());
    assert_eq!(
        out.subject_gaps,
        vec![(DemandSubject::product(missing), DemandGap::ProductMissing)]
    );
}

#[test]
fn an_ingredient_with_no_products_behind_it_reports_a_gap() {
    let ingredient = IngredientId::new();
    let recipe = recipe(
        1,
        vec![component(
            RecipeRequirement::Ingredient {
                ingredient_id: ingredient,
            },
            grams(500),
        )],
    );

    let out = expand_recipe(
        &recipe,
        d(1),
        &fulfilments(vec![], vec![(ingredient, vec![])]),
    );

    assert!(out.wants.is_empty());
    assert_eq!(
        out.subject_gaps,
        vec![(
            DemandSubject::ingredient(ingredient),
            DemandGap::IngredientHasNoProducts
        )]
    );
}

#[test]
fn an_ingredient_asked_for_in_servings_cannot_be_turned_into_a_measure() {
    let ingredient = IngredientId::new();
    let recipe = recipe(
        1,
        vec![component(
            RecipeRequirement::Ingredient {
                ingredient_id: ingredient,
            },
            ConsumedAmount::Servings(d(2)),
        )],
    );

    let out = expand_recipe(
        &recipe,
        d(1),
        &fulfilments(vec![], vec![(ingredient, vec![product("Milk")])]),
    );

    assert!(out.wants.is_empty());
    assert_eq!(
        out.subject_gaps,
        vec![(
            DemandSubject::ingredient(ingredient),
            DemandGap::AmountUnresolvable
        )]
    );
}

#[test]
fn a_pinned_product_with_no_pack_size_cannot_answer_a_request_in_packs() {
    let milk = product("Milk");
    let recipe = recipe(
        1,
        vec![component(
            RecipeRequirement::Product {
                product_id: milk.id,
            },
            ConsumedAmount::Packs(d(2)),
        )],
    );

    let out = expand_recipe(&recipe, d(1), &fulfilments(vec![milk.clone()], vec![]));

    assert!(out.wants.is_empty());
    assert_eq!(
        out.subject_gaps,
        vec![(
            DemandSubject::product(milk.id),
            DemandGap::AmountUnresolvable
        )]
    );
}

#[test]
fn packs_of_a_pinned_product_resolve_through_its_pack_size_and_scale() {
    let milk = packaged("Milk", Quantity::new(d(1000), Unit::Millilitre), None);
    let recipe = recipe(
        2,
        vec![component(
            RecipeRequirement::Product {
                product_id: milk.id,
            },
            ConsumedAmount::Packs(d(1)),
        )],
    );

    let out = expand_recipe(&recipe, d(4), &fulfilments(vec![milk.clone()], vec![]));

    assert_eq!(out.wants[0].want, Quantity::new(d(2000), Unit::Millilitre));
}

#[test]
fn every_component_is_expanded_and_gaps_do_not_stop_the_rest() {
    let milk = product("Milk");
    let missing = ProductId::new();
    let recipe = recipe(
        1,
        vec![
            component(
                RecipeRequirement::Product {
                    product_id: missing,
                },
                grams(100),
            ),
            component(
                RecipeRequirement::Product {
                    product_id: milk.id,
                },
                grams(200),
            ),
            component(
                RecipeRequirement::Unresolved {
                    text: "salt".to_owned(),
                },
                grams(1),
            ),
        ],
    );

    let out = expand_recipe(&recipe, d(1), &fulfilments(vec![milk.clone()], vec![]));

    assert_eq!(out.wants.len(), 1);
    assert_eq!(out.wants[0].want, Quantity::new(d(200), Unit::Gram));
    assert_eq!(out.subject_gaps.len(), 1);
    assert_eq!(out.loose_gaps, vec![DemandGap::UnresolvedRecipeLine]);
}
