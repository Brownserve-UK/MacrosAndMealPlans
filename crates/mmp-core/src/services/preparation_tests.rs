use std::sync::Arc;

use rust_decimal::Decimal;
use time::macros::{date, datetime};

use super::{PreparationService, RecordPreparation};
use crate::domain::{
    ConsumedAmount, NutritionFacts, PortionPlacement, PreparationSource, Product, ProductId,
    Provenance, Quantity, Recipe, RecipeComponent, RecipeComponentId, RecipeId, RecipeRequirement,
    RecipeVisibility, Revision, StockItem, StockItemId, StockLevel, StockSubject, StorageLocation,
    Unit, UsabilityDeadline, UserId,
};
use crate::ports::{FixedClock, PageRequest, StockQuery, StockRepository};
use crate::testing::{
    InMemoryIngredientRepository, InMemoryPreparedBatchRepository, InMemoryProductRepository,
    InMemoryRecipeRepository, InMemoryStockRepository,
};

const NOW: time::OffsetDateTime = datetime!(2026-09-06 09:00 UTC);

fn d(value: i64) -> Decimal {
    Decimal::from(value)
}

struct Harness {
    service: PreparationService,
    stock: InMemoryStockRepository,
    recipe: Recipe,
    rice: ProductId,
}

fn harness() -> Harness {
    let owner = UserId::new();
    let products = InMemoryProductRepository::new();
    let recipes = InMemoryRecipeRepository::new();
    let stock = InMemoryStockRepository::new();
    let batches = InMemoryPreparedBatchRepository::with_stock(stock.clone());

    let rice = Product {
        id: ProductId::new(),
        name: "Basmati Rice".to_owned(),
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
        created_at: NOW,
        updated_at: NOW,
        archived_at: None,
    };
    products.seed(rice.clone());

    let recipe = Recipe {
        id: RecipeId::new(),
        name: "Chicken and Rice".to_owned(),
        description: None,
        servings: 4,
        preparation_minutes: None,
        cooking_minutes: None,
        notes: None,
        components: vec![RecipeComponent {
            id: RecipeComponentId::new(),
            requirement: RecipeRequirement::Product {
                product_id: rice.id,
            },
            source_text: None,
            amount: ConsumedAmount::Measure(Quantity::new(d(400), Unit::Gram)),
            position: 0,
        }],
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
        created_at: NOW,
        updated_at: NOW,
        archived_at: None,
    };
    recipes.seed(recipe.clone());

    let service = PreparationService::new(
        Arc::new(batches),
        Arc::new(recipes),
        Arc::new(products),
        Arc::new(InMemoryIngredientRepository::new()),
        Arc::new(FixedClock::new(NOW)),
    );

    Harness {
        service,
        stock,
        recipe,
        rice: rice.id,
    }
}

fn seed_rice(h: &Harness, grams: i64) -> StockItemId {
    let item = StockItem {
        id: StockItemId::new(),
        subject: StockSubject::product(h.rice),
        level: StockLevel::Exact {
            quantity: Quantity::new(d(grams), Unit::Gram),
        },
        storage_location: StorageLocation::Ambient,
        source_date: None,
        usability_deadline: None,
        note: None,
        revision: Revision::INITIAL,
        created_at: NOW,
        updated_at: NOW,
        archived_at: None,
    };
    let id = item.id;
    h.stock.seed(item);
    id
}

async fn portions(h: &Harness) -> Vec<StockItem> {
    let mut items: Vec<StockItem> = h
        .stock
        .list(&StockQuery {
            include_archived: false,
            page: PageRequest::new(1, PageRequest::MAX_PER_PAGE),
            ..Default::default()
        })
        .await
        .unwrap()
        .items
        .into_iter()
        .filter(|item| item.is_prepared_portion())
        .collect();
    items.sort_by_key(|item| item.storage_location.code());
    items
}

fn placement(location: StorageLocation, servings: i64, deadline: time::Date) -> PortionPlacement {
    PortionPlacement {
        storage_location: location,
        servings: d(servings),
        usability_deadline: Some(UsabilityDeadline {
            date: deadline,
            basis: None,
        }),
        note: None,
    }
}

fn record(h: &Harness, produced: i64, placements: Vec<PortionPlacement>) -> RecordPreparation {
    RecordPreparation {
        recipe_id: h.recipe.id,
        source: PreparationSource::Standalone,
        servings_produced: d(produced),
        placements,
        prepared_at: None,
        actor: UserId::new(),
    }
}

#[tokio::test]
async fn a_cook_with_no_meal_behind_it_is_still_found_by_date() {
    let h = harness();
    seed_rice(&h, 2000);

    h.service
        .record(record(
            &h,
            6,
            vec![placement(StorageLocation::Frozen, 6, date!(2026 - 12 - 05))],
        ))
        .await
        .unwrap();

    let found = h
        .service
        .list_in_range(date!(2026 - 09 - 06), date!(2026 - 09 - 06))
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].servings_produced, d(6));
    assert_eq!(found[0].source, PreparationSource::Standalone);

    let elsewhere = h
        .service
        .list_in_range(date!(2026 - 09 - 07), date!(2026 - 09 - 30))
        .await
        .unwrap();
    assert!(elsewhere.is_empty());
}

#[tokio::test]
async fn one_cook_can_land_in_two_places() {
    let h = harness();
    seed_rice(&h, 2000);

    h.service
        .record(record(
            &h,
            6,
            vec![
                placement(StorageLocation::Chilled, 2, date!(2026 - 09 - 09)),
                placement(StorageLocation::Frozen, 4, date!(2026 - 12 - 05)),
            ],
        ))
        .await
        .unwrap();

    let held = portions(&h).await;
    assert_eq!(held.len(), 2, "one cook, two places, two stock items");
    assert_eq!(held[0].storage_location, StorageLocation::Chilled);
    assert_eq!(held[1].storage_location, StorageLocation::Frozen);
    assert_eq!(
        held[0].prepared_batch_id(),
        held[1].prepared_batch_id(),
        "both portions belong to the same cook"
    );
    assert_eq!(
        held[0].usability_deadline.as_ref().unwrap().date,
        date!(2026 - 09 - 09),
        "the fridge portion keeps its own use-by"
    );
}

#[tokio::test]
async fn the_raw_ingredients_are_drawn_once_for_the_whole_cook() {
    let h = harness();
    let rice = seed_rice(&h, 2000);

    h.service
        .record(record(
            &h,
            6,
            vec![
                placement(StorageLocation::Chilled, 2, date!(2026 - 09 - 09)),
                placement(StorageLocation::Frozen, 4, date!(2026 - 12 - 05)),
            ],
        ))
        .await
        .unwrap();

    let remaining = h.stock.get(rice).await.unwrap().unwrap();
    let StockLevel::Exact { quantity } = remaining.level else {
        panic!("expected an exact level");
    };
    assert_eq!(
        quantity.amount,
        d(1400),
        "six servings of a four serving recipe draws 600 g, however many places it lands in"
    );
}

#[tokio::test]
async fn the_places_have_to_add_up_to_what_was_made() {
    let h = harness();
    seed_rice(&h, 2000);

    let error = h
        .service
        .record(record(
            &h,
            6,
            vec![placement(StorageLocation::Frozen, 4, date!(2026 - 12 - 05))],
        ))
        .await
        .unwrap_err();

    assert!(
        matches!(error, crate::error::CoreError::Validation(_)),
        "expected a validation error, got {error:?}"
    );
    assert!(
        portions(&h).await.is_empty(),
        "nothing should be stored when the split does not balance"
    );
}
