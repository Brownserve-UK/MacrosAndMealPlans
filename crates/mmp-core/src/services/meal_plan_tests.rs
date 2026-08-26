use std::sync::Arc;

use rust_decimal::Decimal;
use time::OffsetDateTime;
use time::macros::{date, datetime, time};

use super::*;
use crate::domain::{
    ActualMealPlanComponent, ConsumedAmount, HouseholdMemberId, MealPlanEntryPatch, MealPlanStatus,
    MealSlot, NewConsumptionRecord, NewMealPlanComponent, NewMealPlanEntry, NewNutritionTarget,
    NutritionFacts, NutritionGoals, Product, ProductId, Provenance, Quantity, Revision, Unit,
    UserId,
};
use crate::ports::FixedClock;
use crate::services::{DiaryService, NutritionTargetService};
use crate::testing::{
    InMemoryConsumptionRecordRepository, InMemoryMealPlanRepository,
    InMemoryNutritionTargetRepository, InMemoryProductRepository,
};

struct Harness {
    service: MealPlanService,
    diary: DiaryService,
    targets: NutritionTargetService,
    products: InMemoryProductRepository,
    records: InMemoryConsumptionRecordRepository,
    member_id: HouseholdMemberId,
    actor_id: UserId,
}

fn harness() -> Harness {
    let products = InMemoryProductRepository::new();
    let records = InMemoryConsumptionRecordRepository::new();
    let target_repo = InMemoryNutritionTargetRepository::new();
    let clock = Arc::new(FixedClock::new(datetime!(2026-08-24 09:00 UTC)));
    let service = MealPlanService::new(
        Arc::new(InMemoryMealPlanRepository::new(records.clone())),
        Arc::new(products.clone()),
        Arc::new(records.clone()),
        Arc::new(target_repo.clone()),
        clock.clone(),
    );
    let diary = DiaryService::new(
        Arc::new(records.clone()),
        Arc::new(products.clone()),
        clock.clone(),
    );
    let targets = NutritionTargetService::new(Arc::new(target_repo.clone()), clock);
    Harness {
        service,
        diary,
        targets,
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

async fn set_target(h: &Harness, effective: time::Date, goals: NutritionGoals) {
    h.targets
        .create(NewNutritionTarget {
            member_id: h.member_id,
            effective_from: effective,
            goals,
        })
        .await
        .unwrap();
}

fn kcal_goals(value: i64) -> NutritionGoals {
    NutritionGoals {
        energy_kcal: Some(Decimal::new(value, 0)),
        ..Default::default()
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
            slot: MealSlot::Lunch,
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
    assert_eq!(record.slot, entry.entry.slot);

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
        .amend(
            amended.id,
            amended.revision,
            crate::domain::ConsumptionRecordPatch {
                slot: Some(MealSlot::Lunch),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(error, CoreError::Conflict { .. }));

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

#[tokio::test]
async fn reopening_an_eaten_entry_removes_its_diary_records() {
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
    assert_eq!(h.records.count(), 1);

    let reopened = h
        .service
        .reopen(confirmed.entry.id, confirmed.entry.revision, h.actor_id)
        .await
        .unwrap();

    assert_eq!(reopened.entry.status, MealPlanStatus::Planned);
    assert!(reopened.entry.resolved_by.is_none());
    assert!(reopened.entry.resolved_at.is_none());
    assert_eq!(h.records.count(), 0);
    assert!(reopened.components[0].consumption_record.is_none());
}

#[tokio::test]
async fn reopening_a_not_eaten_entry_returns_it_to_planned() {
    let h = harness();
    let food = product("Food", 200);
    h.products.seed(food.clone());
    let entry = planned(&h, vec![measured(food.id, 100)]).await;
    let resolved = h
        .service
        .mark_not_eaten(entry.entry.id, entry.entry.revision, h.actor_id)
        .await
        .unwrap();

    let reopened = h
        .service
        .reopen(resolved.entry.id, resolved.entry.revision, h.actor_id)
        .await
        .unwrap();

    assert_eq!(reopened.entry.status, MealPlanStatus::Planned);
    let week = h
        .service
        .week(h.member_id, date!(2026 - 08 - 24))
        .await
        .unwrap();
    assert_eq!(
        week.remaining_planned.nutrition.energy_kcal,
        Some(Decimal::new(200, 0))
    );
}

#[tokio::test]
async fn a_still_planned_entry_cannot_be_reopened() {
    let h = harness();
    let food = product("Food", 200);
    h.products.seed(food.clone());
    let entry = planned(&h, vec![measured(food.id, 100)]).await;

    let error = h
        .service
        .reopen(entry.entry.id, entry.entry.revision, h.actor_id)
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Conflict { .. }));
}

#[tokio::test]
async fn reopening_with_a_stale_revision_is_refused() {
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
        .reopen(resolved.entry.id, entry.entry.revision, h.actor_id)
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::RevisionMismatch { .. }));
}

#[tokio::test]
async fn a_target_in_force_all_week_resolves_per_day_and_sums_the_week() {
    let h = harness();
    set_target(&h, date!(2026 - 08 - 01), kcal_goals(2000)).await;

    let week = h
        .service
        .week(h.member_id, date!(2026 - 08 - 24))
        .await
        .unwrap();

    for day in &week.days {
        assert_eq!(
            day.target.as_ref().and_then(|goals| goals.energy_kcal),
            Some(Decimal::new(2000, 0))
        );
    }
    assert_eq!(
        week.target.as_ref().and_then(|goals| goals.energy_kcal),
        Some(Decimal::new(14000, 0))
    );
    assert!(week.insufficient_target_coverage.is_empty());
}

#[tokio::test]
async fn a_target_starting_midweek_is_not_enough_data_for_the_week() {
    let h = harness();
    set_target(&h, date!(2026 - 08 - 26), kcal_goals(2000)).await;

    let week = h
        .service
        .week(h.member_id, date!(2026 - 08 - 24))
        .await
        .unwrap();

    assert!(week.days[0].target.is_none());
    assert!(week.days[1].target.is_none());
    assert_eq!(
        week.days[2]
            .target
            .as_ref()
            .and_then(|goals| goals.energy_kcal),
        Some(Decimal::new(2000, 0))
    );
    assert!(week.target.is_none());
    assert_eq!(
        week.insufficient_target_coverage,
        vec!["energy_kcal".to_owned()]
    );
}

#[tokio::test]
async fn a_target_change_sums_energy_but_flags_a_newly_added_nutrient() {
    let h = harness();
    set_target(&h, date!(2026 - 08 - 01), kcal_goals(2000)).await;
    set_target(
        &h,
        date!(2026 - 08 - 27),
        NutritionGoals {
            energy_kcal: Some(Decimal::new(1800, 0)),
            protein_g: Some(Decimal::new(120, 0)),
            ..Default::default()
        },
    )
    .await;

    let week = h
        .service
        .week(h.member_id, date!(2026 - 08 - 24))
        .await
        .unwrap();

    assert_eq!(
        week.target.as_ref().and_then(|goals| goals.energy_kcal),
        Some(Decimal::new(13200, 0))
    );
    assert_eq!(week.target.as_ref().and_then(|goals| goals.protein_g), None);
    assert_eq!(
        week.insufficient_target_coverage,
        vec!["protein_g".to_owned()]
    );
}

#[tokio::test]
async fn a_reopened_entry_can_be_edited_and_confirmed_again() {
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
    let reopened = h
        .service
        .reopen(confirmed.entry.id, confirmed.entry.revision, h.actor_id)
        .await
        .unwrap();

    let edited = h
        .service
        .update(
            reopened.entry.id,
            reopened.entry.revision,
            MealPlanEntryPatch {
                components: Some(vec![measured(food.id, 150)]),
                ..Default::default()
            },
            h.actor_id,
        )
        .await
        .unwrap();

    let reconfirmed = h
        .service
        .mark_eaten(
            edited.entry.id,
            edited.entry.revision,
            ConfirmMealPlanEntry {
                consumed_on: date!(2026 - 08 - 26),
                consumed_at: datetime!(2026-08-26 18:30 UTC),
                components: vec![ActualMealPlanComponent {
                    component_id: edited.components[0].component.id,
                    amount: edited.components[0].component.amount,
                }],
                actor_id: h.actor_id,
            },
        )
        .await
        .unwrap();

    assert_eq!(reconfirmed.entry.status, MealPlanStatus::Eaten);
    assert_eq!(h.records.count(), 1);
}

#[tokio::test]
async fn date_policy_forbids_creating_a_plan_in_the_past() {
    let h = harness();
    let food = product("Food", 200);
    h.products.seed(food.clone());
    let error = h
        .service
        .create(NewMealPlanEntry {
            id: None,
            member_id: h.member_id,
            planned_on: date!(2026 - 08 - 20),
            planned_time: None,
            slot: MealSlot::Dinner,
            components: vec![measured(food.id, 100)],
            actor_id: h.actor_id,
        })
        .await
        .unwrap_err();
    assert!(matches!(error, CoreError::Validation(_)));
}

#[tokio::test]
async fn date_policy_allows_a_one_day_grace_into_the_past() {
    let h = harness();
    let food = product("Food", 200);
    h.products.seed(food.clone());
    h.service
        .create(NewMealPlanEntry {
            id: None,
            member_id: h.member_id,
            planned_on: date!(2026 - 08 - 23),
            planned_time: None,
            slot: MealSlot::Dinner,
            components: vec![measured(food.id, 100)],
            actor_id: h.actor_id,
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn date_policy_forbids_moving_a_plan_into_the_past() {
    let h = harness();
    let food = product("Food", 200);
    h.products.seed(food.clone());
    let entry = planned(&h, vec![measured(food.id, 100)]).await;

    let error = h
        .service
        .update(
            entry.entry.id,
            entry.entry.revision,
            MealPlanEntryPatch {
                planned_on: Some(date!(2026 - 08 - 20)),
                ..Default::default()
            },
            h.actor_id,
        )
        .await
        .unwrap_err();
    assert!(matches!(error, CoreError::Validation(_)));
}

#[tokio::test]
async fn date_policy_forbids_resolving_a_plan_that_is_not_yet_due() {
    let h = harness();
    let food = product("Food", 200);
    h.products.seed(food.clone());
    let entry = h
        .service
        .create(NewMealPlanEntry {
            id: None,
            member_id: h.member_id,
            planned_on: date!(2026 - 08 - 30),
            planned_time: None,
            slot: MealSlot::Dinner,
            components: vec![measured(food.id, 100)],
            actor_id: h.actor_id,
        })
        .await
        .unwrap();

    let error = h
        .service
        .mark_eaten(
            entry.entry.id,
            entry.entry.revision,
            ConfirmMealPlanEntry {
                consumed_on: date!(2026 - 08 - 30),
                consumed_at: datetime!(2026-08-30 18:30 UTC),
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
        .unwrap_err();
    assert!(matches!(error, CoreError::Conflict { .. }));
}

#[tokio::test]
async fn the_week_projects_a_planned_item_and_moves_it_once_eaten() {
    let h = harness();
    let food = product("Food", 200);
    h.products.seed(food.clone());
    let entry = planned(&h, vec![measured(food.id, 100)]).await;

    let week = h
        .service
        .week(h.member_id, date!(2026 - 08 - 24))
        .await
        .unwrap();
    let planned_day = week
        .days
        .iter()
        .find(|day| day.date == date!(2026 - 08 - 25))
        .unwrap();
    let dinner = planned_day
        .slots
        .iter()
        .find(|slot| slot.slot == MealSlot::Dinner)
        .unwrap();
    assert_eq!(dinner.items.len(), 1);
    assert_eq!(dinner.items[0].status, MealPlanStatus::Planned);
    assert_eq!(dinner.items[0].product_name, "Food");

    h.service
        .mark_eaten(
            entry.entry.id,
            entry.entry.revision,
            ConfirmMealPlanEntry {
                consumed_on: date!(2026 - 08 - 26),
                consumed_at: datetime!(2026-08-26 19:00 UTC),
                components: vec![ActualMealPlanComponent {
                    component_id: entry.components[0].component.id,
                    amount: ConsumedAmount::Measure(Quantity::new(
                        Decimal::new(150, 0),
                        Unit::Gram,
                    )),
                }],
                actor_id: h.actor_id,
            },
        )
        .await
        .unwrap();

    let week = h
        .service
        .week(h.member_id, date!(2026 - 08 - 24))
        .await
        .unwrap();
    let planned_day = week
        .days
        .iter()
        .find(|day| day.date == date!(2026 - 08 - 25))
        .unwrap();
    let dinner = planned_day
        .slots
        .iter()
        .find(|slot| slot.slot == MealSlot::Dinner)
        .unwrap();
    assert!(dinner.items.is_empty());

    let eaten_day = week
        .days
        .iter()
        .find(|day| day.date == date!(2026 - 08 - 26))
        .unwrap();
    let dinner = eaten_day
        .slots
        .iter()
        .find(|slot| slot.slot == MealSlot::Dinner)
        .unwrap();
    assert_eq!(dinner.items.len(), 1);
    let item = &dinner.items[0];
    assert_eq!(item.status, MealPlanStatus::Eaten);
    assert_eq!(item.planned_on, Some(date!(2026 - 08 - 25)));
    assert_eq!(
        item.amount,
        ConsumedAmount::Measure(Quantity::new(Decimal::new(150, 0), Unit::Gram))
    );
    assert_eq!(
        item.planned_amount,
        Some(ConsumedAmount::Measure(Quantity::new(
            Decimal::new(100, 0),
            Unit::Gram
        )))
    );
}

#[tokio::test]
async fn the_week_projects_directly_logged_food_on_its_own_date() {
    let h = harness();
    let food = product("Food", 200);
    h.products.seed(food.clone());
    h.diary
        .record(NewConsumptionRecord {
            id: None,
            member_id: h.member_id,
            product_id: food.id,
            recorded_by: Some(h.actor_id),
            meal_plan_entry_id: None,
            meal_plan_component_id: None,
            slot: MealSlot::Snacks,
            amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(30, 0), Unit::Gram)),
            consumed_on: date!(2026 - 08 - 24),
            consumed_at: Some(datetime!(2026-08-24 15:00 UTC)),
        })
        .await
        .unwrap();

    let week = h
        .service
        .week(h.member_id, date!(2026 - 08 - 24))
        .await
        .unwrap();
    let day = week
        .days
        .iter()
        .find(|day| day.date == date!(2026 - 08 - 24))
        .unwrap();
    let snacks = day
        .slots
        .iter()
        .find(|slot| slot.slot == MealSlot::Snacks)
        .unwrap();
    assert_eq!(snacks.items.len(), 1);
    assert_eq!(snacks.items[0].product_name, "Food");
    assert!(matches!(
        snacks.items[0].source,
        MealItemSource::Logged { .. }
    ));
}
