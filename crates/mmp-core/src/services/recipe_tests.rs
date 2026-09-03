use std::sync::Arc;

use rust_decimal::Decimal;
use time::OffsetDateTime;
use time::macros::datetime;

use crate::CoreError;
use crate::domain::{
    ConsumedAmount, Ingredient, IngredientId, MealCategory, NewRecipe, NewRecipeComponent,
    NewRecipeInstruction, NutritionFacts, NutritionQuality, Product, ProductId, Provenance,
    Quantity, RecipePatch, RecipePhotoDerivatives, RecipeRequirement, RecipeVisibility, Revision,
    Unit, UserId,
};
use crate::ports::{FixedClock, PageRequest, RecipeQuery, SortDirection};
use crate::services::{NutritionGapReason, RecipeService, ResolveRequirement};
use crate::testing::{
    InMemoryIngredientRepository, InMemoryProductRepository, InMemoryRecipeRepository,
};

struct Harness {
    service: RecipeService,
    products: InMemoryProductRepository,
    ingredients: InMemoryIngredientRepository,
    recipes: InMemoryRecipeRepository,
}

fn harness() -> Harness {
    harness_at(datetime!(2026-08-22 09:00 UTC))
}

fn harness_at(now: OffsetDateTime) -> Harness {
    let products = InMemoryProductRepository::new();
    let ingredients = InMemoryIngredientRepository::new();
    let recipes = InMemoryRecipeRepository::new();
    let service = RecipeService::new(
        Arc::new(recipes.clone()),
        Arc::new(products.clone()),
        Arc::new(ingredients.clone()),
        Arc::new(FixedClock::new(now)),
    );
    Harness {
        service,
        products,
        ingredients,
        recipes,
    }
}

fn seed_ingredient(ingredients: &InMemoryIngredientRepository) -> IngredientId {
    let id = IngredientId::new();
    ingredients.seed(Ingredient {
        id,
        name: "Rolled Oats".to_owned(),
        default_unit: Unit::Gram,
        shopping_section: None,
        track_stock: None,
        provenance: Provenance::local(),
        revision: Revision::INITIAL,
        created_at: datetime!(2026-08-22 09:00 UTC),
        updated_at: datetime!(2026-08-22 09:00 UTC),
        archived_at: None,
    });
    id
}

fn d(value: i64) -> Decimal {
    Decimal::from(value)
}

fn seed_product_mapped(
    products: &InMemoryProductRepository,
    energy: i64,
    ingredient_id: IngredientId,
) -> ProductId {
    seed_product_with(products, energy, Some(ingredient_id))
}

fn seed_product(products: &InMemoryProductRepository, energy: i64) -> ProductId {
    seed_product_with(products, energy, None)
}

fn seed_product_with(
    products: &InMemoryProductRepository,
    energy: i64,
    mapped_ingredient_id: Option<IngredientId>,
) -> ProductId {
    let id = ProductId::new();
    products.seed(Product {
        id,
        name: "Test".to_owned(),
        brand: None,
        barcode: None,
        retailer: None,
        shopping_section: None,
        track_stock: None,
        package_quantity: None,
        servings_per_pack: None,
        mapped_ingredient_id,
        nutrition: NutritionFacts {
            basis: Some(Quantity::new(d(100), Unit::Gram)),
            energy_kcal: Some(d(energy)),
            protein_g: Some(d(1)),
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
    });
    id
}

fn grams(value: i64) -> ConsumedAmount {
    ConsumedAmount::Measure(Quantity::new(d(value), Unit::Gram))
}

fn component(product_id: ProductId) -> NewRecipeComponent {
    NewRecipeComponent {
        id: None,
        requirement: RecipeRequirement::Product { product_id },
        amount: grams(100),
    }
}

fn new_recipe(actor: UserId, components: Vec<NewRecipeComponent>) -> NewRecipe {
    NewRecipe {
        id: None,
        name: "Soup".to_owned(),
        description: None,
        servings: 2,
        preparation_minutes: None,
        cooking_minutes: None,
        notes: None,
        components,
        instructions: vec![],
        meal_categories: vec![],
        country_categories: vec![],
        tags: vec![],
        actor_id: actor,
    }
}

fn query_for(owner: UserId) -> RecipeQuery {
    RecipeQuery {
        owner_id: owner,
        search: None,
        include_archived: false,
        page: PageRequest::default(),
        sort: SortDirection::Ascending,
    }
}

#[tokio::test]
async fn creates_a_recipe_owned_by_the_actor() {
    let h = harness();
    let actor = UserId::new();
    let product = seed_product(&h.products, 200);

    let recipe = h
        .service
        .create_recipe(new_recipe(actor, vec![component(product)]))
        .await
        .unwrap();

    assert_eq!(recipe.owner_id, actor);
    assert_eq!(recipe.revision, Revision::INITIAL);
    assert_eq!(recipe.components.len(), 1);
    assert_eq!(h.recipes.count(), 1);
}

#[tokio::test]
async fn lists_unmapped_ingredients_used_by_visible_recipes() {
    let h = harness();
    let viewer = UserId::new();
    let other = UserId::new();
    let own_ingredient = seed_ingredient(&h.ingredients);
    let shared_ingredient = IngredientId::new();
    h.ingredients.seed(Ingredient {
        id: shared_ingredient,
        name: "Cinnamon".to_owned(),
        default_unit: Unit::Gram,
        shopping_section: None,
        track_stock: None,
        provenance: Provenance::local(),
        revision: Revision::INITIAL,
        created_at: datetime!(2026-08-22 09:00 UTC),
        updated_at: datetime!(2026-08-22 09:00 UTC),
        archived_at: None,
    });
    let own = h
        .service
        .create_recipe(new_recipe(
            viewer,
            vec![ingredient_component(own_ingredient)],
        ))
        .await
        .unwrap();
    let mut shared = h
        .service
        .create_recipe(new_recipe(
            other,
            vec![ingredient_component(shared_ingredient)],
        ))
        .await
        .unwrap();
    shared.visibility = RecipeVisibility::Shared;
    h.recipes.seed(shared);
    h.recipes.seed(own);

    let result = h
        .service
        .ingredients_needing_products(viewer, false)
        .await
        .unwrap();

    assert_eq!(
        result
            .iter()
            .map(|ingredient| ingredient.id)
            .collect::<Vec<_>>(),
        vec![shared_ingredient, own_ingredient]
    );
}

#[tokio::test]
async fn excludes_mapped_ingredients_and_private_recipes_from_review() {
    let h = harness();
    let viewer = UserId::new();
    let other = UserId::new();
    let mapped = seed_ingredient(&h.ingredients);
    seed_product_mapped(&h.products, 100, mapped);
    h.service
        .create_recipe(new_recipe(viewer, vec![ingredient_component(mapped)]))
        .await
        .unwrap();
    let private = IngredientId::new();
    h.ingredients.seed(Ingredient {
        id: private,
        name: "Private".to_owned(),
        default_unit: Unit::Gram,
        shopping_section: None,
        track_stock: None,
        provenance: Provenance::local(),
        revision: Revision::INITIAL,
        created_at: datetime!(2026-08-22 09:00 UTC),
        updated_at: datetime!(2026-08-22 09:00 UTC),
        archived_at: None,
    });
    h.service
        .create_recipe(new_recipe(other, vec![ingredient_component(private)]))
        .await
        .unwrap();

    let result = h
        .service
        .ingredients_needing_products(viewer, false)
        .await
        .unwrap();

    assert!(result.is_empty());
}

#[tokio::test]
async fn rejects_an_unknown_product() {
    let h = harness();
    let actor = UserId::new();

    let err = h
        .service
        .create_recipe(new_recipe(actor, vec![component(ProductId::new())]))
        .await
        .unwrap_err();

    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn normalises_metadata_and_preserves_instruction_order() {
    let h = harness();
    let actor = UserId::new();
    let product = seed_product(&h.products, 200);
    let mut input = new_recipe(actor, vec![component(product)]);
    input.description = Some("  Warming soup  ".to_owned());
    input.notes = Some("   ".to_owned());
    input.preparation_minutes = Some(10);
    input.cooking_minutes = Some(20);
    input.instructions = vec![
        NewRecipeInstruction {
            id: None,
            text: "  Chop  ".to_owned(),
        },
        NewRecipeInstruction {
            id: None,
            text: "Cook".to_owned(),
        },
    ];
    input.meal_categories = vec![MealCategory::Lunch, MealCategory::Lunch];
    input.country_categories = vec!["GB".to_owned(), "GB".to_owned()];
    input.tags = vec![
        " Quick ".to_owned(),
        "quick".to_owned(),
        "Family".to_owned(),
    ];

    let recipe = h.service.create_recipe(input).await.unwrap();

    assert_eq!(recipe.description.as_deref(), Some("Warming soup"));
    assert_eq!(recipe.notes, None);
    assert_eq!(recipe.instructions[0].text, "Chop");
    assert_eq!(recipe.instructions[1].position, 1);
    assert_eq!(recipe.meal_categories, vec![MealCategory::Lunch]);
    assert_eq!(recipe.country_categories, vec!["GB"]);
    assert_eq!(recipe.tags, vec!["Quick", "Family"]);
}

#[tokio::test]
async fn photo_changes_increment_recipe_and_photo_revisions() {
    let h = harness();
    let actor = UserId::new();
    let product = seed_product(&h.products, 200);
    let recipe = h
        .service
        .create_recipe(new_recipe(actor, vec![component(product)]))
        .await
        .unwrap();
    let derivatives = RecipePhotoDerivatives {
        hero_jpeg: vec![1],
        card_jpeg: vec![2],
        hero_width: 10,
        hero_height: 5,
        card_width: 10,
        card_height: 5,
    };

    let with_photo = h
        .service
        .replace_photo(recipe.id, recipe.revision, derivatives.clone(), actor)
        .await
        .unwrap();
    assert_eq!(with_photo.revision, Revision::new(2));
    assert_eq!(with_photo.photo_version, Some(1));

    let replaced = h
        .service
        .replace_photo(with_photo.id, with_photo.revision, derivatives, actor)
        .await
        .unwrap();
    assert_eq!(replaced.revision, Revision::new(3));
    assert_eq!(replaced.photo_version, Some(2));

    let deleted = h
        .service
        .delete_photo(replaced.id, replaced.revision, actor)
        .await
        .unwrap();
    assert_eq!(deleted.revision, Revision::new(4));
    assert_eq!(deleted.photo_version, None);
}

#[tokio::test]
async fn a_non_owner_cannot_read_a_recipe() {
    let h = harness();
    let owner = UserId::new();
    let stranger = UserId::new();
    let product = seed_product(&h.products, 200);

    let recipe = h
        .service
        .create_recipe(new_recipe(owner, vec![component(product)]))
        .await
        .unwrap();

    let err = h.service.get_recipe(recipe.id, stranger).await.unwrap_err();
    assert!(matches!(err, CoreError::NotFound { .. }));
}

#[tokio::test]
async fn listing_is_scoped_to_the_owner() {
    let h = harness();
    let owner = UserId::new();
    let other = UserId::new();
    let product = seed_product(&h.products, 200);

    h.service
        .create_recipe(new_recipe(owner, vec![component(product)]))
        .await
        .unwrap();
    h.service
        .create_recipe(new_recipe(other, vec![component(product)]))
        .await
        .unwrap();

    let page = h.service.list_recipes(&query_for(owner)).await.unwrap();
    assert_eq!(page.total, 1);
}

#[tokio::test]
async fn stale_updates_are_rejected() {
    let h = harness();
    let actor = UserId::new();
    let product = seed_product(&h.products, 200);
    let recipe = h
        .service
        .create_recipe(new_recipe(actor, vec![component(product)]))
        .await
        .unwrap();

    let patch = RecipePatch {
        name: Some("Stew".to_owned()),
        ..RecipePatch::default()
    };
    let err = h
        .service
        .update_recipe(recipe.id, Revision::new(99), patch, actor)
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::RevisionMismatch { .. }));
}

#[tokio::test]
async fn reordering_preserves_component_ids() {
    let h = harness();
    let actor = UserId::new();
    let first = seed_product(&h.products, 200);
    let second = seed_product(&h.products, 100);
    let recipe = h
        .service
        .create_recipe(new_recipe(actor, vec![component(first), component(second)]))
        .await
        .unwrap();
    let first_id = recipe.components[0].id;
    let second_id = recipe.components[1].id;

    let patch = RecipePatch {
        components: Some(vec![
            NewRecipeComponent {
                id: Some(second_id),
                requirement: RecipeRequirement::Product { product_id: second },
                amount: grams(100),
            },
            NewRecipeComponent {
                id: Some(first_id),
                requirement: RecipeRequirement::Product { product_id: first },
                amount: grams(100),
            },
        ]),
        ..RecipePatch::default()
    };
    let updated = h
        .service
        .update_recipe(recipe.id, recipe.revision, patch, actor)
        .await
        .unwrap();

    assert_eq!(updated.components[0].id, second_id);
    assert_eq!(updated.components[0].position, 0);
    assert_eq!(updated.components[1].id, first_id);
    assert_eq!(updated.components[1].position, 1);
}

#[tokio::test]
async fn rejects_a_component_id_from_another_recipe() {
    let h = harness();
    let actor = UserId::new();
    let product = seed_product(&h.products, 200);
    let recipe = h
        .service
        .create_recipe(new_recipe(actor, vec![component(product)]))
        .await
        .unwrap();

    let patch = RecipePatch {
        components: Some(vec![NewRecipeComponent {
            id: Some(crate::domain::RecipeComponentId::new()),
            requirement: RecipeRequirement::Product {
                product_id: product,
            },
            amount: grams(100),
        }]),
        ..RecipePatch::default()
    };
    let err = h
        .service
        .update_recipe(recipe.id, recipe.revision, patch, actor)
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn archives_and_unarchives() {
    let h = harness();
    let actor = UserId::new();
    let product = seed_product(&h.products, 200);
    let recipe = h
        .service
        .create_recipe(new_recipe(actor, vec![component(product)]))
        .await
        .unwrap();

    let archived = h
        .service
        .set_recipe_archived(recipe.id, recipe.revision, true, actor)
        .await
        .unwrap();
    assert!(archived.is_archived());

    let unarchived = h
        .service
        .set_recipe_archived(archived.id, archived.revision, false, actor)
        .await
        .unwrap();
    assert!(!unarchived.is_archived());
}

#[tokio::test]
async fn derives_per_serving_nutrition() {
    let h = harness();
    let actor = UserId::new();
    let product = seed_product(&h.products, 200);
    // 100g of a 200 kcal/100g product => 200 kcal, across 2 servings => 100 kcal each.
    let recipe = h
        .service
        .create_recipe(new_recipe(actor, vec![component(product)]))
        .await
        .unwrap();

    let nutrition = h.service.nutrition_for(recipe.id, actor).await.unwrap();
    assert_eq!(nutrition.consumed.facts.energy_kcal, Some(d(100)));
    assert_eq!(nutrition.consumed.quality, NutritionQuality::Known);
    assert!(nutrition.gaps.is_empty());
}

fn ingredient_component(ingredient_id: IngredientId) -> NewRecipeComponent {
    NewRecipeComponent {
        id: None,
        requirement: RecipeRequirement::Ingredient { ingredient_id },
        amount: grams(100),
    }
}

fn unresolved_component(text: &str) -> NewRecipeComponent {
    NewRecipeComponent {
        id: None,
        requirement: RecipeRequirement::Unresolved {
            text: text.to_owned(),
        },
        amount: grams(100),
    }
}

#[tokio::test]
async fn a_generic_ingredient_line_yields_estimated_nutrition() {
    let h = harness();
    let actor = UserId::new();
    let oats = seed_ingredient(&h.ingredients);
    seed_product_mapped(&h.products, 200, oats);
    seed_product_mapped(&h.products, 400, oats);

    let recipe = h
        .service
        .create_recipe(new_recipe(actor, vec![ingredient_component(oats)]))
        .await
        .unwrap();

    let nutrition = h.service.nutrition_for(recipe.id, actor).await.unwrap();
    // Mean of 200 and 400 kcal/100g over 100g => 300 kcal, across 2 servings => 150 each.
    assert_eq!(nutrition.consumed.facts.energy_kcal, Some(d(150)));
    assert_eq!(nutrition.consumed.quality, NutritionQuality::Estimated);
    assert!(nutrition.gaps.is_empty());
}

#[tokio::test]
async fn a_generic_ingredient_with_no_products_is_unknown() {
    let h = harness();
    let actor = UserId::new();
    let oats = seed_ingredient(&h.ingredients);

    let recipe = h
        .service
        .create_recipe(new_recipe(actor, vec![ingredient_component(oats)]))
        .await
        .unwrap();

    let nutrition = h.service.nutrition_for(recipe.id, actor).await.unwrap();
    assert_eq!(nutrition.consumed.facts.energy_kcal, None);
    assert_eq!(nutrition.consumed.quality, NutritionQuality::Unknown);
    assert_eq!(nutrition.gaps.len(), 1);
    assert_eq!(
        nutrition.gaps[0].component_id,
        Some(recipe.components[0].id)
    );
    assert_eq!(nutrition.gaps[0].reason, NutritionGapReason::NoData);
}

#[tokio::test]
async fn a_recipe_saves_with_an_unresolved_line() {
    let h = harness();
    let actor = UserId::new();

    let recipe = h
        .service
        .create_recipe(new_recipe(actor, vec![unresolved_component("Jasmin Rice")]))
        .await
        .unwrap();

    assert!(recipe.components[0].requirement.is_unresolved());
    let nutrition = h.service.nutrition_for(recipe.id, actor).await.unwrap();
    assert_eq!(nutrition.consumed.quality, NutritionQuality::Unknown);
    assert_eq!(nutrition.gaps.len(), 1);
    assert_eq!(nutrition.gaps[0].reason, NutritionGapReason::Unmatched);
    assert_eq!(nutrition.gaps[0].name, "Jasmin Rice");
}

#[tokio::test]
async fn an_unknown_ingredient_is_rejected() {
    let h = harness();
    let actor = UserId::new();

    let err = h
        .service
        .create_recipe(new_recipe(
            actor,
            vec![ingredient_component(IngredientId::new())],
        ))
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn a_generic_line_cannot_be_measured_in_servings() {
    let h = harness();
    let actor = UserId::new();
    let oats = seed_ingredient(&h.ingredients);

    let mut component = ingredient_component(oats);
    component.amount = ConsumedAmount::Servings(d(1));
    let err = h
        .service
        .create_recipe(new_recipe(actor, vec![component]))
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn resolving_an_unresolved_line_records_the_original_text() {
    let h = harness();
    let actor = UserId::new();
    let oats = seed_ingredient(&h.ingredients);
    let recipe = h
        .service
        .create_recipe(new_recipe(actor, vec![unresolved_component("Rolld Oats")]))
        .await
        .unwrap();
    let component_id = recipe.components[0].id;

    let resolved = h
        .service
        .resolve_component(
            recipe.id,
            recipe.revision,
            component_id,
            ResolveRequirement::Ingredient {
                ingredient_id: oats,
            },
            actor,
        )
        .await
        .unwrap();

    assert_eq!(
        resolved.components[0].requirement,
        RecipeRequirement::Ingredient {
            ingredient_id: oats
        }
    );
    assert_eq!(
        resolved.components[0].source_text.as_deref(),
        Some("Rolld Oats")
    );
    assert_eq!(resolved.revision, recipe.revision.next());
}

#[tokio::test]
async fn resolving_an_already_resolved_line_is_rejected() {
    let h = harness();
    let actor = UserId::new();
    let product = seed_product(&h.products, 200);
    let oats = seed_ingredient(&h.ingredients);
    let recipe = h
        .service
        .create_recipe(new_recipe(actor, vec![component(product)]))
        .await
        .unwrap();

    let err = h
        .service
        .resolve_component(
            recipe.id,
            recipe.revision,
            recipe.components[0].id,
            ResolveRequirement::Ingredient {
                ingredient_id: oats,
            },
            actor,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn resolving_with_a_stale_revision_conflicts() {
    let h = harness();
    let actor = UserId::new();
    let oats = seed_ingredient(&h.ingredients);
    let recipe = h
        .service
        .create_recipe(new_recipe(actor, vec![unresolved_component("Rolld Oats")]))
        .await
        .unwrap();

    let err = h
        .service
        .resolve_component(
            recipe.id,
            Revision::new(99),
            recipe.components[0].id,
            ResolveRequirement::Ingredient {
                ingredient_id: oats,
            },
            actor,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::RevisionMismatch { .. }));
}

#[tokio::test]
async fn resolving_one_recipe_does_not_touch_another_with_the_same_text() {
    let h = harness();
    let actor = UserId::new();
    let oats = seed_ingredient(&h.ingredients);
    let first = h
        .service
        .create_recipe(new_recipe(actor, vec![unresolved_component("Rolld Oats")]))
        .await
        .unwrap();
    let second = h
        .service
        .create_recipe(new_recipe(actor, vec![unresolved_component("Rolld Oats")]))
        .await
        .unwrap();

    h.service
        .resolve_component(
            first.id,
            first.revision,
            first.components[0].id,
            ResolveRequirement::Ingredient {
                ingredient_id: oats,
            },
            actor,
        )
        .await
        .unwrap();

    let reloaded = h.service.get_recipe(second.id, actor).await.unwrap();
    assert!(reloaded.components[0].requirement.is_unresolved());
}
