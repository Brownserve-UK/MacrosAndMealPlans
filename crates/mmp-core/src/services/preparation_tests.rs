use std::sync::Arc;

use rust_decimal::Decimal;
use time::macros::{date, datetime};

use super::{MoveCookedFood, PreparationService, RecordPreparation};
use crate::domain::{
    ConsumedAmount, NutritionFacts, PortionPlacement, PreparationSource, Product, ProductId,
    Provenance, Quantity, Recipe, RecipeComponent, RecipeComponentId, RecipeId, RecipeRequirement,
    RecipeVisibility, Revision, StockItem, StockItemId, StockLevel, StockSubject, StorageLocation,
    Unit, UsabilityDeadline, UserId,
};
use crate::ports::{FixedClock, PageRequest, StockQuery, StockRepository};
use crate::testing::{
    InMemoryHouseholdSettingsRepository, InMemoryIngredientRepository,
    InMemoryPreparedBatchRepository, InMemoryPreparedMealRepository, InMemoryProductRepository,
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
        mapped_prepared_meal_id: None,
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
        Arc::new(InMemoryPreparedMealRepository::new()),
        Arc::new(InMemoryHouseholdSettingsRepository::new()),
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
async fn putting_leftovers_away_can_split_them_between_two_places() {
    let h = harness();
    seed_rice(&h, 2000);

    let batch = h
        .service
        .record(record(
            &h,
            6,
            vec![placement(
                StorageLocation::Ambient,
                6,
                date!(2026 - 09 - 07),
            )],
        ))
        .await
        .unwrap()
        .into_value();

    h.service
        .place(
            batch.id,
            batch.revision,
            vec![
                placement(StorageLocation::Chilled, 2, date!(2026 - 09 - 09)),
                placement(StorageLocation::Frozen, 4, date!(2026 - 12 - 05)),
            ],
            UserId::new(),
        )
        .await
        .unwrap();

    let held = portions(&h).await;
    assert_eq!(held.len(), 2, "one portion became two");
    assert_eq!(held[0].storage_location, StorageLocation::Chilled);
    assert_eq!(held[1].storage_location, StorageLocation::Frozen);
    assert_eq!(
        held[0].prepared_batch_id(),
        held[1].prepared_batch_id(),
        "both still belong to the same cook"
    );
}

#[tokio::test]
async fn putting_away_more_than_is_left_is_refused() {
    let h = harness();
    seed_rice(&h, 2000);

    let batch = h
        .service
        .record(record(
            &h,
            4,
            vec![placement(
                StorageLocation::Ambient,
                4,
                date!(2026 - 09 - 07),
            )],
        ))
        .await
        .unwrap()
        .into_value();

    let refused = h
        .service
        .place(
            batch.id,
            batch.revision,
            vec![placement(StorageLocation::Frozen, 5, date!(2026 - 12 - 05))],
            UserId::new(),
        )
        .await;
    assert!(refused.is_err());
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

async fn cook_chilled(h: &Harness, servings: i64, at: time::OffsetDateTime) {
    h.service
        .record(RecordPreparation {
            recipe_id: h.recipe.id,
            source: PreparationSource::Standalone,
            servings_produced: d(servings),
            placements: vec![PortionPlacement::new(StorageLocation::Chilled, d(servings))],
            prepared_at: Some(at),
            actor: UserId::new(),
        })
        .await
        .unwrap();
}

fn servings_in(items: &[StockItem], location: StorageLocation) -> Decimal {
    items
        .iter()
        .filter(|item| item.storage_location == location)
        .filter_map(|item| item.level.conservative_quantity())
        .map(|quantity| quantity.amount)
        .sum()
}

fn move_to(
    h: &Harness,
    from: StorageLocation,
    to: StorageLocation,
    servings: i64,
) -> MoveCookedFood {
    MoveCookedFood {
        recipe_id: h.recipe.id,
        from,
        to,
        servings: d(servings),
        actor: UserId::new(),
    }
}

#[tokio::test]
async fn moving_servings_empties_the_oldest_cook_first() {
    let h = harness();
    seed_rice(&h, 4000);
    cook_chilled(&h, 2, NOW - time::Duration::days(1)).await;
    cook_chilled(&h, 3, NOW).await;

    h.service
        .move_cooked(move_to(
            &h,
            StorageLocation::Chilled,
            StorageLocation::Frozen,
            3,
        ))
        .await
        .unwrap();

    let left = portions(&h).await;
    assert_eq!(
        servings_in(&left, StorageLocation::Frozen),
        d(3),
        "three servings went into the freezer"
    );
    assert_eq!(
        servings_in(&left, StorageLocation::Chilled),
        d(2),
        "and two are still in the fridge"
    );
    assert_eq!(
        left.iter()
            .filter(|item| item.storage_location == StorageLocation::Chilled)
            .count(),
        1,
        "the older cook was emptied outright rather than left as a nil row"
    );
}

#[tokio::test]
async fn moved_food_takes_its_new_deadline_from_where_it_lands() {
    let h = harness();
    seed_rice(&h, 4000);
    cook_chilled(&h, 4, NOW).await;

    h.service
        .move_cooked(move_to(
            &h,
            StorageLocation::Chilled,
            StorageLocation::Frozen,
            4,
        ))
        .await
        .unwrap();
    let frozen = portions(&h).await;
    assert_eq!(
        frozen[0].usability_deadline.as_ref().map(|d| d.date),
        Some(NOW.date() + time::Duration::days(90)),
        "the freezer sets its own clock, nobody has to type a date"
    );

    h.service
        .move_cooked(move_to(
            &h,
            StorageLocation::Frozen,
            StorageLocation::Chilled,
            4,
        ))
        .await
        .unwrap();
    let chilled = portions(&h).await;
    assert_eq!(
        chilled[0].usability_deadline.as_ref().map(|d| d.date),
        Some(NOW.date() + time::Duration::days(2)),
        "and defrosting it resets the clock to the fridge's"
    );
}

#[tokio::test]
async fn moving_more_than_is_there_is_refused_and_changes_nothing() {
    let h = harness();
    seed_rice(&h, 4000);
    cook_chilled(&h, 2, NOW).await;

    let error = h
        .service
        .move_cooked(move_to(
            &h,
            StorageLocation::Chilled,
            StorageLocation::Frozen,
            3,
        ))
        .await
        .unwrap_err();

    assert!(
        matches!(error, crate::error::CoreError::Conflict { .. }),
        "expected a conflict, got {error:?}"
    );
    let left = portions(&h).await;
    assert_eq!(servings_in(&left, StorageLocation::Chilled), d(2));
    assert_eq!(servings_in(&left, StorageLocation::Frozen), Decimal::ZERO);
}

#[tokio::test]
async fn cooked_food_cannot_be_moved_to_the_cupboard() {
    let h = harness();
    seed_rice(&h, 4000);
    cook_chilled(&h, 2, NOW).await;

    let error = h
        .service
        .move_cooked(move_to(
            &h,
            StorageLocation::Chilled,
            StorageLocation::Ambient,
            1,
        ))
        .await
        .unwrap_err();

    assert!(
        matches!(error, crate::error::CoreError::Conflict { .. }),
        "expected a conflict, got {error:?}"
    );
}
