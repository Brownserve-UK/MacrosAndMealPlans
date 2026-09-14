use std::sync::Arc;

use rust_decimal::Decimal;
use time::OffsetDateTime;
use time::macros::{date, datetime};

use super::*;
use crate::domain::{
    ConsumedAmount, HouseholdMember, HouseholdMemberId, Ingredient, IngredientId, MealItemRef,
    MealPlanComponent, MealPlanEntry, MealSlot, MissingStockInterpretation, NewStockItem, Product,
    ProductId, Provenance, Quantity, Revision, SectionOrder, StockLevel, StockSubject,
    StorageLocation, Unit, UserId, WeightDisplay,
};
use crate::ports::{Clock, FixedClock, MealPlanRepository};
use crate::testing::{
    InMemoryFinishShopRepository, InMemoryHouseholdMemberRepository,
    InMemoryHouseholdSettingsRepository, InMemoryIngredientRepository, InMemoryMealPlanRepository,
    InMemoryPreparedBatchRepository, InMemoryPreparedMealRepository, InMemoryProductRepository,
    InMemoryPurchaseRepository, InMemoryRecipeRepository, InMemoryShoppingCadenceRepository,
    InMemoryShoppingListItemRepository, InMemoryShoppingOpportunityRepository,
    InMemoryShoppingSuggestionDismissalRepository, InMemoryShoppingTripRepository,
    InMemoryStockRepository,
};
use time::Weekday;

const TODAY: time::Date = date!(2026 - 08 - 31);

struct Harness {
    shopping: ShoppingService,
    stock_service: StockService,
    stock: InMemoryStockRepository,
    products: InMemoryProductRepository,
    ingredients: InMemoryIngredientRepository,
    meal_plans: InMemoryMealPlanRepository,
    settings: InMemoryHouseholdSettingsRepository,
    purchases: InMemoryPurchaseRepository,
    member_id: HouseholdMemberId,
    actor_id: UserId,
}

fn harness() -> Harness {
    let stock = InMemoryStockRepository::new();
    let products = InMemoryProductRepository::new();
    let ingredients = InMemoryIngredientRepository::new();
    let meal_plans = InMemoryMealPlanRepository::default();
    let recipes = InMemoryRecipeRepository::new();
    let members = InMemoryHouseholdMemberRepository::new();
    let settings = InMemoryHouseholdSettingsRepository::new();
    let cadence = InMemoryShoppingCadenceRepository::new();
    let opportunities = InMemoryShoppingOpportunityRepository::new();
    let purchases = InMemoryPurchaseRepository::new();
    let list_items = InMemoryShoppingListItemRepository::new();
    let trips = InMemoryShoppingTripRepository::new();
    let member_id = HouseholdMemberId::new();
    let now = OffsetDateTime::UNIX_EPOCH;
    members.seed(HouseholdMember {
        id: member_id,
        display_name: "Sample".to_owned(),
        linked_user_id: None,
        weight_display: WeightDisplay::default(),
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    });
    let clock: Arc<dyn Clock> = Arc::new(FixedClock::new(datetime!(2026-08-31 09:00 UTC)));
    let stock_service = StockService::new(
        Arc::new(stock.clone()),
        Arc::new(products.clone()),
        Arc::new(ingredients.clone()),
        Arc::new(InMemoryPreparedMealRepository::new()),
        Arc::new(meal_plans.clone()),
        Arc::new(recipes),
        Arc::new(InMemoryPreparedBatchRepository::with_stock(stock.clone())),
        Arc::new(members),
        Arc::new(settings.clone()),
        clock.clone(),
    );
    let shopping = ShoppingService::new(
        Arc::new(cadence),
        Arc::new(opportunities),
        Arc::new(purchases.clone()),
        Arc::new(list_items.clone()),
        Arc::new(trips.clone()),
        Arc::new(InMemoryFinishShopRepository::new(
            purchases.clone(),
            list_items.clone(),
            trips,
        )),
        Arc::new(InMemoryShoppingSuggestionDismissalRepository::new()),
        Arc::new(ingredients.clone()),
        Arc::new(InMemoryPreparedMealRepository::new()),
        Arc::new(products.clone()),
        Arc::new(settings.clone()),
        stock_service.clone(),
        clock,
    );

    Harness {
        shopping,
        stock_service,
        stock,
        products,
        ingredients,
        meal_plans,
        settings,
        purchases,
        member_id,
        actor_id: UserId::new(),
    }
}

fn ml(value: i64) -> Quantity {
    Quantity::new(Decimal::new(value, 0), Unit::Millilitre)
}

fn seed_ingredient(h: &Harness, id: IngredientId, name: &str) {
    seed_ingredient_in(h, id, name, ShoppingSection::Dairy);
}

fn seed_ingredient_in(h: &Harness, id: IngredientId, name: &str, section: ShoppingSection) {
    let now = OffsetDateTime::UNIX_EPOCH;
    h.ingredients.seed(Ingredient {
        id,
        name: name.to_owned(),
        default_unit: Unit::Millilitre,
        shopping_section: Some(section),
        track_stock: None,
        provenance: Provenance::local(),
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    });
}

fn mapped(name: &str, ingredient_id: IngredientId) -> Product {
    let now = OffsetDateTime::UNIX_EPOCH;
    Product {
        id: ProductId::new(),
        name: name.to_owned(),
        brand: None,
        barcode: None,
        retailer: None,
        shopping_section: None,
        track_stock: None,
        package_quantity: Some(ml(1000)),
        servings_per_pack: None,
        mapped_ingredient_id: Some(ingredient_id),
        mapped_prepared_meal_id: None,
        nutrition: Default::default(),
        provenance: Provenance::local(),
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    }
}

async fn add_stock(h: &Harness, product_id: ProductId, quantity: Quantity) {
    h.stock_service
        .create(
            NewStockItem {
                subject: StockSubject::product(product_id),
                level: StockLevel::Exact { quantity },
                storage_location: StorageLocation::Chilled,
                source_date: None,
                usability_deadline: None,
                note: None,
            },
            h.actor_id,
            Some(h.member_id),
        )
        .await
        .unwrap();
}

async fn plan_product(h: &Harness, product_id: ProductId, quantity: Quantity, on: time::Date) {
    let now = OffsetDateTime::UNIX_EPOCH;
    let entry = MealPlanEntry {
        id: crate::domain::MealPlanEntryId::new(),
        scope: crate::domain::MealPlanScope::Member,
        member_id: Some(h.member_id),
        planned_on: on,
        planned_time: None,
        slot: MealSlot::Breakfast,
        components: vec![MealPlanComponent {
            id: crate::domain::MealPlanComponentId::new(),
            item: MealItemRef::product(product_id),
            amount: ConsumedAmount::Measure(quantity),
            position: 0,
            snapshot: None,
            revision: Revision::INITIAL,
            display_order: uuid::Uuid::nil(),
        }],
        participants: Vec::new(),
        guest_groups: Vec::new(),
        opted_out: Vec::new(),
        created_by: h.actor_id,
        updated_by: h.actor_id,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
    };
    h.meal_plans.insert(&entry).await.unwrap();
}

async fn plan_ingredient(
    h: &Harness,
    ingredient_id: IngredientId,
    quantity: Quantity,
    on: time::Date,
) {
    let now = OffsetDateTime::UNIX_EPOCH;
    let entry = MealPlanEntry {
        id: crate::domain::MealPlanEntryId::new(),
        scope: crate::domain::MealPlanScope::Member,
        member_id: Some(h.member_id),
        planned_on: on,
        planned_time: None,
        slot: MealSlot::Breakfast,
        components: vec![MealPlanComponent {
            id: crate::domain::MealPlanComponentId::new(),
            item: MealItemRef::ingredient(ingredient_id),
            amount: ConsumedAmount::Measure(quantity),
            position: 0,
            snapshot: None,
            revision: Revision::INITIAL,
            display_order: uuid::Uuid::nil(),
        }],
        participants: Vec::new(),
        guest_groups: Vec::new(),
        opted_out: Vec::new(),
        created_by: h.actor_id,
        updated_by: h.actor_id,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
    };
    h.meal_plans.insert(&entry).await.unwrap();
}

async fn weekly_saturdays(h: &Harness) {
    h.shopping
        .set_cadence(
            Revision::UNRECORDED,
            NewShoppingCadence {
                interval_weeks: 1,
                days: vec![Weekday::Saturday],
                anchor: TODAY,
                usual_time: None,
            },
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn a_pool_is_one_requirement_not_one_per_product_in_it() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Sample Whole Milk", milk);
    let b = mapped("Sample Value Whole Milk", milk);
    h.products.seed(a.clone());
    h.products.seed(b.clone());
    add_stock(&h, a.id, ml(500)).await;
    add_stock(&h, b.id, ml(500)).await;

    plan_product(&h, a.id, ml(800), date!(2026 - 09 - 02)).await;
    plan_product(&h, b.id, ml(800), date!(2026 - 09 - 03)).await;

    let list = h.shopping.requirements(None).await.unwrap();

    assert_eq!(
        list.requirements.len(),
        1,
        "the pool asks to be bought once, not once per bottle: {:?}",
        list.requirements
            .iter()
            .map(|r| &r.name)
            .collect::<Vec<_>>()
    );
    assert_eq!(list.requirements[0].name, "Whole Milk");
    assert_eq!(list.requirements[0].quantity, Some(ml(600)));
}

#[tokio::test]
async fn a_pool_pinned_entirely_to_products_still_knows_its_dates() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Sample Whole Milk", milk);
    h.products.seed(a.clone());
    add_stock(&h, a.id, ml(500)).await;

    plan_product(&h, a.id, ml(400), date!(2026 - 09 - 02)).await;
    plan_product(&h, a.id, ml(400), date!(2026 - 09 - 05)).await;

    let list = h.shopping.requirements(None).await.unwrap();
    let requirement = list
        .requirements
        .iter()
        .find(|r| r.name == "Whole Milk")
        .expect("the pool is reported");

    assert_eq!(requirement.required_by, Some(date!(2026 - 09 - 05)));
    assert_eq!(requirement.use_by_at_least, Some(date!(2026 - 09 - 05)));
    assert!(!requirement.claims.is_empty());
}

#[tokio::test]
async fn something_we_hold_no_record_of_is_only_a_suggestion() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Sample Whole Milk", milk);
    h.products.seed(a.clone());
    plan_product(&h, a.id, ml(400), date!(2026 - 09 - 02)).await;

    let list = h.shopping.requirements(None).await.unwrap();
    let requirement = &list.requirements[0];

    assert_eq!(
        requirement.certainty,
        Certainty::Suggested {
            reason: SuggestionReason::UnknownAvailability
        }
    );
}

#[tokio::test]
async fn the_same_gap_is_definite_when_the_household_reads_absence_as_absent() {
    let h = harness();
    weekly_saturdays(&h).await;
    h.settings
        .set_missing_stock_interpretation(MissingStockInterpretation::Absent);
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Sample Whole Milk", milk);
    h.products.seed(a.clone());
    plan_product(&h, a.id, ml(400), date!(2026 - 09 - 02)).await;

    let list = h.shopping.requirements(None).await.unwrap();

    assert_eq!(list.requirements[0].certainty, Certainty::Definite);
}

#[tokio::test]
async fn an_untracked_ingredient_never_reaches_the_list() {
    let h = harness();
    weekly_saturdays(&h).await;
    let paprika = IngredientId::new();
    seed_ingredient(&h, paprika, "Paprika");
    h.ingredients.set_track_stock(paprika, Some(false));
    let a = mapped("Sample Paprika", paprika);
    h.products.seed(a.clone());
    plan_product(&h, a.id, ml(10), date!(2026 - 09 - 02)).await;

    let list = h.shopping.requirements(None).await.unwrap();

    assert!(list.requirements.is_empty());
}

#[tokio::test]
async fn a_tracked_ingredient_is_a_definite_buy_not_a_suggestion() {
    let h = harness();
    weekly_saturdays(&h).await;
    let apples = IngredientId::new();
    seed_ingredient(&h, apples, "Apples");
    h.ingredients.set_track_stock(apples, Some(true));
    let a = mapped("Sample Braeburn", apples);
    h.products.seed(a.clone());
    plan_product(&h, a.id, ml(400), date!(2026 - 09 - 02)).await;

    let list = h.shopping.requirements(None).await.unwrap();

    assert_eq!(list.requirements.len(), 1);
    assert_eq!(list.requirements[0].certainty, Certainty::Definite);
}

#[tokio::test]
async fn a_food_with_no_mapped_product_is_reported_not_dropped() {
    let h = harness();
    weekly_saturdays(&h).await;
    let lasagne = IngredientId::new();
    seed_ingredient(&h, lasagne, "Frozen lasagne");
    h.ingredients.set_track_stock(lasagne, Some(true));
    plan_ingredient(&h, lasagne, ml(400), date!(2026 - 09 - 02)).await;

    let list = h.shopping.requirements(None).await.unwrap();

    assert_eq!(list.requirements.len(), 1);
    assert_eq!(
        list.requirements[0].certainty,
        Certainty::Suggested {
            reason: SuggestionReason::NoProductYet
        }
    );
    assert!(
        list.requirements[0]
            .gaps
            .contains(&DemandGap::FoodHasNoProducts)
    );
    assert!(!list.requirements[0].claims.is_empty());
}

#[tokio::test]
async fn a_not_tracked_staple_never_asks_to_be_bought() {
    let h = harness();
    weekly_saturdays(&h).await;
    let salt = IngredientId::new();
    seed_ingredient(&h, salt, "Salt");
    let a = mapped("Sample Salt", salt);
    h.products.seed(a.clone());
    h.stock_service
        .create(
            NewStockItem {
                subject: StockSubject::product(a.id),
                level: StockLevel::NotTracked,
                storage_location: StorageLocation::Ambient,
                source_date: None,
                usability_deadline: None,
                note: None,
            },
            h.actor_id,
            Some(h.member_id),
        )
        .await
        .unwrap();
    plan_product(&h, a.id, ml(400), date!(2026 - 09 - 02)).await;

    let list = h.shopping.requirements(None).await.unwrap();

    assert!(list.requirements.is_empty());
}

#[tokio::test]
async fn starting_a_shop_pins_the_list_you_set_off_with() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let bottle = mapped("Sample Whole Milk", milk);
    h.products.seed(bottle.clone());
    plan_product(&h, bottle.id, ml(400), date!(2026 - 09 - 08)).await;

    let trip = h
        .shopping
        .start_shop(date!(2026 - 09 - 05), h.actor_id)
        .await
        .unwrap();
    assert_eq!(trip.state, TripState::Shopping);
    assert_eq!(trip.rows.len(), 1);
    assert_eq!(trip.rows[0].name, "Whole Milk");
    assert_eq!(trip.rows[0].quantity, Some(ml(400)));

    let bread = IngredientId::new();
    seed_ingredient_in(&h, bread, "Bread", ShoppingSection::Bakery);
    let loaf = mapped("Sample Bread", bread);
    h.products.seed(loaf.clone());
    plan_product(&h, loaf.id, ml(400), date!(2026 - 09 - 08)).await;

    let pinned = h
        .shopping
        .trip(date!(2026 - 09 - 05))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pinned.rows.len(), 1);
    assert_eq!(pinned.rows[0].name, "Whole Milk");

    let live = h
        .shopping
        .requirements(Some(date!(2026 - 09 - 05)))
        .await
        .unwrap();
    assert_eq!(live.requirements.len(), 2);
    assert!(live.trip.is_some());
}

#[tokio::test]
async fn setting_off_twice_keeps_the_first_trip() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let bottle = mapped("Sample Whole Milk", milk);
    h.products.seed(bottle.clone());
    plan_product(&h, bottle.id, ml(400), date!(2026 - 09 - 08)).await;

    let first = h
        .shopping
        .start_shop(date!(2026 - 09 - 05), h.actor_id)
        .await
        .unwrap();
    let again = h
        .shopping
        .start_shop(date!(2026 - 09 - 05), h.actor_id)
        .await
        .unwrap();

    assert_eq!(first.id, again.id);
    assert_eq!(first.started_at, again.started_at);
}

#[tokio::test]
async fn abandoning_a_shop_discards_the_baseline_and_keeps_purchases() {
    let h = harness();
    weekly_saturdays(&h).await;
    let trip = h
        .shopping
        .start_shop(date!(2026 - 09 - 05), h.actor_id)
        .await
        .unwrap();
    let purchase = h
        .shopping
        .record_purchase(
            NewPurchase {
                ingredient_id: None,
                prepared_meal_id: None,
                product_id: None,
                name: Some("Kitchen roll".to_owned()),
                quantity: None,
                opportunity_date: Some(date!(2026 - 09 - 05)),
                note: None,
            },
            h.actor_id,
        )
        .await
        .unwrap();

    h.shopping
        .abandon_shop(date!(2026 - 09 - 05), trip.revision)
        .await
        .unwrap();

    assert_eq!(h.shopping.trip(date!(2026 - 09 - 05)).await.unwrap(), None);
    assert_eq!(
        h.purchases.get(purchase.id).await.unwrap().unwrap().state,
        PurchaseState::Pending
    );
}

#[tokio::test]
async fn pending_purchases_without_a_shop_are_ready_to_put_away() {
    let h = harness();
    let purchase = h
        .shopping
        .record_purchase(
            NewPurchase {
                ingredient_id: None,
                prepared_meal_id: None,
                product_id: None,
                name: Some("Kitchen roll".to_owned()),
                quantity: None,
                opportunity_date: None,
                note: None,
            },
            h.actor_id,
        )
        .await
        .unwrap();

    assert_eq!(
        h.shopping.awaiting_put_away().await.unwrap(),
        vec![purchase]
    );
}

#[tokio::test]
async fn pending_purchases_from_a_past_shop_are_ready_to_put_away() {
    let h = harness();
    let purchase = h
        .shopping
        .record_purchase(
            NewPurchase {
                ingredient_id: None,
                prepared_meal_id: None,
                product_id: None,
                name: Some("Kitchen roll".to_owned()),
                quantity: None,
                opportunity_date: Some(date!(2026 - 08 - 30)),
                note: None,
            },
            h.actor_id,
        )
        .await
        .unwrap();

    assert_eq!(
        h.shopping.awaiting_put_away().await.unwrap(),
        vec![purchase]
    );
}

#[tokio::test]
async fn pending_purchases_from_a_finished_shop_are_ready_to_put_away() {
    let h = harness();
    let trip = h
        .shopping
        .start_shop(date!(2026 - 09 - 05), h.actor_id)
        .await
        .unwrap();
    let purchase = h
        .shopping
        .record_purchase(
            NewPurchase {
                ingredient_id: None,
                prepared_meal_id: None,
                product_id: None,
                name: Some("Kitchen roll".to_owned()),
                quantity: None,
                opportunity_date: Some(date!(2026 - 09 - 05)),
                note: None,
            },
            h.actor_id,
        )
        .await
        .unwrap();
    h.shopping
        .finish_shop(date!(2026 - 09 - 05), h.actor_id, trip.revision)
        .await
        .unwrap();

    assert_eq!(
        h.shopping.awaiting_put_away().await.unwrap(),
        vec![purchase]
    );
}

#[tokio::test]
async fn an_unfinished_shop_with_purchases_is_reachable_from_the_hub() {
    let h = harness();
    h.shopping
        .start_shop(date!(2026 - 09 - 05), h.actor_id)
        .await
        .unwrap();
    h.shopping
        .record_purchase(
            NewPurchase {
                ingredient_id: None,
                prepared_meal_id: None,
                product_id: None,
                name: Some("Kitchen roll".to_owned()),
                quantity: None,
                opportunity_date: Some(date!(2026 - 09 - 05)),
                note: None,
            },
            h.actor_id,
        )
        .await
        .unwrap();

    let list = h.shopping.requirements(None).await.unwrap();

    assert_eq!(
        list.unfinished,
        vec![UnfinishedShop {
            date: date!(2026 - 09 - 05),
            purchases: 1,
        }]
    );
    assert!(h.shopping.awaiting_put_away().await.unwrap().is_empty());
}

#[tokio::test]
async fn finishing_closes_the_trip_it_was_started_as() {
    let h = harness();
    weekly_saturdays(&h).await;

    h.shopping
        .start_shop(date!(2026 - 09 - 05), h.actor_id)
        .await
        .unwrap();
    h.shopping
        .finish_shop(date!(2026 - 09 - 05), h.actor_id, Revision::INITIAL)
        .await
        .unwrap();

    let trip = h
        .shopping
        .trip(date!(2026 - 09 - 05))
        .await
        .unwrap()
        .unwrap();
    assert!(trip.is_finished());
    assert!(trip.finished_at.is_some());
}

#[tokio::test]
async fn finishing_puts_the_shopping_where_it_belongs() {
    let h = harness();
    weekly_saturdays(&h).await;

    let peas = IngredientId::new();
    seed_ingredient_in(&h, peas, "Peas", ShoppingSection::Frozen);
    let bag = mapped("Sample Peas", peas);
    h.products.seed(bag.clone());

    let chicken = IngredientId::new();
    seed_ingredient_in(&h, chicken, "Chicken breast", ShoppingSection::MeatFish);
    let pack = mapped("Sample Chicken", chicken);
    h.products.seed(pack.clone());

    let flour = IngredientId::new();
    seed_ingredient_in(&h, flour, "Plain flour", ShoppingSection::Ambient);
    let sack = mapped("Sample Flour", flour);
    h.products.seed(sack.clone());

    for (ingredient_id, product_id) in [(peas, bag.id), (chicken, pack.id), (flour, sack.id)] {
        h.shopping
            .record_purchase(
                NewPurchase {
                    ingredient_id: Some(ingredient_id),
                    prepared_meal_id: None,
                    product_id: Some(product_id),
                    name: None,
                    quantity: Some(ml(500)),
                    opportunity_date: Some(date!(2026 - 09 - 05)),
                    note: None,
                },
                h.actor_id,
            )
            .await
            .unwrap();
    }

    h.shopping
        .finish_shop(date!(2026 - 09 - 05), h.actor_id, Revision::UNRECORDED)
        .await
        .unwrap();

    let stocked = h.purchases.created_stock();
    let where_it_went = |product_id: ProductId| {
        stocked
            .iter()
            .find(|item| item.product_id() == Some(product_id))
            .expect("the purchase became stock")
            .storage_location
    };
    assert_eq!(where_it_went(bag.id), StorageLocation::Frozen);
    assert_eq!(where_it_went(pack.id), StorageLocation::Chilled);
    assert_eq!(where_it_went(sack.id), StorageLocation::Ambient);
}

#[tokio::test]
async fn something_grabbed_in_store_is_recorded_by_name_alone() {
    let h = harness();
    weekly_saturdays(&h).await;

    let purchase = h
        .shopping
        .record_purchase(
            NewPurchase {
                ingredient_id: None,
                prepared_meal_id: None,
                product_id: None,
                name: Some("Kiwi Fruit".to_owned()),
                quantity: None,
                opportunity_date: Some(date!(2026 - 09 - 05)),
                note: None,
            },
            h.actor_id,
        )
        .await
        .unwrap();

    assert_eq!(purchase.name.as_deref(), Some("Kiwi Fruit"));
    assert_eq!(purchase.state, PurchaseState::Pending);

    let list = h.shopping.requirements(None).await.unwrap();
    assert_eq!(list.unplanned.len(), 1);
    assert_eq!(list.unplanned[0].name.as_deref(), Some("Kiwi Fruit"));

    h.shopping
        .finish_shop(date!(2026 - 09 - 05), h.actor_id, Revision::UNRECORDED)
        .await
        .unwrap();
    assert_eq!(h.stock.count(), 0);
}

#[tokio::test]
async fn the_list_walks_the_aisles_in_the_households_own_order() {
    let h = harness();
    weekly_saturdays(&h).await;

    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let bottle = mapped("Sample Whole Milk", milk);
    h.products.seed(bottle.clone());
    plan_product(&h, bottle.id, ml(400), date!(2026 - 09 - 08)).await;

    let bread = IngredientId::new();
    seed_ingredient_in(&h, bread, "Bread", ShoppingSection::Bakery);
    let loaf = mapped("Sample Bread", bread);
    h.products.seed(loaf.clone());
    plan_product(&h, loaf.id, ml(400), date!(2026 - 09 - 08)).await;

    let dairy_first = h.shopping.requirements(None).await.unwrap();
    assert_eq!(dairy_first.requirements[0].name, "Whole Milk");
    assert_eq!(dairy_first.requirements[1].name, "Bread");

    h.settings.set_section_order(
        SectionOrder::new([
            ShoppingSection::Bakery,
            ShoppingSection::FreshProduce,
            ShoppingSection::MeatFish,
            ShoppingSection::Dairy,
            ShoppingSection::Frozen,
            ShoppingSection::Ambient,
            ShoppingSection::Drinks,
            ShoppingSection::Household,
            ShoppingSection::Other,
        ])
        .unwrap(),
    );

    let bakery_first = h.shopping.requirements(None).await.unwrap();
    assert_eq!(bakery_first.requirements[0].name, "Bread");
    assert_eq!(bakery_first.requirements[1].name, "Whole Milk");
}

#[tokio::test]
async fn something_added_by_hand_survives_the_plan_changing() {
    let h = harness();
    weekly_saturdays(&h).await;

    h.shopping
        .add_list_item(
            NewShoppingListItem {
                ingredient_id: None,
                prepared_meal_id: None,
                product_id: None,
                name: "Onion Salt".to_owned(),
                quantity: None,
                section: Some(ShoppingSection::Ambient),
                opportunity_date: None,
            },
            h.actor_id,
        )
        .await
        .unwrap();

    let before = h.shopping.requirements(None).await.unwrap();
    assert_eq!(before.manual.len(), 1);
    assert_eq!(before.manual[0].name, "Onion Salt");

    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Sample Whole Milk", milk);
    h.products.seed(a.clone());
    plan_product(&h, a.id, ml(400), date!(2026 - 09 - 08)).await;

    let after = h.shopping.requirements(None).await.unwrap();
    assert_eq!(after.manual.len(), 1);
    assert_eq!(after.manual[0].name, "Onion Salt");
    assert!(!after.requirements.is_empty());
}

#[tokio::test]
async fn a_hand_added_item_is_never_derived_onto_the_list() {
    let h = harness();
    weekly_saturdays(&h).await;

    h.shopping
        .add_list_item(
            NewShoppingListItem {
                ingredient_id: None,
                prepared_meal_id: None,
                product_id: None,
                name: "Tomato Ketchup".to_owned(),
                quantity: Some(ml(500)),
                section: Some(ShoppingSection::Ambient),
                opportunity_date: None,
            },
            h.actor_id,
        )
        .await
        .unwrap();

    let list = h.shopping.requirements(None).await.unwrap();

    assert!(list.requirements.is_empty());
    assert_eq!(list.manual.len(), 1);
    assert_eq!(list.manual[0].quantity, Some(ml(500)));
}

#[tokio::test]
async fn finishing_a_shop_clears_the_things_you_added_to_it() {
    let h = harness();
    weekly_saturdays(&h).await;

    h.shopping
        .add_list_item(
            NewShoppingListItem {
                ingredient_id: None,
                prepared_meal_id: None,
                product_id: None,
                name: "Onion Salt".to_owned(),
                quantity: None,
                section: None,
                opportunity_date: Some(date!(2026 - 09 - 05)),
            },
            h.actor_id,
        )
        .await
        .unwrap();
    h.shopping
        .add_list_item(
            NewShoppingListItem {
                ingredient_id: None,
                prepared_meal_id: None,
                product_id: None,
                name: "Kitchen roll".to_owned(),
                quantity: None,
                section: None,
                opportunity_date: Some(date!(2026 - 09 - 12)),
            },
            h.actor_id,
        )
        .await
        .unwrap();

    h.shopping
        .finish_shop(date!(2026 - 09 - 05), h.actor_id, Revision::UNRECORDED)
        .await
        .unwrap();

    let left = h.shopping.list_items().await.unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].name, "Kitchen roll");
}

#[tokio::test]
async fn a_shop_only_buys_what_is_needed_before_the_next_one() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Sample Whole Milk", milk);
    h.products.seed(a.clone());
    plan_product(&h, a.id, ml(400), date!(2026 - 09 - 08)).await;
    plan_product(&h, a.id, ml(400), date!(2026 - 09 - 15)).await;

    let first = h
        .shopping
        .requirements(Some(date!(2026 - 09 - 05)))
        .await
        .unwrap();
    assert_eq!(first.requirements.len(), 1);
    assert_eq!(first.requirements[0].quantity, Some(ml(400)));
    assert_eq!(
        first.requirements[0].use_by_at_least,
        Some(date!(2026 - 09 - 08))
    );

    let second = h
        .shopping
        .requirements(Some(date!(2026 - 09 - 12)))
        .await
        .unwrap();
    assert_eq!(second.requirements.len(), 1);
    assert_eq!(second.requirements[0].quantity, Some(ml(400)));
    assert_eq!(
        second.requirements[0].use_by_at_least,
        Some(date!(2026 - 09 - 15))
    );
}

#[tokio::test]
async fn a_purchase_on_the_focused_shop_binds_to_its_surviving_requirement() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let bottle = mapped("Sample Whole Milk", milk);
    h.products.seed(bottle.clone());
    plan_product(&h, bottle.id, ml(400), date!(2026 - 09 - 08)).await;
    plan_product(&h, bottle.id, ml(400), date!(2026 - 09 - 15)).await;

    let purchase = h
        .shopping
        .record_purchase(
            NewPurchase {
                ingredient_id: Some(milk),
                prepared_meal_id: None,
                product_id: None,
                name: None,
                quantity: None,
                opportunity_date: Some(date!(2026 - 09 - 12)),
                note: None,
            },
            h.actor_id,
        )
        .await
        .unwrap();

    let list = h
        .shopping
        .requirements(Some(date!(2026 - 09 - 12)))
        .await
        .unwrap();

    assert_eq!(list.requirements.len(), 1);
    assert_eq!(list.requirements[0].purchases, vec![purchase]);
    assert!(list.unplanned.is_empty());
}

#[tokio::test]
async fn incompatible_amounts_make_the_requirement_total_unknown() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    h.ingredients.set_track_stock(milk, Some(true));
    h.products.seed(mapped("Sample Whole Milk", milk));
    plan_ingredient(&h, milk, ml(400), date!(2026 - 09 - 02)).await;
    plan_ingredient(
        &h,
        milk,
        Quantity::new(Decimal::new(200, 0), Unit::Gram),
        date!(2026 - 09 - 03),
    )
    .await;

    let list = h.shopping.requirements(None).await.unwrap();

    assert_eq!(list.requirements.len(), 1);
    assert_eq!(list.requirements[0].quantity, None);
    assert!(
        list.requirements[0]
            .gaps
            .contains(&DemandGap::IncompatibleUnits)
    );
}

#[tokio::test]
async fn a_meal_we_can_half_cover_only_buys_the_rest_in_its_own_bucket() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Sample Whole Milk", milk);
    h.products.seed(a.clone());
    add_stock(&h, a.id, ml(300)).await;
    plan_product(&h, a.id, ml(400), date!(2026 - 09 - 08)).await;
    plan_product(&h, a.id, ml(400), date!(2026 - 09 - 15)).await;

    let first = h
        .shopping
        .requirements(Some(date!(2026 - 09 - 05)))
        .await
        .unwrap();
    assert_eq!(first.requirements[0].quantity, Some(ml(100)));

    let second = h
        .shopping
        .requirements(Some(date!(2026 - 09 - 12)))
        .await
        .unwrap();
    assert_eq!(second.requirements[0].quantity, Some(ml(400)));
}

#[tokio::test]
async fn buying_without_details_records_the_purchase_but_creates_no_stock() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Sample Whole Milk", milk);
    h.products.seed(a.clone());
    add_stock(&h, a.id, ml(100)).await;
    plan_product(&h, a.id, ml(400), date!(2026 - 09 - 02)).await;

    let purchase = h
        .shopping
        .record_purchase(
            NewPurchase {
                ingredient_id: Some(milk),
                prepared_meal_id: None,
                product_id: None,
                name: None,
                quantity: None,
                opportunity_date: Some(date!(2026 - 09 - 05)),
                note: None,
            },
            h.actor_id,
        )
        .await
        .unwrap();

    assert_eq!(purchase.state, PurchaseState::Pending);
    assert_eq!(purchase.stock_item_id, None);
    assert_eq!(h.stock.count(), 1);

    let list = h.shopping.requirements(None).await.unwrap();
    let requirement = &list.requirements[0];
    assert_eq!(requirement.quantity, Some(ml(300)));
    assert_eq!(requirement.purchases.len(), 1);
}

#[tokio::test]
async fn buying_with_full_details_still_makes_no_stock_until_the_shop_is_finished() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Sample Whole Milk", milk);
    h.products.seed(a.clone());

    let purchase = h
        .shopping
        .record_purchase(
            NewPurchase {
                ingredient_id: Some(milk),
                prepared_meal_id: None,
                product_id: Some(a.id),
                name: None,
                quantity: Some(ml(1000)),
                opportunity_date: Some(date!(2026 - 09 - 05)),
                note: None,
            },
            h.actor_id,
        )
        .await
        .unwrap();

    assert_eq!(purchase.state, PurchaseState::Pending);
    assert_eq!(purchase.stock_item_id, None);
    assert!(h.purchases.created_stock().is_empty());

    let finished = h
        .shopping
        .finish_shop(date!(2026 - 09 - 05), h.actor_id, Revision::UNRECORDED)
        .await
        .unwrap();

    assert_eq!(finished.stocked, 1);
    assert_eq!(finished.still_pending, 0);
    let created = h.purchases.created_stock();
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].level, StockLevel::Exact { quantity: ml(1000) });
}

#[tokio::test]
async fn changing_your_mind_mid_shop_is_allowed_at_every_step() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Sample Whole Milk", milk);
    let b = mapped("Sample Value Whole Milk", milk);
    h.products.seed(a.clone());
    h.products.seed(b.clone());

    let purchase = h
        .shopping
        .record_purchase(
            NewPurchase {
                ingredient_id: Some(milk),
                prepared_meal_id: None,
                product_id: Some(a.id),
                name: None,
                quantity: Some(ml(1000)),
                opportunity_date: Some(date!(2026 - 09 - 05)),
                note: None,
            },
            h.actor_id,
        )
        .await
        .unwrap();

    let swapped = h
        .shopping
        .update_purchase(
            purchase.id,
            purchase.revision,
            PurchasePatch {
                product_id: Some(b.id),
                quantity: Some(ml(2000)),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(swapped.product_id, Some(b.id));
    assert_eq!(swapped.quantity, Some(ml(2000)));
    assert_eq!(swapped.state, PurchaseState::Pending);

    let dropped = h
        .shopping
        .update_purchase(
            swapped.id,
            swapped.revision,
            PurchasePatch {
                cancelled: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(dropped.state, PurchaseState::Cancelled);
    assert!(h.purchases.created_stock().is_empty());
}

#[tokio::test]
async fn two_products_can_answer_one_requirement() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Sample Whole Milk", milk);
    let b = mapped("Sample Value Whole Milk", milk);
    h.products.seed(a.clone());
    h.products.seed(b.clone());
    plan_product(&h, a.id, ml(400), date!(2026 - 09 - 06)).await;

    for (product, amount) in [(&a, ml(500)), (&b, ml(750))] {
        h.shopping
            .record_purchase(
                NewPurchase {
                    ingredient_id: Some(milk),
                    prepared_meal_id: None,
                    product_id: Some(product.id),
                    name: None,
                    quantity: Some(amount),
                    opportunity_date: Some(date!(2026 - 09 - 05)),
                    note: None,
                },
                h.actor_id,
            )
            .await
            .unwrap();
    }

    let list = h.shopping.requirements(None).await.unwrap();
    assert_eq!(list.requirements[0].purchases.len(), 2);

    let finished = h
        .shopping
        .finish_shop(date!(2026 - 09 - 05), h.actor_id, Revision::UNRECORDED)
        .await
        .unwrap();

    assert_eq!(finished.stocked, 2);
    assert_eq!(h.purchases.created_stock().len(), 2);
}

#[tokio::test]
async fn finishing_leaves_a_purchase_with_no_details_waiting() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    h.products.seed(mapped("Sample Whole Milk", milk));

    h.shopping
        .record_purchase(
            NewPurchase {
                ingredient_id: Some(milk),
                prepared_meal_id: None,
                product_id: None,
                name: None,
                quantity: None,
                opportunity_date: Some(date!(2026 - 09 - 05)),
                note: None,
            },
            h.actor_id,
        )
        .await
        .unwrap();

    let finished = h
        .shopping
        .finish_shop(date!(2026 - 09 - 05), h.actor_id, Revision::UNRECORDED)
        .await
        .unwrap();

    assert_eq!(finished.stocked, 0);
    assert_eq!(finished.still_pending, 1);
    assert!(h.purchases.created_stock().is_empty());
    assert_eq!(h.shopping.pending_purchases().await.unwrap().len(), 1);
}

#[tokio::test]
async fn finishing_a_shop_twice_changes_nothing_and_locks_what_it_stocked() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Sample Whole Milk", milk);
    h.products.seed(a.clone());

    let purchase = h
        .shopping
        .record_purchase(
            NewPurchase {
                ingredient_id: Some(milk),
                prepared_meal_id: None,
                product_id: Some(a.id),
                name: None,
                quantity: Some(ml(1000)),
                opportunity_date: Some(date!(2026 - 09 - 05)),
                note: None,
            },
            h.actor_id,
        )
        .await
        .unwrap();

    h.shopping
        .finish_shop(date!(2026 - 09 - 05), h.actor_id, Revision::UNRECORDED)
        .await
        .unwrap();
    let again = h
        .shopping
        .finish_shop(date!(2026 - 09 - 05), h.actor_id, Revision::UNRECORDED)
        .await
        .unwrap();

    assert_eq!(again.stocked, 0);
    assert_eq!(h.purchases.created_stock().len(), 1);

    let stocked = h.purchases.get(purchase.id).await.unwrap().unwrap();
    let refused = h
        .shopping
        .update_purchase(
            stocked.id,
            stocked.revision,
            PurchasePatch {
                quantity: Some(ml(1)),
                ..Default::default()
            },
        )
        .await;
    assert!(refused.is_err(), "a finished purchase is the stock record");
}

#[tokio::test]
async fn skipping_the_next_shop_moves_the_list_to_the_one_after() {
    let h = harness();
    weekly_saturdays(&h).await;

    let before = h.shopping.requirements(None).await.unwrap();
    assert_eq!(before.focus, Some(date!(2026 - 09 - 05)));

    h.shopping
        .skip_opportunity(date!(2026 - 09 - 05), Revision::UNRECORDED)
        .await
        .unwrap();

    let after = h.shopping.requirements(None).await.unwrap();
    assert_eq!(after.focus, Some(date!(2026 - 09 - 12)));
}

#[tokio::test]
async fn restoring_a_one_off_removes_it_without_changing_the_cadence() {
    let h = harness();
    weekly_saturdays(&h).await;
    h.shopping
        .add_one_off(
            date!(2026 - 09 - 02),
            Some("Extra shop".to_owned()),
            Revision::UNRECORDED,
        )
        .await
        .unwrap();
    let one_off = h
        .shopping
        .opportunities(TODAY, date!(2026 - 09 - 06))
        .await
        .unwrap()
        .into_iter()
        .find(|opportunity| opportunity.date == date!(2026 - 09 - 02))
        .unwrap();

    h.shopping
        .restore_opportunity(one_off.date, one_off.revision)
        .await
        .unwrap();

    let dates: Vec<_> = h
        .shopping
        .opportunities(TODAY, date!(2026 - 09 - 13))
        .await
        .unwrap()
        .into_iter()
        .map(|opportunity| opportunity.date)
        .collect();
    assert_eq!(dates, vec![date!(2026 - 09 - 05), date!(2026 - 09 - 12)]);
}

#[tokio::test]
async fn dismissing_a_suggestion_only_hides_it_for_that_shop() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let bottle = mapped("Sample Whole Milk", milk);
    h.products.seed(bottle.clone());
    plan_product(&h, bottle.id, ml(400), date!(2026 - 09 - 08)).await;
    plan_product(&h, bottle.id, ml(400), date!(2026 - 09 - 15)).await;

    h.shopping
        .dismiss_suggestion(date!(2026 - 09 - 05), DemandSubject::ingredient(milk))
        .await
        .unwrap();

    assert!(
        h.shopping
            .requirements(Some(date!(2026 - 09 - 05)))
            .await
            .unwrap()
            .requirements
            .is_empty()
    );
    assert_eq!(
        h.shopping
            .requirements(Some(date!(2026 - 09 - 12)))
            .await
            .unwrap()
            .requirements
            .len(),
        1
    );
}

#[tokio::test]
async fn with_no_cadence_nothing_is_assigned_to_a_shop() {
    let h = harness();
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Sample Whole Milk", milk);
    h.products.seed(a.clone());
    add_stock(&h, a.id, ml(100)).await;
    plan_product(&h, a.id, ml(400), date!(2026 - 09 - 02)).await;

    let list = h.shopping.requirements(None).await.unwrap();

    assert!(!list.cadence_configured);
    assert_eq!(list.focus, None);
    assert_eq!(list.requirements.len(), 1);
    assert_eq!(list.requirements[0].assignment, Assignment::Unassigned);
}

#[tokio::test]
async fn something_needed_before_any_shop_asks_for_an_earlier_one() {
    let h = harness();
    weekly_saturdays(&h).await;
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Sample Whole Milk", milk);
    h.products.seed(a.clone());
    plan_product(&h, a.id, ml(400), date!(2026 - 09 - 01)).await;

    let list = h.shopping.requirements(None).await.unwrap();

    assert_eq!(
        list.requirements[0].assignment,
        Assignment::NeedsEarlierOpportunity
    );
}
