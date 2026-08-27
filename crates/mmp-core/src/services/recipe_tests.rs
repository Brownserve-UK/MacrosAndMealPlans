use std::sync::Arc;

use rust_decimal::Decimal;
use time::OffsetDateTime;
use time::macros::datetime;

use crate::CoreError;
use crate::domain::{
    ConsumedAmount, NewRecipe, NewRecipeComponent, NutritionFacts, NutritionQuality, Product,
    ProductId, Provenance, Quantity, RecipePatch, Revision, Unit, UserId,
};
use crate::ports::{FixedClock, PageRequest, RecipeQuery, SortDirection};
use crate::services::RecipeService;
use crate::testing::{InMemoryProductRepository, InMemoryRecipeRepository};

struct Harness {
    service: RecipeService,
    products: InMemoryProductRepository,
    recipes: InMemoryRecipeRepository,
}

fn harness() -> Harness {
    harness_at(datetime!(2026-08-22 09:00 UTC))
}

fn harness_at(now: OffsetDateTime) -> Harness {
    let products = InMemoryProductRepository::new();
    let recipes = InMemoryRecipeRepository::new();
    let service = RecipeService::new(
        Arc::new(recipes.clone()),
        Arc::new(products.clone()),
        Arc::new(FixedClock::new(now)),
    );
    Harness {
        service,
        products,
        recipes,
    }
}

fn d(value: i64) -> Decimal {
    Decimal::from(value)
}

fn seed_product(products: &InMemoryProductRepository, energy: i64) -> ProductId {
    let id = ProductId::new();
    products.seed(Product {
        id,
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
        product_id,
        amount: grams(100),
    }
}

fn new_recipe(actor: UserId, components: Vec<NewRecipeComponent>) -> NewRecipe {
    NewRecipe {
        id: None,
        name: "Soup".to_owned(),
        servings: 2,
        components,
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
                product_id: second,
                amount: grams(100),
            },
            NewRecipeComponent {
                id: Some(first_id),
                product_id: first,
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
            product_id: product,
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
    assert_eq!(nutrition.facts.energy_kcal, Some(d(100)));
    assert_eq!(nutrition.quality, NutritionQuality::Known);
}
