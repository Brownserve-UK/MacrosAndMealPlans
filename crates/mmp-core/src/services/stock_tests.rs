use std::sync::Arc;

use rust_decimal::Decimal;
use time::OffsetDateTime;
use time::macros::{date, datetime};

use super::*;
use crate::domain::{
    Availability, Confidence, ConsumedAmount, HouseholdMember, HouseholdMemberId, MealItemRef,
    MealPlanComponent, MealPlanEntry, MealPlanStatus, MealSlot, NewStockItem, Product, ProductId,
    Provenance, Quantity, Revision, StockLevel, StorageLocation, Unit, UserId,
};
use crate::ports::{FixedClock, MealPlanRepository, StockQuery};
use crate::testing::{
    InMemoryHouseholdMemberRepository, InMemoryHouseholdSettingsRepository,
    InMemoryMealPlanRepository, InMemoryProductRepository, InMemoryStockRepository,
};

struct Harness {
    service: StockService,
    stock: InMemoryStockRepository,
    products: InMemoryProductRepository,
    meal_plans: InMemoryMealPlanRepository,
    member_id: HouseholdMemberId,
    actor_id: UserId,
}

fn harness() -> Harness {
    let stock = InMemoryStockRepository::new();
    let products = InMemoryProductRepository::new();
    let meal_plans = InMemoryMealPlanRepository::default();
    let members = InMemoryHouseholdMemberRepository::new();
    let settings = InMemoryHouseholdSettingsRepository::new();
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
    let service = StockService::new(
        Arc::new(stock.clone()),
        Arc::new(products.clone()),
        Arc::new(meal_plans.clone()),
        Arc::new(members),
        Arc::new(settings),
        Arc::new(FixedClock::new(datetime!(2026-08-24 09:00 UTC))),
    );
    Harness {
        service,
        stock,
        products,
        meal_plans,
        member_id,
        actor_id: UserId::new(),
    }
}

fn product() -> Product {
    let now = OffsetDateTime::UNIX_EPOCH;
    Product {
        id: ProductId::new(),
        name: "Chicken breast".to_owned(),
        brand: None,
        barcode: None,
        retailer: None,
        shopping_section: None,
        package_quantity: Some(Quantity::new(Decimal::new(1000, 0), Unit::Gram)),
        servings_per_pack: Some(4),
        mapped_ingredient_id: None,
        nutrition: Default::default(),
        provenance: Provenance::local(),
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    }
}

fn grams(value: i64) -> Quantity {
    Quantity::new(Decimal::new(value, 0), Unit::Gram)
}

async fn plan_measured(h: &Harness, product_id: ProductId, g: i64) {
    let now = OffsetDateTime::UNIX_EPOCH;
    let entry = MealPlanEntry {
        id: crate::domain::MealPlanEntryId::new(),
        scope: crate::domain::MealPlanScope::Member,
        member_id: Some(h.member_id),
        planned_on: date!(2026 - 08 - 25),
        planned_time: None,
        slot: MealSlot::Dinner,
        status: MealPlanStatus::Planned,
        components: vec![MealPlanComponent {
            id: crate::domain::MealPlanComponentId::new(),
            item: MealItemRef::product(product_id),
            amount: ConsumedAmount::Measure(grams(g)),
            position: 0,
            snapshot: None,
            status: MealPlanStatus::Planned,
            resolved_by: None,
            resolved_at: None,
            revision: Revision::INITIAL,
            display_order: uuid::Uuid::nil(),
        }],
        participants: Vec::new(),
        guest_groups: Vec::new(),
        created_by: h.actor_id,
        updated_by: h.actor_id,
        resolved_by: None,
        resolved_at: None,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
    };
    h.meal_plans.insert(&entry).await.unwrap();
}

fn new_item(product_id: ProductId, level: StockLevel) -> NewStockItem {
    NewStockItem {
        product_id,
        level,
        storage_location: StorageLocation::Chilled,
        source_date: None,
        usability_deadline: None,
        note: None,
    }
}

#[tokio::test]
async fn create_writes_an_item_and_an_audit_event() {
    let h = harness();
    let p = product();
    h.products.seed(p.clone());

    let created = h
        .service
        .create(
            new_item(
                p.id,
                StockLevel::Exact {
                    quantity: grams(400),
                },
            ),
            h.actor_id,
            Some(h.member_id),
        )
        .await
        .unwrap();

    assert_eq!(created.revision, Revision::INITIAL);
    assert_eq!(h.stock.count(), 1);
    assert_eq!(h.stock.event_count(), 1);
    let events = h.service.events(created.id).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].actor_user_id, Some(h.actor_id));
    assert_eq!(events[0].subject_member_id, Some(h.member_id));
}

#[tokio::test]
async fn planned_demand_reduces_available_stock() {
    let h = harness();
    let p = product();
    h.products.seed(p.clone());
    h.service
        .create(
            new_item(
                p.id,
                StockLevel::Exact {
                    quantity: grams(1000),
                },
            ),
            h.actor_id,
            None,
        )
        .await
        .unwrap();
    plan_measured(&h, p.id, 250).await;
    plan_measured(&h, p.id, 250).await;

    let result = h
        .service
        .availability(&[p.id], date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    match &result[0].availability {
        Availability::Quantified {
            on_hand,
            planned_demand,
            unallocated,
            confidence,
        } => {
            assert_eq!(*on_hand, grams(1000));
            assert_eq!(*planned_demand, grams(500));
            assert_eq!(*unallocated, grams(500));
            assert_eq!(*confidence, Confidence::Exact);
        }
        other => panic!("expected a quantified availability, got {other:?}"),
    }
}

#[tokio::test]
async fn estimated_stock_uses_its_lower_bound() {
    let h = harness();
    let p = product();
    h.products.seed(p.clone());
    h.service
        .create(
            new_item(
                p.id,
                StockLevel::Estimated {
                    low: Decimal::new(200, 0),
                    high: Decimal::new(600, 0),
                    unit: Unit::Gram,
                },
            ),
            h.actor_id,
            None,
        )
        .await
        .unwrap();

    let result = h
        .service
        .availability(&[p.id], date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    match &result[0].availability {
        Availability::Quantified {
            on_hand,
            confidence,
            ..
        } => {
            assert_eq!(*on_hand, grams(200));
            assert_eq!(*confidence, Confidence::Estimated);
        }
        other => panic!("expected a quantified availability, got {other:?}"),
    }
}

#[tokio::test]
async fn a_not_tracked_item_is_assumed_available_and_never_short() {
    let h = harness();
    let p = product();
    h.products.seed(p.clone());
    h.service
        .create(new_item(p.id, StockLevel::NotTracked), h.actor_id, None)
        .await
        .unwrap();
    plan_measured(&h, p.id, 5000).await;

    let result = h
        .service
        .availability(&[p.id], date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    assert_eq!(result[0].availability, Availability::AssumedAvailable);
    assert!(!result[0].availability.is_short());
}

#[tokio::test]
async fn a_product_with_no_stock_record_is_unknown_by_default() {
    let h = harness();
    let p = product();
    h.products.seed(p.clone());

    let result = h
        .service
        .availability(&[p.id], date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    assert_eq!(result[0].availability, Availability::Unknown);
}

#[tokio::test]
async fn a_recipe_component_in_the_horizon_marks_demand_incomplete() {
    let h = harness();
    let p = product();
    h.products.seed(p.clone());
    h.service
        .create(
            new_item(
                p.id,
                StockLevel::Exact {
                    quantity: grams(1000),
                },
            ),
            h.actor_id,
            None,
        )
        .await
        .unwrap();

    let now = OffsetDateTime::UNIX_EPOCH;
    let entry = MealPlanEntry {
        id: crate::domain::MealPlanEntryId::new(),
        scope: crate::domain::MealPlanScope::Member,
        member_id: Some(h.member_id),
        planned_on: date!(2026 - 08 - 25),
        planned_time: None,
        slot: MealSlot::Lunch,
        status: MealPlanStatus::Planned,
        components: vec![MealPlanComponent {
            id: crate::domain::MealPlanComponentId::new(),
            item: MealItemRef::recipe(crate::domain::RecipeId::new()),
            amount: ConsumedAmount::Servings(Decimal::new(1, 0)),
            position: 0,
            snapshot: None,
            status: MealPlanStatus::Planned,
            resolved_by: None,
            resolved_at: None,
            revision: Revision::INITIAL,
            display_order: uuid::Uuid::nil(),
        }],
        participants: Vec::new(),
        guest_groups: Vec::new(),
        created_by: h.actor_id,
        updated_by: h.actor_id,
        resolved_by: None,
        resolved_at: None,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
    };
    h.meal_plans.insert(&entry).await.unwrap();

    let result = h
        .service
        .availability(&[p.id], date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    assert!(result[0].demand_incomplete);
}

#[tokio::test]
async fn updating_a_quantity_bumps_revision_and_records_an_event() {
    let h = harness();
    let p = product();
    h.products.seed(p.clone());
    let created = h
        .service
        .create(
            new_item(
                p.id,
                StockLevel::Exact {
                    quantity: grams(400),
                },
            ),
            h.actor_id,
            None,
        )
        .await
        .unwrap();

    let patch = crate::domain::StockItemPatch {
        level: Some(StockLevel::Exact {
            quantity: grams(150),
        }),
        ..Default::default()
    };
    let updated = h
        .service
        .update(
            created.id,
            created.revision,
            patch,
            h.actor_id,
            Some(h.member_id),
        )
        .await
        .unwrap();

    assert_eq!(updated.revision, created.revision.next());
    assert_eq!(h.stock.event_count(), 2);
    let listed = h
        .service
        .list(&StockQuery {
            product_id: Some(p.id),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(listed.items.len(), 1);
}
