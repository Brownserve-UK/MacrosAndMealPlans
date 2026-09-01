use std::sync::Arc;

use rust_decimal::Decimal;
use time::OffsetDateTime;
use time::macros::{date, datetime, time};

use super::*;
use crate::domain::{
    ActualMealPlanComponent, ConfirmMealPlanComponent, ConsumedAmount, HouseholdMember,
    HouseholdMemberId, MealItemRef, MealPlanEntryPatch, MealPlanScope, MealPlanStatus, MealSlot,
    NewConsumptionRecord, NewMealPlanComponent, NewMealPlanEntry, NewNutritionTarget,
    NutritionFacts, NutritionGoals, NutritionQuality, OutcomeActor, Product, ProductId, Provenance,
    Quantity, Recipe, RecipeComponent, RecipeId, RecipeVisibility, Revision, StockItem, StockLevel,
    StorageLocation, Unit, UserId,
};
use crate::ports::{FixedClock, StockRepository};
use crate::services::{DiaryService, NutritionTargetService};
use crate::testing::{
    InMemoryConsumptionRecordRepository, InMemoryHouseholdMemberRepository,
    InMemoryHouseholdSettingsRepository, InMemoryMealPlanRepository,
    InMemoryNutritionTargetRepository, InMemoryProductRepository, InMemoryRecipeRepository,
    InMemoryStockRepository,
};

struct Harness {
    service: MealPlanService,
    diary: DiaryService,
    targets: NutritionTargetService,
    products: InMemoryProductRepository,
    recipes: InMemoryRecipeRepository,
    records: InMemoryConsumptionRecordRepository,
    members: InMemoryHouseholdMemberRepository,
    settings: InMemoryHouseholdSettingsRepository,
    stock: InMemoryStockRepository,
    plans: InMemoryMealPlanRepository,
    member_id: HouseholdMemberId,
    actor_id: UserId,
}

impl Harness {
    fn seed_stock_grams(&self, product_id: ProductId, grams: i64) -> crate::domain::StockItemId {
        let item = StockItem {
            id: crate::domain::StockItemId::new(),
            product_id,
            level: StockLevel::Exact {
                quantity: Quantity::new(Decimal::new(grams, 0), Unit::Gram),
            },
            storage_location: StorageLocation::Chilled,
            source_date: None,
            usability_deadline: None,
            note: None,
            revision: Revision::INITIAL,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
            archived_at: None,
        };
        let id = item.id;
        self.stock.seed(item);
        id
    }

    async fn stock_grams(&self, id: crate::domain::StockItemId) -> Decimal {
        match self.stock.get(id).await.unwrap().unwrap().level {
            StockLevel::Exact { quantity } => quantity.amount,
            _ => panic!("expected an exact level"),
        }
    }
}

impl Harness {
    fn add_member(&self, name: &str) -> HouseholdMemberId {
        let id = HouseholdMemberId::new();
        self.members.seed(HouseholdMember {
            id,
            display_name: name.to_owned(),
            linked_user_id: None,
            revision: Revision::INITIAL,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
            archived_at: None,
        });
        id
    }
}

fn harness() -> Harness {
    let products = InMemoryProductRepository::new();
    let recipes = InMemoryRecipeRepository::new();
    let stock = InMemoryStockRepository::new();
    let records = InMemoryConsumptionRecordRepository::with_stock(stock.clone());
    let target_repo = InMemoryNutritionTargetRepository::new();
    let members = InMemoryHouseholdMemberRepository::new();
    let settings = InMemoryHouseholdSettingsRepository::new();
    let clock = Arc::new(FixedClock::new(datetime!(2026-08-24 09:00 UTC)));
    let member_id = HouseholdMemberId::new();
    members.seed(HouseholdMember {
        id: member_id,
        display_name: "Test Member".to_owned(),
        linked_user_id: None,
        revision: Revision::INITIAL,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
        archived_at: None,
    });
    let plans = InMemoryMealPlanRepository::new(records.clone());
    let service = MealPlanService::new(
        Arc::new(plans.clone()),
        Arc::new(products.clone()),
        Arc::new(recipes.clone()),
        Arc::new(records.clone()),
        Arc::new(target_repo.clone()),
        Arc::new(members.clone()),
        Arc::new(settings.clone()),
        clock.clone(),
    );
    let diary = DiaryService::new(
        Arc::new(records.clone()),
        Arc::new(products.clone()),
        Arc::new(recipes.clone()),
        clock.clone(),
    );
    let targets = NutritionTargetService::new(Arc::new(target_repo.clone()), clock);
    Harness {
        service,
        diary,
        targets,
        products,
        recipes,
        records,
        members,
        settings,
        stock,
        plans,
        member_id,
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
        id: None,
        item: MealItemRef::product(product_id),
        amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(grams, 0), Unit::Gram)),
    }
}

fn recipe(name: &str, owner_id: UserId, servings: i32, components: Vec<RecipeComponent>) -> Recipe {
    let now = OffsetDateTime::now_utc();
    Recipe {
        id: RecipeId::new(),
        name: name.to_owned(),
        description: None,
        servings,
        preparation_minutes: None,
        cooking_minutes: None,
        notes: None,
        components,
        instructions: Vec::new(),
        meal_categories: Vec::new(),
        country_categories: Vec::new(),
        tags: Vec::new(),
        photo_version: None,
        owner_id,
        visibility: RecipeVisibility::Private,
        created_by: owner_id,
        updated_by: owner_id,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    }
}

fn recipe_line(product_id: ProductId, grams: i64) -> RecipeComponent {
    RecipeComponent {
        id: crate::domain::RecipeComponentId::new(),
        requirement: crate::domain::RecipeRequirement::Product { product_id },
        source_text: None,
        amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(grams, 0), Unit::Gram)),
        position: 0,
    }
}

fn servings_of(recipe_id: RecipeId, count: i64) -> NewMealPlanComponent {
    NewMealPlanComponent {
        id: None,
        item: MealItemRef::recipe(recipe_id),
        amount: ConsumedAmount::Servings(Decimal::new(count, 0)),
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
            scope: MealPlanScope::Member,
            member_id: Some(h.member_id),
            planned_on: date!(2026 - 08 - 25),
            planned_time: Some(time!(18:30)),
            slot: MealSlot::Dinner,
            components,
            participants: None,
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn a_member_has_one_main_meal_entry_per_day_and_slot() {
    let h = harness();
    let food = product("Food", 200);
    h.products.seed(food.clone());
    planned(&h, vec![measured(food.id, 100)]).await;

    let error = h
        .service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Member,
            member_id: Some(h.member_id),
            planned_on: date!(2026 - 08 - 25),
            planned_time: Some(time!(19:00)),
            slot: MealSlot::Dinner,
            components: vec![measured(food.id, 50)],
            participants: None,
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Conflict { .. }));
}

#[tokio::test]
async fn snacks_allow_distinct_timed_occurrences_and_one_untimed_occurrence() {
    let h = harness();
    let food = product("Food", 200);
    h.products.seed(food.clone());
    let entry = h
        .service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Member,
            member_id: Some(h.member_id),
            planned_on: date!(2026 - 08 - 25),
            planned_time: Some(time!(20:30)),
            slot: MealSlot::Snacks,
            components: vec![measured(food.id, 100)],
            participants: None,
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap();
    assert_eq!(entry.entry.planned_time, Some(time!(20:30)));

    let updated = h
        .service
        .update(
            entry.entry.id,
            entry.entry.revision,
            MealPlanEntryPatch {
                planned_time: Some(Some(time!(21:00))),
                ..Default::default()
            },
            h.actor_id,
        )
        .await
        .unwrap();
    assert_eq!(updated.entry.planned_time, Some(time!(21:00)));

    let timed = h
        .service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Member,
            member_id: Some(h.member_id),
            planned_on: date!(2026 - 08 - 25),
            planned_time: Some(time!(20:30)),
            slot: MealSlot::Snacks,
            components: vec![measured(food.id, 50)],
            participants: None,
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap();
    assert_eq!(timed.entry.planned_time, Some(time!(20:30)));

    let duplicate_timed = h
        .service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Member,
            member_id: Some(h.member_id),
            planned_on: date!(2026 - 08 - 25),
            planned_time: Some(time!(20:30)),
            slot: MealSlot::Snacks,
            components: vec![measured(food.id, 25)],
            participants: None,
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap_err();
    assert!(matches!(duplicate_timed, CoreError::Conflict { .. }));

    h.service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Member,
            member_id: Some(h.member_id),
            planned_on: date!(2026 - 08 - 25),
            planned_time: None,
            slot: MealSlot::Snacks,
            components: vec![measured(food.id, 25)],
            participants: None,
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap();

    let duplicate_untimed = h
        .service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Member,
            member_id: Some(h.member_id),
            planned_on: date!(2026 - 08 - 25),
            planned_time: None,
            slot: MealSlot::Snacks,
            components: vec![measured(food.id, 25)],
            participants: None,
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap_err();
    assert!(matches!(duplicate_untimed, CoreError::Conflict { .. }));
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
            item: MealItemRef::product(food.id),
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
                consumed_at: Some(datetime!(2026-08-26 19:15 UTC)),
                components: entry
                    .components
                    .iter()
                    .map(|component| ActualMealPlanComponent {
                        component_id: component.component.id,
                        amount: component.component.amount,
                    })
                    .collect(),
                actor_id: h.actor_id,
                subject_member_id: None,
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
async fn confirming_one_component_does_not_resolve_its_siblings() {
    let h = harness();
    let oats = product("Oats", 200);
    let milk = product("Milk", 100);
    let banana = product("Banana", 80);
    h.products.seed(oats.clone());
    h.products.seed(milk.clone());
    h.products.seed(banana.clone());
    let entry = planned(
        &h,
        vec![
            measured(oats.id, 80),
            measured(milk.id, 250),
            measured(banana.id, 100),
        ],
    )
    .await;
    let banana_component = entry.components[2].component.clone();

    let updated = h
        .service
        .mark_component_eaten_unchecked(
            entry.entry.id,
            banana_component.id,
            banana_component.revision,
            ConfirmMealPlanComponent {
                consumed_on: date!(2026 - 08 - 25),
                consumed_at: Some(datetime!(2026-08-25 08:00 UTC)),
                amount: banana_component.amount,
                actor_id: h.actor_id,
                subject_member_id: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.entry.status, MealPlanStatus::PartiallyResolved);
    assert_eq!(
        updated.components[0].component.status,
        MealPlanStatus::Planned
    );
    assert_eq!(
        updated.components[1].component.status,
        MealPlanStatus::Planned
    );
    assert_eq!(
        updated.components[2].component.status,
        MealPlanStatus::Eaten
    );
    assert_eq!(h.records.count(), 1);

    let week = h
        .service
        .week(h.member_id, date!(2026 - 08 - 24))
        .await
        .unwrap();
    let dinner = week.days[1]
        .slots
        .iter()
        .find(|slot| slot.slot == MealSlot::Dinner)
        .unwrap();
    let component_ids: Vec<_> = dinner
        .items
        .iter()
        .filter_map(|item| match item.source {
            MealItemSource::Planned { component_id, .. } => Some(component_id),
            MealItemSource::Logged { .. } => None,
        })
        .collect();
    assert_eq!(
        component_ids,
        entry
            .components
            .iter()
            .map(|component| component.component.id)
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn editing_one_component_preserves_its_siblings() {
    let h = harness();
    let oats = product("Oats", 200);
    let milk = product("Milk", 100);
    let banana = product("Banana", 80);
    h.products.seed(oats.clone());
    h.products.seed(milk.clone());
    h.products.seed(banana.clone());
    let entry = planned(
        &h,
        vec![
            measured(oats.id, 80),
            measured(milk.id, 250),
            measured(banana.id, 100),
        ],
    )
    .await;
    let original_ids: Vec<_> = entry
        .components
        .iter()
        .map(|component| component.component.id)
        .collect();
    let components = entry
        .components
        .iter()
        .enumerate()
        .map(|(index, component)| NewMealPlanComponent {
            id: Some(component.component.id),
            item: component.component.item,
            amount: if index == 2 {
                ConsumedAmount::Measure(Quantity::new(Decimal::new(120, 0), Unit::Gram))
            } else {
                component.component.amount
            },
        })
        .collect();

    let updated = h
        .service
        .update(
            entry.entry.id,
            entry.entry.revision,
            MealPlanEntryPatch {
                components: Some(components),
                ..Default::default()
            },
            h.actor_id,
        )
        .await
        .unwrap();

    assert_eq!(
        updated
            .components
            .iter()
            .map(|component| component.component.id)
            .collect::<Vec<_>>(),
        original_ids
    );
    assert_eq!(updated.components[0].component.revision, Revision::INITIAL);
    assert_eq!(updated.components[1].component.revision, Revision::INITIAL);
    assert_eq!(
        updated.components[2].component.revision,
        Revision::INITIAL.next()
    );
}

#[tokio::test]
async fn later_planned_components_append_after_food_already_logged_in_the_slot() {
    let h = harness();
    let oats = product("Oats", 200);
    let milk = product("Milk", 100);
    let shake = product("Protein Shake", 120);
    let latte = product("Latte", 90);
    h.products.seed(oats.clone());
    h.products.seed(milk.clone());
    h.products.seed(shake.clone());
    h.products.seed(latte.clone());
    let entry = planned(&h, vec![measured(oats.id, 80), measured(milk.id, 250)]).await;

    h.diary
        .record(NewConsumptionRecord {
            id: None,
            member_id: h.member_id,
            item: MealItemRef::product(shake.id),
            recorded_by: Some(h.actor_id),
            meal_plan_entry_id: None,
            meal_plan_component_id: None,
            slot: MealSlot::Dinner,
            amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(300, 0), Unit::Gram)),
            consumed_on: date!(2026 - 08 - 25),
            consumed_at: Some(datetime!(2026-08-25 18:35 UTC)),
        })
        .await
        .unwrap();

    let mut components: Vec<_> = entry
        .components
        .iter()
        .map(|component| NewMealPlanComponent {
            id: Some(component.component.id),
            item: component.component.item,
            amount: component.component.amount,
        })
        .collect();
    components.push(measured(latte.id, 250));
    h.service
        .update(
            entry.entry.id,
            entry.entry.revision,
            MealPlanEntryPatch {
                components: Some(components),
                ..Default::default()
            },
            h.actor_id,
        )
        .await
        .unwrap();

    let week = h
        .service
        .week(h.member_id, date!(2026 - 08 - 24))
        .await
        .unwrap();
    let dinner = week.days[1]
        .slots
        .iter()
        .find(|slot| slot.slot == MealSlot::Dinner)
        .unwrap();
    assert_eq!(
        dinner
            .items
            .iter()
            .map(|item| item.item_name.as_str())
            .collect::<Vec<_>>(),
        vec!["Oats", "Milk", "Protein Shake", "Latte"]
    );
}

#[tokio::test]
async fn marking_remaining_eaten_skips_an_item_marked_not_eaten() {
    let h = harness();
    let first = product("First", 200);
    let second = product("Second", 100);
    let third = product("Third", 80);
    h.products.seed(first.clone());
    h.products.seed(second.clone());
    h.products.seed(third.clone());
    let entry = planned(
        &h,
        vec![
            measured(first.id, 80),
            measured(second.id, 250),
            measured(third.id, 100),
        ],
    )
    .await;
    let rejected = h
        .service
        .mark_component_not_eaten_unchecked(
            entry.entry.id,
            entry.components[0].component.id,
            entry.components[0].component.revision,
            OutcomeActor::own(h.actor_id),
        )
        .await
        .unwrap();
    let pending: Vec<_> = rejected
        .components
        .iter()
        .filter(|component| component.component.status == MealPlanStatus::Planned)
        .map(|component| ActualMealPlanComponent {
            component_id: component.component.id,
            amount: component.component.amount,
        })
        .collect();

    let resolved = h
        .service
        .mark_eaten_unchecked(
            rejected.entry.id,
            rejected.entry.revision,
            ConfirmMealPlanEntry {
                consumed_on: date!(2026 - 08 - 25),
                consumed_at: Some(datetime!(2026-08-25 08:00 UTC)),
                components: pending,
                actor_id: h.actor_id,
                subject_member_id: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(resolved.entry.status, MealPlanStatus::PartiallyResolved);
    assert_eq!(
        resolved.components[0].component.status,
        MealPlanStatus::NotEaten
    );
    assert!(
        resolved.components[1..]
            .iter()
            .all(|component| component.component.status == MealPlanStatus::Eaten)
    );
    assert_eq!(h.records.count(), 2);
}

#[tokio::test]
async fn a_resolved_entry_keeps_its_product_snapshot() {
    let h = harness();
    let original = product("Original name", 200);
    h.products.seed(original.clone());
    let entry = planned(&h, vec![measured(original.id, 100)]).await;
    let resolved = h
        .service
        .mark_not_eaten(
            entry.entry.id,
            entry.entry.revision,
            OutcomeActor::own(h.actor_id),
        )
        .await
        .unwrap();

    let mut changed = original;
    changed.name = "Changed name".to_owned();
    changed.nutrition.energy_kcal = Some(Decimal::new(900, 0));
    h.products.seed(changed);

    let loaded = h.service.get(resolved.entry.id).await.unwrap();
    assert_eq!(loaded.components[0].item_name, "Original name");
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
        .mark_not_eaten(
            entry.entry.id,
            entry.entry.revision,
            OutcomeActor::own(h.actor_id),
        )
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
                consumed_at: Some(datetime!(2026-08-25 18:30 UTC)),
                components: vec![ActualMealPlanComponent {
                    component_id: entry.components[0].component.id,
                    amount: entry.components[0].component.amount,
                }],
                actor_id: h.actor_id,
                subject_member_id: None,
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
            scope: MealPlanScope::Member,
            member_id: Some(h.member_id),
            planned_on: date!(2026 - 08 - 26),
            planned_time: None,
            slot: MealSlot::Lunch,
            components: vec![measured(food.id, 100)],
            participants: None,
            guest_groups: Vec::new(),
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
                consumed_at: Some(datetime!(2026-08-25 18:30 UTC)),
                components: vec![ActualMealPlanComponent {
                    component_id: entry.components[0].component.id,
                    amount: entry.components[0].component.amount,
                }],
                actor_id: h.actor_id,
                subject_member_id: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(h.records.count(), 1);

    let reopened = h
        .service
        .reopen(
            confirmed.entry.id,
            confirmed.entry.revision,
            OutcomeActor::own(h.actor_id),
        )
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
        .mark_not_eaten(
            entry.entry.id,
            entry.entry.revision,
            OutcomeActor::own(h.actor_id),
        )
        .await
        .unwrap();

    let reopened = h
        .service
        .reopen(
            resolved.entry.id,
            resolved.entry.revision,
            OutcomeActor::own(h.actor_id),
        )
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
        .reopen(
            entry.entry.id,
            entry.entry.revision,
            OutcomeActor::own(h.actor_id),
        )
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
        .mark_not_eaten(
            entry.entry.id,
            entry.entry.revision,
            OutcomeActor::own(h.actor_id),
        )
        .await
        .unwrap();

    let error = h
        .service
        .reopen(
            resolved.entry.id,
            entry.entry.revision,
            OutcomeActor::own(h.actor_id),
        )
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
                consumed_at: Some(datetime!(2026-08-25 18:30 UTC)),
                components: vec![ActualMealPlanComponent {
                    component_id: entry.components[0].component.id,
                    amount: entry.components[0].component.amount,
                }],
                actor_id: h.actor_id,
                subject_member_id: None,
            },
        )
        .await
        .unwrap();
    let reopened = h
        .service
        .reopen(
            confirmed.entry.id,
            confirmed.entry.revision,
            OutcomeActor::own(h.actor_id),
        )
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
                consumed_at: Some(datetime!(2026-08-26 18:30 UTC)),
                components: vec![ActualMealPlanComponent {
                    component_id: edited.components[0].component.id,
                    amount: edited.components[0].component.amount,
                }],
                actor_id: h.actor_id,
                subject_member_id: None,
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
            scope: MealPlanScope::Member,
            member_id: Some(h.member_id),
            planned_on: date!(2026 - 08 - 20),
            planned_time: None,
            slot: MealSlot::Dinner,
            components: vec![measured(food.id, 100)],
            participants: None,
            guest_groups: Vec::new(),
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
            scope: MealPlanScope::Member,
            member_id: Some(h.member_id),
            planned_on: date!(2026 - 08 - 23),
            planned_time: None,
            slot: MealSlot::Dinner,
            components: vec![measured(food.id, 100)],
            participants: None,
            guest_groups: Vec::new(),
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
            scope: MealPlanScope::Member,
            member_id: Some(h.member_id),
            planned_on: date!(2026 - 08 - 30),
            planned_time: None,
            slot: MealSlot::Dinner,
            components: vec![measured(food.id, 100)],
            participants: None,
            guest_groups: Vec::new(),
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
                consumed_at: Some(datetime!(2026-08-30 18:30 UTC)),
                components: entry
                    .components
                    .iter()
                    .map(|component| ActualMealPlanComponent {
                        component_id: component.component.id,
                        amount: component.component.amount,
                    })
                    .collect(),
                actor_id: h.actor_id,
                subject_member_id: None,
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
    assert_eq!(dinner.items[0].item_name, "Food");

    h.service
        .mark_eaten(
            entry.entry.id,
            entry.entry.revision,
            ConfirmMealPlanEntry {
                consumed_on: date!(2026 - 08 - 26),
                consumed_at: Some(datetime!(2026-08-26 19:00 UTC)),
                components: vec![ActualMealPlanComponent {
                    component_id: entry.components[0].component.id,
                    amount: ConsumedAmount::Measure(Quantity::new(
                        Decimal::new(150, 0),
                        Unit::Gram,
                    )),
                }],
                actor_id: h.actor_id,
                subject_member_id: None,
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
            item: MealItemRef::product(food.id),
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
    assert_eq!(snacks.items[0].item_name, "Food");
    assert!(matches!(
        snacks.items[0].source,
        MealItemSource::Logged { .. }
    ));
}

async fn seed_recipe(
    h: &Harness,
    name: &str,
    servings: i32,
    lines: Vec<RecipeComponent>,
) -> Recipe {
    let recipe = recipe(name, h.actor_id, servings, lines);
    h.recipes.seed(recipe.clone());
    recipe
}

#[tokio::test]
async fn a_recipe_serving_can_be_planned_and_counts_toward_the_day() {
    let h = harness();
    let rice = product("Rice", 100);
    h.products.seed(rice.clone());
    let curry = seed_recipe(&h, "Curry", 5, vec![recipe_line(rice.id, 500)]).await;

    let entry = planned(&h, vec![servings_of(curry.id, 1)]).await;

    assert_eq!(entry.components.len(), 1);
    assert_eq!(entry.components[0].item_name, "Curry");
    assert_eq!(
        entry.components[0].component.item,
        MealItemRef::recipe(curry.id)
    );
    assert_eq!(
        entry.planned.nutrition.energy_kcal,
        Some(Decimal::new(100, 0))
    );

    let week = h
        .service
        .week(h.member_id, date!(2026 - 08 - 25))
        .await
        .unwrap();
    let day = week
        .days
        .iter()
        .find(|day| day.date == date!(2026 - 08 - 25))
        .unwrap();
    let dinner = day
        .slots
        .iter()
        .find(|slot| slot.slot == MealSlot::Dinner)
        .unwrap();
    assert_eq!(dinner.items[0].item_name, "Curry");
    assert_eq!(
        dinner.nutrition.nutrition.energy_kcal,
        Some(Decimal::new(100, 0))
    );
}

#[tokio::test]
async fn two_servings_double_the_planned_nutrition() {
    let h = harness();
    let rice = product("Rice", 100);
    h.products.seed(rice.clone());
    let curry = seed_recipe(&h, "Curry", 5, vec![recipe_line(rice.id, 500)]).await;

    let entry = planned(&h, vec![servings_of(curry.id, 2)]).await;

    assert_eq!(
        entry.planned.nutrition.energy_kcal,
        Some(Decimal::new(200, 0))
    );
}

#[tokio::test]
async fn planning_a_recipe_you_do_not_own_is_refused() {
    let h = harness();
    let rice = product("Rice", 100);
    h.products.seed(rice.clone());
    let someone_else = UserId::new();
    let curry = recipe("Curry", someone_else, 4, vec![recipe_line(rice.id, 400)]);
    h.recipes.seed(curry.clone());

    let error = h
        .service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Member,
            member_id: Some(h.member_id),
            planned_on: date!(2026 - 08 - 25),
            planned_time: Some(time!(18:30)),
            slot: MealSlot::Dinner,
            components: vec![servings_of(curry.id, 1)],
            participants: None,
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::NotFound { .. }));
}

#[tokio::test]
async fn a_recipe_component_rejects_a_measured_amount() {
    let h = harness();
    let rice = product("Rice", 100);
    h.products.seed(rice.clone());
    let curry = seed_recipe(&h, "Curry", 4, vec![recipe_line(rice.id, 400)]).await;

    let error = h
        .service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Member,
            member_id: Some(h.member_id),
            planned_on: date!(2026 - 08 - 25),
            planned_time: Some(time!(18:30)),
            slot: MealSlot::Dinner,
            components: vec![NewMealPlanComponent {
                id: None,
                item: MealItemRef::recipe(curry.id),
                amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(200, 0), Unit::Gram)),
            }],
            participants: None,
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Validation { .. }));
}

#[tokio::test]
async fn editing_a_recipe_moves_planned_numbers_but_not_eaten_history() {
    let h = harness();
    let rice = product("Rice", 100);
    h.products.seed(rice.clone());
    let mut curry = seed_recipe(&h, "Curry", 5, vec![recipe_line(rice.id, 500)]).await;

    let entry = planned(&h, vec![servings_of(curry.id, 1)]).await;
    assert_eq!(
        entry.planned.nutrition.energy_kcal,
        Some(Decimal::new(100, 0))
    );

    curry.components = vec![recipe_line(rice.id, 1000)];
    h.recipes.seed(curry.clone());

    let reloaded = h.service.get(entry.entry.id).await.unwrap();
    assert_eq!(
        reloaded.planned.nutrition.energy_kcal,
        Some(Decimal::new(200, 0))
    );

    let component_id = reloaded.components[0].component.id;
    let eaten = h
        .service
        .mark_component_eaten_unchecked(
            entry.entry.id,
            component_id,
            reloaded.components[0].component.revision,
            ConfirmMealPlanComponent {
                consumed_on: date!(2026 - 08 - 25),
                consumed_at: None,
                amount: ConsumedAmount::Servings(Decimal::ONE),
                actor_id: h.actor_id,
                subject_member_id: None,
            },
        )
        .await
        .unwrap();
    let eaten_kcal = eaten.components[0]
        .consumption_record
        .as_ref()
        .unwrap()
        .nutrition
        .energy_kcal;
    assert_eq!(eaten_kcal, Some(Decimal::new(200, 0)));

    curry.components = vec![recipe_line(rice.id, 250)];
    h.recipes.seed(curry.clone());

    let after = h.service.get(entry.entry.id).await.unwrap();
    assert_eq!(
        after.components[0]
            .consumption_record
            .as_ref()
            .unwrap()
            .nutrition
            .energy_kcal,
        Some(Decimal::new(200, 0))
    );
    assert_eq!(
        after.components[0].nutrition.energy_kcal,
        Some(Decimal::new(200, 0))
    );
}

#[tokio::test]
async fn confirming_a_recipe_component_writes_a_recipe_referencing_record() {
    let h = harness();
    let rice = product("Rice", 100);
    h.products.seed(rice.clone());
    let curry = seed_recipe(&h, "Curry", 4, vec![recipe_line(rice.id, 400)]).await;
    let entry = planned(&h, vec![servings_of(curry.id, 1)]).await;
    let component_id = entry.components[0].component.id;

    h.service
        .mark_component_eaten_unchecked(
            entry.entry.id,
            component_id,
            entry.components[0].component.revision,
            ConfirmMealPlanComponent {
                consumed_on: date!(2026 - 08 - 25),
                consumed_at: None,
                amount: ConsumedAmount::Servings(Decimal::ONE),
                actor_id: h.actor_id,
                subject_member_id: None,
            },
        )
        .await
        .unwrap();

    let logged = h
        .records
        .list_for_meal_plan_entry(entry.entry.id)
        .await
        .unwrap();
    assert_eq!(logged.len(), 1);
    assert_eq!(logged[0].item, MealItemRef::recipe(curry.id));
}

#[tokio::test]
async fn a_household_meal_defaults_to_every_member_when_enabled() {
    let h = harness();
    h.settings.set_default_all_members_participate(true);
    let morgan = h.add_member("Morgan");
    let taylor = h.add_member("Taylor");
    let food = product("Stew", 200);
    h.products.seed(food.clone());

    let entry = h
        .service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Household,
            member_id: None,
            planned_on: date!(2026 - 08 - 25),
            planned_time: Some(time!(18:30)),
            slot: MealSlot::Dinner,
            components: vec![measured(food.id, 600)],
            participants: None,
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap();

    let member_ids: std::collections::HashSet<_> = entry
        .participants
        .iter()
        .map(|participant| participant.member_id)
        .collect();
    assert_eq!(member_ids.len(), 3);
    assert!(member_ids.contains(&h.member_id));
    assert!(member_ids.contains(&morgan));
    assert!(member_ids.contains(&taylor));
}

#[tokio::test]
async fn a_personal_meal_never_auto_adds_members() {
    let h = harness();
    h.settings.set_default_all_members_participate(true);
    h.add_member("Morgan");
    let food = product("Toast", 120);
    h.products.seed(food.clone());

    let entry = planned(&h, vec![measured(food.id, 60)]).await;

    assert_eq!(entry.participants.len(), 1);
    assert_eq!(entry.participants[0].member_id, h.member_id);
}

#[tokio::test]
async fn a_participant_sees_only_their_own_share_and_outcome() {
    let h = harness();
    let taylor = h.add_member("Taylor");
    let food = product("Curry", 100);
    h.products.seed(food.clone());
    let entry = planned(&h, vec![measured(food.id, 400)]).await;

    let with_taylor = h
        .service
        .set_participants(
            entry.entry.id,
            entry.entry.revision,
            crate::domain::SetMealParticipants {
                actor_id: h.actor_id,
                guest_groups: Vec::new(),
                participants: vec![
                    crate::domain::NewMealParticipant {
                        id: None,
                        member_id: h.member_id,
                        allocations: vec![crate::domain::NewMealParticipantAllocation {
                            component_id: entry.components[0].component.id,
                            allocated: ConsumedAmount::Measure(Quantity::new(
                                Decimal::new(300, 0),
                                Unit::Gram,
                            )),
                        }],
                    },
                    crate::domain::NewMealParticipant {
                        id: None,
                        member_id: taylor,
                        allocations: vec![crate::domain::NewMealParticipantAllocation {
                            component_id: entry.components[0].component.id,
                            allocated: ConsumedAmount::Measure(Quantity::new(
                                Decimal::new(100, 0),
                                Unit::Gram,
                            )),
                        }],
                    },
                ],
            },
        )
        .await
        .unwrap();

    let prep = &with_taylor.components[0].preparation;
    assert_eq!(
        prep.leftover,
        Some(ConsumedAmount::Measure(Quantity::new(
            Decimal::ZERO,
            Unit::Gram
        )))
    );

    let component = with_taylor.components[0].component.clone();
    h.service
        .mark_component_eaten_unchecked(
            with_taylor.entry.id,
            component.id,
            component.revision,
            ConfirmMealPlanComponent {
                consumed_on: date!(2026 - 08 - 25),
                consumed_at: None,
                amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(280, 0), Unit::Gram)),
                actor_id: h.actor_id,
                subject_member_id: Some(taylor),
            },
        )
        .await
        .unwrap();

    let after = h.service.get(with_taylor.entry.id).await.unwrap();
    assert_eq!(after.entry.status, MealPlanStatus::PartiallyResolved);
    let taylor_participant = after
        .participants
        .iter()
        .find(|participant| participant.member_id == taylor)
        .unwrap();
    assert_eq!(taylor_participant.status, MealPlanStatus::Eaten);
    let owner_participant = after
        .participants
        .iter()
        .find(|participant| participant.member_id == h.member_id)
        .unwrap();
    assert_eq!(owner_participant.status, MealPlanStatus::Planned);
}

#[tokio::test]
async fn a_recipe_built_from_products_without_nutrition_reports_unknown_not_zero() {
    let h = harness();
    let mut blank = product("Mystery", 0);
    blank.nutrition = NutritionFacts::default();
    h.products.seed(blank.clone());
    let curry = seed_recipe(&h, "Curry", 4, vec![recipe_line(blank.id, 400)]).await;

    let entry = planned(&h, vec![servings_of(curry.id, 1)]).await;

    assert_eq!(entry.components[0].quality, NutritionQuality::Unknown);
    assert_eq!(entry.components[0].nutrition.energy_kcal, None);
}

fn dgrams(value: i64) -> Decimal {
    Decimal::new(value, 0)
}

async fn confirm_component(
    h: &Harness,
    entry_id: crate::domain::MealPlanEntryId,
    component_id: crate::domain::MealPlanComponentId,
    revision: Revision,
    amount: ConsumedAmount,
) -> StockAffected<MealPlanEntryView> {
    h.service
        .mark_component_eaten_unchecked(
            entry_id,
            component_id,
            revision,
            ConfirmMealPlanComponent {
                consumed_on: date!(2026 - 08 - 25),
                consumed_at: None,
                amount,
                actor_id: h.actor_id,
                subject_member_id: None,
            },
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn confirming_a_planned_component_draws_its_prepared_amount_from_stock() {
    let h = harness();
    let chicken = product("Chicken", 120);
    h.products.seed(chicken.clone());
    let item = h.seed_stock_grams(chicken.id, 500);

    let entry = planned(&h, vec![measured(chicken.id, 300)]).await;
    let component = entry.components[0].component.clone();

    let outcome = confirm_component(
        &h,
        entry.entry.id,
        component.id,
        component.revision,
        ConsumedAmount::Measure(Quantity::new(dgrams(300), Unit::Gram)),
    )
    .await;

    assert!(outcome.stock.is_empty(), "a covered draw raises no warning");
    assert_eq!(h.stock_grams(item).await, dgrams(200));
}

#[tokio::test]
async fn a_second_participant_confirming_does_not_draw_stock_again() {
    let h = harness();
    h.settings.set_default_all_members_participate(true);
    let other = h.add_member("Other");
    let chicken = product("Chicken", 120);
    h.products.seed(chicken.clone());
    let item = h.seed_stock_grams(chicken.id, 500);

    let created = h
        .service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Household,
            member_id: None,
            planned_on: date!(2026 - 08 - 25),
            planned_time: Some(time!(18:30)),
            slot: MealSlot::Dinner,
            components: vec![measured(chicken.id, 300)],
            participants: None,
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap();
    let component = created.components[0].component.clone();

    let confirm = |subject, revision| {
        h.service.mark_component_eaten_unchecked(
            created.entry.id,
            component.id,
            revision,
            ConfirmMealPlanComponent {
                consumed_on: date!(2026 - 08 - 25),
                consumed_at: None,
                amount: ConsumedAmount::Measure(Quantity::new(dgrams(150), Unit::Gram)),
                actor_id: h.actor_id,
                subject_member_id: Some(subject),
            },
        )
    };

    let first = confirm(h.member_id, component.revision).await.unwrap();
    assert_eq!(h.stock_grams(item).await, dgrams(200));

    let next_revision = first.components[0].component.revision;
    confirm(other, next_revision).await.unwrap();

    assert_eq!(
        h.stock_grams(item).await,
        dgrams(200),
        "the physical draw already happened at first confirmation"
    );
}

#[tokio::test]
async fn marking_a_component_not_eaten_draws_no_stock() {
    let h = harness();
    let chicken = product("Chicken", 120);
    h.products.seed(chicken.clone());
    let item = h.seed_stock_grams(chicken.id, 500);

    let entry = planned(&h, vec![measured(chicken.id, 300)]).await;
    let component = entry.components[0].component.clone();

    h.service
        .mark_component_not_eaten_unchecked(
            entry.entry.id,
            component.id,
            component.revision,
            OutcomeActor::own(h.actor_id),
        )
        .await
        .unwrap();

    assert_eq!(h.stock_grams(item).await, dgrams(500));
}

#[tokio::test]
async fn reopening_the_last_eater_returns_the_exact_amount_taken() {
    let h = harness();
    let chicken = product("Chicken", 120);
    h.products.seed(chicken.clone());
    let item = h.seed_stock_grams(chicken.id, 500);

    let entry = planned(&h, vec![measured(chicken.id, 300)]).await;
    let component = entry.components[0].component.clone();

    let after = confirm_component(
        &h,
        entry.entry.id,
        component.id,
        component.revision,
        ConsumedAmount::Measure(Quantity::new(dgrams(300), Unit::Gram)),
    )
    .await;
    assert_eq!(h.stock_grams(item).await, dgrams(200));

    let component = after.components[0].component.clone();
    h.service
        .reopen_component(
            entry.entry.id,
            component.id,
            component.revision,
            OutcomeActor::own(h.actor_id),
        )
        .await
        .unwrap();

    assert_eq!(h.stock_grams(item).await, dgrams(500));
}

#[tokio::test]
async fn a_short_confirmation_floors_stock_at_zero_and_warns() {
    let h = harness();
    let chicken = product("Chicken", 120);
    h.products.seed(chicken.clone());
    let item = h.seed_stock_grams(chicken.id, 150);

    let entry = planned(&h, vec![measured(chicken.id, 400)]).await;
    let component = entry.components[0].component.clone();

    let outcome = confirm_component(
        &h,
        entry.entry.id,
        component.id,
        component.revision,
        ConsumedAmount::Measure(Quantity::new(dgrams(400), Unit::Gram)),
    )
    .await;

    assert_eq!(h.stock_grams(item).await, dgrams(0));
    assert_eq!(outcome.stock.len(), 1);
    let warning = &outcome.stock[0];
    assert_eq!(warning.product_name, "Chicken");
    assert!(matches!(
        warning.shortfall,
        crate::domain::Shortfall::Short { .. }
    ));
}

#[tokio::test]
async fn a_recipe_component_draws_no_stock_and_raises_no_warning() {
    let h = harness();
    let rice = product("Rice", 100);
    h.products.seed(rice.clone());
    let curry = seed_recipe(&h, "Curry", 4, vec![recipe_line(rice.id, 400)]).await;
    let item = h.seed_stock_grams(rice.id, 500);

    let entry = planned(&h, vec![servings_of(curry.id, 1)]).await;
    let component = entry.components[0].component.clone();

    let outcome = confirm_component(
        &h,
        entry.entry.id,
        component.id,
        component.revision,
        ConsumedAmount::Servings(Decimal::ONE),
    )
    .await;

    assert!(outcome.stock.is_empty());
    assert_eq!(h.stock_grams(item).await, dgrams(500));
}

#[tokio::test]
async fn a_confirmed_component_stops_counting_as_planned_stock_demand() {
    let h = harness();
    let chicken = product("Chicken", 120);
    h.products.seed(chicken.clone());
    h.seed_stock_grams(chicken.id, 500);

    let entry = planned(&h, vec![measured(chicken.id, 300)]).await;
    let component = entry.components[0].component.clone();

    let stock_service = crate::services::StockService::new(
        Arc::new(h.stock.clone()),
        Arc::new(h.products.clone()),
        Arc::new(h.plans.clone()),
        Arc::new(h.members.clone()),
        Arc::new(h.settings.clone()),
        Arc::new(FixedClock::new(datetime!(2026-08-24 09:00 UTC))),
    );

    let before = stock_service
        .availability(&[chicken.id], date!(2026 - 08 - 25), date!(2026 - 08 - 25))
        .await
        .unwrap();
    let crate::domain::Availability::Quantified { unallocated, .. } = before[0].availability else {
        panic!("expected a quantified availability");
    };
    assert_eq!(unallocated.amount, dgrams(200));

    confirm_component(
        &h,
        entry.entry.id,
        component.id,
        component.revision,
        ConsumedAmount::Measure(Quantity::new(dgrams(300), Unit::Gram)),
    )
    .await;

    let after = stock_service
        .availability(&[chicken.id], date!(2026 - 08 - 25), date!(2026 - 08 - 25))
        .await
        .unwrap();
    let crate::domain::Availability::Quantified { unallocated, .. } = after[0].availability else {
        panic!("expected a quantified availability");
    };
    assert_eq!(
        unallocated.amount,
        dgrams(200),
        "the meal is no longer future demand and the 200 g left is really free"
    );
}
