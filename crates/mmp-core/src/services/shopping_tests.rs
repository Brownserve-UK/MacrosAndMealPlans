use std::sync::Arc;

use rust_decimal::Decimal;
use time::OffsetDateTime;
use time::macros::{date, datetime};

use super::*;
use crate::domain::{
    ConsumedAmount, HouseholdMember, HouseholdMemberId, Ingredient, IngredientId, MealItemRef,
    MealPlanComponent, MealPlanEntry, MealSlot, MissingStockInterpretation, NewStockItem, Product,
    ProductId, Provenance, Quantity, Revision, StockLevel, StorageLocation, Unit, UserId,
};
use crate::ports::{Clock, FixedClock, MealPlanRepository};
use crate::testing::{
    InMemoryHouseholdMemberRepository, InMemoryHouseholdSettingsRepository,
    InMemoryIngredientRepository, InMemoryMealPlanRepository, InMemoryProductRepository,
    InMemoryPurchaseRepository, InMemoryRecipeRepository, InMemoryShoppingCadenceRepository,
    InMemoryShoppingOpportunityRepository, InMemoryStockRepository,
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
    let member_id = HouseholdMemberId::new();
    let now = OffsetDateTime::UNIX_EPOCH;
    members.seed(HouseholdMember {
        id: member_id,
        display_name: "Sample".to_owned(),
        linked_user_id: None,
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
        Arc::new(meal_plans.clone()),
        Arc::new(recipes),
        Arc::new(members),
        Arc::new(settings.clone()),
        clock.clone(),
    );
    let shopping = ShoppingService::new(
        Arc::new(cadence),
        Arc::new(opportunities),
        Arc::new(purchases.clone()),
        Arc::new(ingredients.clone()),
        Arc::new(products.clone()),
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
    let now = OffsetDateTime::UNIX_EPOCH;
    h.ingredients.seed(Ingredient {
        id,
        name: name.to_owned(),
        default_unit: Unit::Millilitre,
        shopping_section: Some(ShoppingSection::Dairy),
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
                product_id,
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
        portioning: crate::domain::Portioning::Equal,
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

async fn weekly_saturdays(h: &Harness) {
    h.shopping
        .set_cadence(NewShoppingCadence {
            interval_weeks: 1,
            days: vec![Weekday::Saturday],
            anchor: TODAY,
            usual_time: None,
        })
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
                product_id: a.id,
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
                product_id: None,
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
    assert!(requirement.purchase.is_some());
}

#[tokio::test]
async fn completing_a_purchase_creates_stock_at_the_amount_actually_bought() {
    let h = harness();
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Sample Whole Milk", milk);
    h.products.seed(a.clone());

    let purchase = h
        .shopping
        .record_purchase(
            NewPurchase {
                ingredient_id: Some(milk),
                product_id: None,
                quantity: None,
                opportunity_date: None,
                note: None,
            },
            h.actor_id,
        )
        .await
        .unwrap();

    let completed = h
        .shopping
        .update_purchase(
            purchase.id,
            purchase.revision,
            PurchasePatch {
                product_id: Some(a.id),
                quantity: Some(ml(2000)),
                ..Default::default()
            },
            h.actor_id,
        )
        .await
        .unwrap();

    assert_eq!(completed.state, PurchaseState::Reconciled);
    let created = h.purchases.created_stock();
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].level, StockLevel::Exact { quantity: ml(2000) });
    assert_eq!(completed.stock_item_id, Some(created[0].id));
}

#[tokio::test]
async fn buying_with_full_details_goes_straight_into_stock() {
    let h = harness();
    let milk = IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Sample Whole Milk", milk);
    h.products.seed(a.clone());

    let purchase = h
        .shopping
        .record_purchase(
            NewPurchase {
                ingredient_id: Some(milk),
                product_id: Some(a.id),
                quantity: Some(ml(1000)),
                opportunity_date: None,
                note: None,
            },
            h.actor_id,
        )
        .await
        .unwrap();

    assert_eq!(purchase.state, PurchaseState::Reconciled);
    assert!(purchase.stock_item_id.is_some());
    assert_eq!(h.purchases.created_stock().len(), 1);
}

#[tokio::test]
async fn skipping_the_next_shop_moves_the_list_to_the_one_after() {
    let h = harness();
    weekly_saturdays(&h).await;

    let before = h.shopping.requirements(None).await.unwrap();
    assert_eq!(before.focus, Some(date!(2026 - 09 - 05)));

    h.shopping
        .skip_opportunity(date!(2026 - 09 - 05))
        .await
        .unwrap();

    let after = h.shopping.requirements(None).await.unwrap();
    assert_eq!(after.focus, Some(date!(2026 - 09 - 12)));
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
