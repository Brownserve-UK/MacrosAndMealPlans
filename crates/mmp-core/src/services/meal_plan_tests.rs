use std::sync::Arc;

use rust_decimal::Decimal;
use time::OffsetDateTime;
use time::macros::{date, datetime, time};

use super::*;
use crate::domain::{
    ActualMealPlanComponent, ConsumedAmount, HouseholdMemberId, MealPlanEntryPatch, MealPlanStatus,
    MealSlot, NewConsumptionRecord, NewMealPlanComponent, NewMealPlanEntry, NutritionFacts,
    Product, ProductId, Provenance, Quantity, Revision, Unit, UserId,
};
use crate::ports::FixedClock;
use crate::services::DiaryService;
use crate::testing::{
    InMemoryConsumptionRecordRepository, InMemoryMealPlanRepository, InMemoryProductRepository,
};

struct Harness {
    service: MealPlanService,
    diary: DiaryService,
    products: InMemoryProductRepository,
    records: InMemoryConsumptionRecordRepository,
    member_id: HouseholdMemberId,
    actor_id: UserId,
}

fn harness() -> Harness {
    let products = InMemoryProductRepository::new();
    let records = InMemoryConsumptionRecordRepository::new();
    let clock = Arc::new(FixedClock::new(datetime!(2026-08-24 09:00 UTC)));
    let service = MealPlanService::new(
        Arc::new(InMemoryMealPlanRepository::new(records.clone())),
        Arc::new(products.clone()),
        Arc::new(records.clone()),
        clock.clone(),
    );
    let diary = DiaryService::new(Arc::new(records.clone()), Arc::new(products.clone()), clock);
    Harness {
        service,
        diary,
        products,
        records,
        member_id: HouseholdMemberId::new(),
        actor_id: UserId::new(),
    }
}

fn product(name: &str, energy_per_100g: i64) -> Product {
    let now = OffsetDateTime::now_utc();
    Product {
        id: ProductId::new(),
        name: name.to_owned(),
        brand: None,
        barcode: None,
        retailer: None,
        shopping_section: None,
        package_quantity: Some(Quantity::new(Decimal::new(500, 0), Unit::Gram)),
        servings_per_pack: Some(5),
        mapped_ingredient_id: None,
        nutrition: NutritionFacts {
            basis: Some(Quantity::new(Decimal::new(100, 0), Unit::Gram)),
            energy_kcal: Some(Decimal::new(energy_per_100g, 0)),
            protein_g: Some(Decimal::new(10, 0)),
            carbohydrate_g: Some(Decimal::new(20, 0)),
            fat_g: Some(Decimal::new(5, 0)),
            ..Default::default()
        },
        provenance: Provenance::local(),
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    }
}

fn measured(product_id: ProductId, grams: i64) -> NewMealPlanComponent {
    NewMealPlanComponent {
        product_id,
        amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(grams, 0), Unit::Gram)),
    }
}

async fn planned(h: &Harness, components: Vec<NewMealPlanComponent>) -> MealPlanEntryView {
    h.service
        .create(NewMealPlanEntry {
            id: None,
            member_id: h.member_id,
            planned_on: date!(2026 - 08 - 25),
            planned_time: Some(time!(18:30)),
            slot: MealSlot::Dinner,
            components,
            actor_id: h.actor_id,
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn a_week_projects_every_planned_component() {
    let h = harness();
    let pasta = product("Pasta", 200);
    let sauce = product("Sauce", 100);
    h.products.seed(pasta.clone());
    h.products.seed(sauce.clone());
    planned(&h, vec![measured(pasta.id, 150), measured(sauce.id, 50)]).await;

    let week = h
        .service
        .week(h.member_id, date!(2026 - 08 - 24))
        .await
        .unwrap();

    assert_eq!(week.days.len(), 7);
    assert_eq!(week.actual.nutrition.energy_kcal, None);
    assert_eq!(
        week.remaining_planned.nutrition.energy_kcal,
        Some(Decimal::new(350, 0))
    );
    assert_eq!(
        week.projected.nutrition.energy_kcal,
        Some(Decimal::new(350, 0))
    );
}

#[tokio::test]
async fn weekly_actuals_include_food_logged_outside_the_meal_plan() {
    let h = harness();
    let food = product("Food", 200);
    h.products.seed(food.clone());
    planned(&h, vec![measured(food.id, 100)]).await;
    h.diary
        .record(NewConsumptionRecord {
            id: None,
            member_id: h.member_id,
            product_id: food.id,
            recorded_by: Some(h.actor_id),
            meal_plan_entry_id: None,
            meal_plan_component_id: None,
            amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(50, 0), Unit::Gram)),
            consumed_on: date!(2026 - 08 - 24),
            consumed_at: Some(datetime!(2026-08-24 12:00 UTC)),
        })
        .await
        .unwrap();

    let week = h
        .service
        .week(h.member_id, date!(2026 - 08 - 24))
        .await
        .unwrap();

    assert_eq!(
        week.actual.nutrition.energy_kcal,
        Some(Decimal::new(100, 0))
    );
    assert_eq!(
        week.remaining_planned.nutrition.energy_kcal,
        Some(Decimal::new(200, 0))
    );
    assert_eq!(
        week.projected.nutrition.energy_kcal,
        Some(Decimal::new(300, 0))
    );
}

#[tokio::test]
async fn confirming_eaten_creates_one_linked_diary_record_per_component() {
    let h = harness();
    let first = product("Pasta", 200);
    let second = product("Sauce", 100);
    h.products.seed(first.clone());
    h.products.seed(second.clone());
    let entry = planned(&h, vec![measured(first.id, 150), measured(second.id, 50)]).await;

    let confirmed = h
        .service
        .mark_eaten(
            entry.entry.id,
            entry.entry.revision,
            ConfirmMealPlanEntry {
                consumed_on: date!(2026 - 08 - 26),
                consumed_at: datetime!(2026-08-26 19:15 UTC),
                components: entry
                    .components
                    .iter()
                    .map(|component| ActualMealPlanComponent {
                        component_id: component.component.id,
                        amount: component.component.amount,
                    })
                    .collect(),
                actor_id: h.actor_id,
            },
        )
        .await
        .unwrap();

    assert_eq!(confirmed.entry.status, MealPlanStatus::Eaten);
    assert_eq!(h.records.count(), 2);
    assert!(confirmed.components.iter().all(|component| {
        component.consumption_record.as_ref().is_some_and(|record| {
            record.meal_plan_entry_id == Some(confirmed.entry.id)
                && record.meal_plan_component_id == Some(component.component.id)
        })
    }));
}

#[tokio::test]
async fn a_resolved_entry_keeps_its_product_snapshot() {
    let h = harness();
    let original = product("Original name", 200);
    h.products.seed(original.clone());
    let entry = planned(&h, vec![measured(original.id, 100)]).await;
    let resolved = h
        .service
        .mark_not_eaten(entry.entry.id, entry.entry.revision, h.actor_id)
        .await
        .unwrap();

    let mut changed = original;
    changed.name = "Changed name".to_owned();
    changed.nutrition.energy_kcal = Some(Decimal::new(900, 0));
    h.products.seed(changed);

    let loaded = h.service.get(resolved.entry.id).await.unwrap();
    assert_eq!(loaded.components[0].product_name, "Original name");
    assert_eq!(
        loaded.components[0].nutrition.energy_kcal,
        Some(Decimal::new(200, 0))
    );
    let week = h
        .service
        .week(h.member_id, date!(2026 - 08 - 24))
        .await
        .unwrap();
    assert_eq!(week.remaining_planned.nutrition.energy_kcal, None);
}

#[tokio::test]
async fn resolved_entries_are_locked() {
    let h = harness();
    let food = product("Food", 200);
    h.products.seed(food.clone());
    let entry = planned(&h, vec![measured(food.id, 100)]).await;
    let resolved = h
        .service
        .mark_not_eaten(entry.entry.id, entry.entry.revision, h.actor_id)
        .await
        .unwrap();

    let error = h
        .service
        .update(
            resolved.entry.id,
            resolved.entry.revision,
            MealPlanEntryPatch {
                planned_time: Some(None),
                ..Default::default()
            },
            h.actor_id,
        )
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Conflict { .. }));
}

#[tokio::test]
async fn linked_diary_records_can_be_amended_but_not_deleted() {
    let h = harness();
    let food = product("Food", 200);
    h.products.seed(food.clone());
    let entry = planned(&h, vec![measured(food.id, 100)]).await;
    let confirmed = h
        .service
        .mark_eaten(
            entry.entry.id,
            entry.entry.revision,
            ConfirmMealPlanEntry {
                consumed_on: date!(2026 - 08 - 25),
                consumed_at: datetime!(2026-08-25 18:30 UTC),
                components: vec![ActualMealPlanComponent {
                    component_id: entry.components[0].component.id,
                    amount: entry.components[0].component.amount,
                }],
                actor_id: h.actor_id,
            },
        )
        .await
        .unwrap();
    let record = confirmed.components[0].consumption_record.as_ref().unwrap();

    let amended = h
        .diary
        .amend(
            record.id,
            record.revision,
            crate::domain::ConsumptionRecordPatch {
                amount: Some(ConsumedAmount::Measure(Quantity::new(
                    Decimal::new(120, 0),
                    Unit::Gram,
                ))),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(amended.revision, record.revision.next());

    let error = h
        .diary
        .remove(amended.id, amended.revision)
        .await
        .unwrap_err();
    assert!(matches!(error, CoreError::Conflict { .. }));
}

#[tokio::test]
async fn an_archived_product_may_be_retained_but_not_newly_added() {
    let h = harness();
    let mut food = product("Food", 200);
    h.products.seed(food.clone());
    let entry = planned(&h, vec![measured(food.id, 100)]).await;
    food.archived_at = Some(datetime!(2026-08-24 10:00 UTC));
    h.products.seed(food.clone());

    let retained = h
        .service
        .update(
            entry.entry.id,
            entry.entry.revision,
            MealPlanEntryPatch {
                components: Some(vec![measured(food.id, 120)]),
                ..Default::default()
            },
            h.actor_id,
        )
        .await;
    assert!(retained.is_ok());

    let newly_added = h
        .service
        .create(NewMealPlanEntry {
            id: None,
            member_id: h.member_id,
            planned_on: date!(2026 - 08 - 26),
            planned_time: None,
            slot: MealSlot::Lunch,
            components: vec![measured(food.id, 100)],
            actor_id: h.actor_id,
        })
        .await;
    assert!(matches!(newly_added, Err(CoreError::Validation(_))));
}
