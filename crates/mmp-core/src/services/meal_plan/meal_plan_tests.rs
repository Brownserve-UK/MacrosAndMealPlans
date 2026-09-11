use std::sync::Arc;

use rust_decimal::Decimal;
use time::OffsetDateTime;
use time::macros::{date, datetime, time};

use super::*;
use crate::domain::{
    ActualMealPlanComponent, ChangedMealOutcome, ConfirmMealPlanComponent, ConfirmMealPlanEntry,
    ConsumedAmount, HouseholdMember, HouseholdMemberId, MealItemRef, MealPlanEntryPatch,
    MealPlanScope, MealPlanStatus, MealSlot, NewConsumptionRecord, NewMealParticipant,
    NewMealPlanComponent, NewMealPlanEntry, NewNutritionTarget, NutritionFacts, NutritionGoals,
    NutritionQuality, OutcomeActor, ParticipantStatus, Product, ProductId, Provenance, Quantity,
    Recipe, RecipeComponent, RecipeId, RecipeVisibility, ReplacementItem, ReviewMealOutcomes,
    ReviewedMealOutcome, ReviewedMemberOutcome, Revision, StockItem, StockLevel, StockSubject,
    StorageLocation, Unit, UserId, WeightDisplay,
};
use crate::ports::{FixedClock, StockRepository};
use crate::services::PreparationService;
use crate::services::stock_effects::StockAffected;
use crate::services::{ConsumptionService, NutritionTargetService};
use crate::testing::{
    InMemoryConsumptionRecordRepository, InMemoryHouseholdMemberRepository,
    InMemoryHouseholdSettingsRepository, InMemoryIngredientRepository, InMemoryMealPlanRepository,
    InMemoryNutritionTargetRepository, InMemoryPreparedBatchRepository,
    InMemoryPreparedMealRepository, InMemoryProductRepository, InMemoryRecipeRepository,
    InMemoryStockRepository,
};

struct Harness {
    service: MealPlanService,
    consumption: ConsumptionService,
    targets: NutritionTargetService,
    products: InMemoryProductRepository,
    ingredients: InMemoryIngredientRepository,
    recipes: InMemoryRecipeRepository,
    records: InMemoryConsumptionRecordRepository,
    members: InMemoryHouseholdMemberRepository,
    settings: InMemoryHouseholdSettingsRepository,
    stock: InMemoryStockRepository,
    plans: InMemoryMealPlanRepository,
    batches: InMemoryPreparedBatchRepository,
    preparation: PreparationService,
    member_id: HouseholdMemberId,
    actor_id: UserId,
}

impl Harness {
    fn seed_stock_grams(&self, product_id: ProductId, grams: i64) -> crate::domain::StockItemId {
        let item = StockItem {
            id: crate::domain::StockItemId::new(),
            subject: StockSubject::product(product_id),
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

    async fn stock_servings(&self, id: crate::domain::StockItemId) -> Decimal {
        self.stock_grams(id).await
    }

    async fn portion_for(
        &self,
        component_id: crate::domain::MealPlanComponentId,
    ) -> crate::domain::StockItemId {
        use crate::ports::PreparedBatchRepository;
        let batch = self
            .batches
            .for_component(component_id)
            .await
            .unwrap()
            .expect("the component should have been prepared");
        self.stock
            .list(&crate::ports::StockQuery::default())
            .await
            .unwrap()
            .items
            .into_iter()
            .find(|item| item.prepared_batch_id() == Some(batch.id))
            .expect("the batch should hold a portion")
            .id
    }

    async fn portion_for_batch(
        &self,
        batch_id: crate::domain::PreparedBatchId,
    ) -> crate::domain::StockItemId {
        self.stock
            .list(&crate::ports::StockQuery::default())
            .await
            .unwrap()
            .items
            .into_iter()
            .find(|item| item.prepared_batch_id() == Some(batch_id))
            .expect("the batch should hold a portion")
            .id
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
            weight_display: WeightDisplay::default(),
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
    let ingredients = InMemoryIngredientRepository::new();
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
        weight_display: WeightDisplay::default(),
        revision: Revision::INITIAL,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
        archived_at: None,
    });
    let plans = InMemoryMealPlanRepository::new(records.clone());
    let batches = InMemoryPreparedBatchRepository::with_stock(stock.clone());
    let prepared_meals = InMemoryPreparedMealRepository::new();
    let preparation = PreparationService::new(
        Arc::new(batches.clone()),
        Arc::new(recipes.clone()),
        Arc::new(products.clone()),
        Arc::new(ingredients.clone()),
        Arc::new(prepared_meals.clone()),
        clock.clone(),
    );
    let preparation_for_tests = preparation.clone();
    let stock_service = crate::services::StockService::new(
        Arc::new(stock.clone()),
        Arc::new(products.clone()),
        Arc::new(ingredients.clone()),
        Arc::new(prepared_meals.clone()),
        Arc::new(plans.clone()),
        Arc::new(recipes.clone()),
        Arc::new(batches.clone()),
        Arc::new(members.clone()),
        Arc::new(settings.clone()),
        clock.clone(),
    );
    let service = MealPlanService::new(
        Arc::new(plans.clone()),
        Arc::new(products.clone()),
        Arc::new(ingredients.clone()),
        Arc::new(prepared_meals.clone()),
        Arc::new(recipes.clone()),
        Arc::new(records.clone()),
        Arc::new(target_repo.clone()),
        Arc::new(members.clone()),
        Arc::new(settings.clone()),
        Arc::new(batches.clone()),
        preparation,
        stock_service,
        clock.clone(),
    );
    let consumption = ConsumptionService::new(
        Arc::new(records.clone()),
        Arc::new(products.clone()),
        Arc::new(ingredients.clone()),
        Arc::new(prepared_meals.clone()),
        Arc::new(recipes.clone()),
        Arc::new(batches.clone()),
        clock.clone(),
    );
    let targets = NutritionTargetService::new(Arc::new(target_repo.clone()), clock);
    Harness {
        service,
        consumption,
        targets,
        products,
        ingredients,
        recipes,
        records,
        members,
        settings,
        stock,
        plans,
        batches,
        preparation: preparation_for_tests,
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
        track_stock: None,
        package_quantity: Some(Quantity::new(Decimal::new(500, 0), Unit::Gram)),
        servings_per_pack: Some(5),
        mapped_ingredient_id: None,
        mapped_prepared_meal_id: None,
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
    h.consumption
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
async fn confirming_eaten_creates_one_linked_consumption_record_per_component() {
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

    assert_eq!(confirmed.status, MealPlanStatus::Eaten);
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
        .mark_component_eaten_backdated(
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

    assert_eq!(updated.status, MealPlanStatus::PartiallyResolved);
    assert_eq!(updated.components[0].status, MealPlanStatus::Planned);
    assert_eq!(updated.components[1].status, MealPlanStatus::Planned);
    assert_eq!(updated.components[2].status, MealPlanStatus::Eaten);
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

    h.consumption
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
        .mark_component_not_eaten_backdated(
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
        .filter(|component| component.status == MealPlanStatus::Planned)
        .map(|component| ActualMealPlanComponent {
            component_id: component.component.id,
            amount: component.component.amount,
        })
        .collect();

    let resolved = h
        .service
        .mark_eaten_backdated(
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

    assert_eq!(resolved.status, MealPlanStatus::PartiallyResolved);
    assert_eq!(resolved.components[0].status, MealPlanStatus::NotEaten);
    assert!(
        resolved.components[1..]
            .iter()
            .all(|component| component.status == MealPlanStatus::Eaten)
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
async fn linked_consumption_records_can_be_amended_but_not_deleted() {
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
        .consumption
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
        .consumption
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
        .consumption
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
async fn reopening_an_eaten_entry_removes_its_consumption_records() {
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

    assert_eq!(reopened.status, MealPlanStatus::Planned);
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

    assert_eq!(reopened.status, MealPlanStatus::Planned);
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

    assert_eq!(reconfirmed.status, MealPlanStatus::Eaten);
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
    h.consumption
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
async fn planned_nutrition_follows_the_serving_you_eat_not_the_batch_you_make() {
    let h = harness();
    let rice = product("Rice", 100);
    h.products.seed(rice.clone());
    let curry = seed_recipe(&h, "Curry", 5, vec![recipe_line(rice.id, 500)]).await;

    let entry = planned(&h, vec![servings_of(curry.id, 2)]).await;

    assert_eq!(
        entry.planned.nutrition.energy_kcal,
        Some(Decimal::new(100, 0))
    );
    assert_eq!(
        entry.components[0].preparation.prepared,
        ConsumedAmount::Servings(Decimal::new(2, 0))
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
    cook(&h, entry.entry.id, component_id, curry.id, 1).await;
    let eaten = h
        .service
        .mark_component_eaten_backdated(
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
    cook(&h, entry.entry.id, component_id, curry.id, 1).await;

    h.service
        .mark_component_eaten_backdated(
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
        .mark_component_eaten_backdated(
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
    assert_eq!(after.status, MealPlanStatus::PartiallyResolved);
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
async fn recording_one_outcome_leaves_everyone_elses_share_alone() {
    let h = harness();
    let taylor = h.add_member("Taylor");
    let food = product("Curry", 100);
    h.products.seed(food.clone());
    let entry = planned(&h, vec![measured(food.id, 400)]).await;
    let component_id = entry.components[0].component.id;

    let shared = h
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
                            component_id,
                            allocated: ConsumedAmount::Measure(Quantity::new(
                                dgrams(300),
                                Unit::Gram,
                            )),
                        }],
                    },
                    crate::domain::NewMealParticipant {
                        id: None,
                        member_id: taylor,
                        allocations: vec![crate::domain::NewMealParticipantAllocation {
                            component_id,
                            allocated: ConsumedAmount::Measure(Quantity::new(
                                dgrams(100),
                                Unit::Gram,
                            )),
                        }],
                    },
                ],
            },
        )
        .await
        .unwrap();

    let component = shared.components[0].component.clone();
    h.service
        .mark_component_eaten_backdated(
            shared.entry.id,
            component.id,
            component.revision,
            ConfirmMealPlanComponent {
                consumed_on: date!(2026 - 08 - 25),
                consumed_at: None,
                amount: ConsumedAmount::Measure(Quantity::new(dgrams(300), Unit::Gram)),
                actor_id: h.actor_id,
                subject_member_id: Some(h.member_id),
            },
        )
        .await
        .unwrap();

    let after = h.service.get(shared.entry.id).await.unwrap();
    let taylor_share = after
        .participants
        .iter()
        .find(|participant| participant.member_id == taylor)
        .unwrap()
        .allocations
        .iter()
        .find(|allocation| allocation.component_id == component_id)
        .unwrap();
    assert_eq!(
        taylor_share.allocated,
        ConsumedAmount::Measure(Quantity::new(dgrams(100), Unit::Gram))
    );
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

async fn household_planned(
    h: &Harness,
    components: Vec<NewMealPlanComponent>,
    members: &[HouseholdMemberId],
) -> MealPlanEntryView {
    h.service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Household,
            member_id: None,
            planned_on: date!(2026 - 08 - 25),
            planned_time: Some(time!(18:30)),
            slot: MealSlot::Dinner,
            components,
            participants: Some(
                members
                    .iter()
                    .map(|member_id| NewMealParticipant {
                        id: None,
                        member_id: *member_id,
                        allocations: Vec::new(),
                    })
                    .collect(),
            ),
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap()
}

async fn confirm_component_for(
    h: &Harness,
    entry_id: crate::domain::MealPlanEntryId,
    component_id: crate::domain::MealPlanComponentId,
    revision: Revision,
    amount: ConsumedAmount,
    subject: HouseholdMemberId,
) -> StockAffected<MealPlanEntryView> {
    h.service
        .mark_component_eaten_backdated(
            entry_id,
            component_id,
            revision,
            ConfirmMealPlanComponent {
                consumed_on: date!(2026 - 08 - 25),
                consumed_at: None,
                amount,
                actor_id: h.actor_id,
                subject_member_id: Some(subject),
            },
        )
        .await
        .unwrap()
}

async fn cook_standalone(
    h: &Harness,
    recipe_id: RecipeId,
    servings: i64,
    location: StorageLocation,
) -> crate::domain::PreparedBatch {
    let made = Decimal::new(servings, 0);
    h.preparation
        .record(crate::services::RecordPreparation {
            recipe_id,
            source: crate::domain::PreparationSource::Standalone,
            servings_produced: made,
            placements: vec![crate::domain::PortionPlacement::new(location, made)],
            prepared_at: None,
            actor: h.actor_id,
        })
        .await
        .unwrap()
        .into_value()
}

async fn cook(
    h: &Harness,
    entry_id: crate::domain::MealPlanEntryId,
    component_id: crate::domain::MealPlanComponentId,
    recipe_id: RecipeId,
    servings: i64,
) {
    let made = Decimal::new(servings, 0);
    h.preparation
        .record(crate::services::RecordPreparation {
            recipe_id,
            source: crate::domain::PreparationSource::MealPlanComponent {
                entry_id,
                component_id,
            },
            servings_produced: made,
            placements: vec![crate::domain::PortionPlacement::new(
                StorageLocation::Chilled,
                made,
            )],
            prepared_at: None,
            actor: h.actor_id,
        })
        .await
        .unwrap();
}

async fn confirm_component(
    h: &Harness,
    entry_id: crate::domain::MealPlanEntryId,
    component_id: crate::domain::MealPlanComponentId,
    revision: Revision,
    amount: ConsumedAmount,
) -> StockAffected<MealPlanEntryView> {
    h.service
        .mark_component_eaten_backdated(
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
        h.service.mark_component_eaten_backdated(
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
        .mark_component_not_eaten_backdated(
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
    assert_eq!(warning.name, "Chicken");
    assert!(matches!(
        warning.shortfall,
        crate::domain::Shortfall::Short { .. }
    ));
}

#[tokio::test]
async fn a_recipe_component_draws_each_of_its_lines_from_stock() {
    let h = harness();
    let rice = product("Rice", 100);
    h.products.seed(rice.clone());
    let curry = seed_recipe(&h, "Curry", 4, vec![recipe_line(rice.id, 400)]).await;
    let item = h.seed_stock_grams(rice.id, 500);

    let entry = planned(&h, vec![servings_of(curry.id, 1)]).await;
    let component = entry.components[0].component.clone();
    cook(&h, entry.entry.id, component.id, curry.id, 1).await;

    let outcome = confirm_component(
        &h,
        entry.entry.id,
        component.id,
        component.revision,
        ConsumedAmount::Servings(Decimal::ONE),
    )
    .await;

    assert!(outcome.stock.is_empty());
    assert_eq!(h.stock_grams(item).await, dgrams(400));
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
        Arc::new(h.ingredients.clone()),
        Arc::new(InMemoryPreparedMealRepository::new()),
        Arc::new(h.plans.clone()),
        Arc::new(h.recipes.clone()),
        Arc::new(h.batches.clone()),
        Arc::new(h.members.clone()),
        Arc::new(h.settings.clone()),
        Arc::new(FixedClock::new(datetime!(2026-08-24 09:00 UTC))),
    );

    let before = stock_service
        .availability(&[chicken.id], date!(2026 - 08 - 25), date!(2026 - 08 - 25))
        .await
        .unwrap();
    let crate::domain::Availability::Quantified { unallocated, .. } =
        before.products[0].availability
    else {
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
    let crate::domain::Availability::Quantified { unallocated, .. } =
        after.products[0].availability
    else {
        panic!("expected a quantified availability");
    };
    assert_eq!(
        unallocated.amount,
        dgrams(200),
        "the meal is no longer future demand and the 200 g left is really free"
    );
}

async fn household_dinner(h: &Harness, product_id: ProductId, grams: i64) -> MealPlanEntryView {
    h.service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Household,
            member_id: None,
            planned_on: date!(2026 - 08 - 25),
            planned_time: Some(time!(18:30)),
            slot: MealSlot::Dinner,
            components: vec![measured(product_id, grams)],
            participants: None,
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn opting_out_frees_the_slot_for_a_personal_meal() {
    let h = harness();
    h.settings.set_default_all_members_participate(true);
    let food = product("Roast", 150);
    h.products.seed(food.clone());
    let household = household_dinner(&h, food.id, 900).await;

    let clash = h
        .service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Member,
            member_id: Some(h.member_id),
            planned_on: date!(2026 - 08 - 25),
            planned_time: None,
            slot: MealSlot::Dinner,
            components: vec![measured(food.id, 100)],
            participants: None,
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await;
    assert!(clash.is_err());

    let after = h
        .service
        .opt_out(
            household.entry.id,
            household.entry.revision,
            h.actor_id,
            h.member_id,
        )
        .await
        .unwrap();
    assert!(after.entry.participant_for(h.member_id).is_none());
    assert!(after.entry.has_opted_out(h.member_id));

    h.service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Member,
            member_id: Some(h.member_id),
            planned_on: date!(2026 - 08 - 25),
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
async fn opting_out_of_a_future_meal_is_allowed() {
    let h = harness();
    h.settings.set_default_all_members_participate(true);
    let food = product("Pie", 150);
    h.products.seed(food.clone());
    let household = h
        .service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Household,
            member_id: None,
            planned_on: date!(2026 - 09 - 20),
            planned_time: Some(time!(18:30)),
            slot: MealSlot::Dinner,
            components: vec![measured(food.id, 900)],
            participants: None,
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap();

    let after = h
        .service
        .opt_out(
            household.entry.id,
            household.entry.revision,
            h.actor_id,
            h.member_id,
        )
        .await
        .unwrap();
    assert!(after.entry.has_opted_out(h.member_id));
}

#[tokio::test]
async fn opting_out_leaves_the_other_portions_alone_and_a_manager_cannot_re_add() {
    let h = harness();
    h.settings.set_default_all_members_participate(true);
    let morgan = h.add_member("Morgan");
    let taylor = h.add_member("Taylor");
    let food = product("Chilli", 150);
    h.products.seed(food.clone());
    let household = household_dinner(&h, food.id, 900).await;
    assert_eq!(household.entry.participants.len(), 3);

    let after = h
        .service
        .opt_out(
            household.entry.id,
            household.entry.revision,
            h.actor_id,
            taylor,
        )
        .await
        .unwrap();
    assert_eq!(after.entry.participants.len(), 2);
    let prep = &after.components[0].preparation;
    assert_eq!(
        prep.allocated,
        Some(ConsumedAmount::Measure(Quantity::new(
            Decimal::new(600, 0),
            Unit::Gram
        )))
    );
    assert_eq!(
        prep.unallocated,
        Some(ConsumedAmount::Measure(Quantity::new(
            Decimal::new(300, 0),
            Unit::Gram
        )))
    );

    let err = h
        .service
        .set_participants(
            after.entry.id,
            after.entry.revision,
            crate::domain::SetMealParticipants {
                actor_id: h.actor_id,
                guest_groups: Vec::new(),
                participants: vec![h.member_id, morgan, taylor]
                    .into_iter()
                    .map(|member_id| crate::domain::NewMealParticipant {
                        id: None,
                        member_id,
                        allocations: Vec::new(),
                    })
                    .collect(),
            },
        )
        .await;
    assert!(err.is_err());
}

#[tokio::test]
async fn opting_out_is_refused_once_the_portion_is_resolved() {
    let h = harness();
    h.settings.set_default_all_members_participate(true);
    let food = product("Bake", 150);
    h.products.seed(food.clone());
    let household = household_dinner(&h, food.id, 300).await;
    let component = household.components[0].component.clone();

    h.service
        .mark_component_eaten_backdated(
            household.entry.id,
            component.id,
            component.revision,
            ConfirmMealPlanComponent {
                consumed_on: date!(2026 - 08 - 25),
                consumed_at: None,
                amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(300, 0), Unit::Gram)),
                actor_id: h.actor_id,
                subject_member_id: Some(h.member_id),
            },
        )
        .await
        .unwrap();

    let current = h.service.get(household.entry.id).await.unwrap();
    let err = h
        .service
        .opt_out(
            current.entry.id,
            current.entry.revision,
            h.actor_id,
            h.member_id,
        )
        .await;
    assert!(err.is_err());
}

#[tokio::test]
async fn a_household_meal_cannot_use_the_snacks_slot() {
    let h = harness();
    let food = product("Nuts", 150);
    h.products.seed(food.clone());
    let err = h
        .service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Household,
            member_id: None,
            planned_on: date!(2026 - 08 - 25),
            planned_time: Some(time!(15:00)),
            slot: MealSlot::Snacks,
            components: vec![measured(food.id, 50)],
            participants: Some(vec![crate::domain::NewMealParticipant {
                id: None,
                member_id: h.member_id,
                allocations: Vec::new(),
            }]),
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await;
    assert!(err.is_err());
}

#[tokio::test]
async fn slot_attendance_marks_self_catering_and_opted_out_members() {
    let h = harness();
    h.settings.set_default_all_members_participate(true);
    let morgan = h.add_member("Morgan");
    let taylor = h.add_member("Taylor");
    let food = product("Tart", 150);
    h.products.seed(food.clone());

    h.service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Member,
            member_id: Some(morgan),
            planned_on: date!(2026 - 08 - 25),
            planned_time: Some(time!(19:00)),
            slot: MealSlot::Dinner,
            components: vec![measured(food.id, 100)],
            participants: None,
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap();

    let household = h
        .service
        .create(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Household,
            member_id: None,
            planned_on: date!(2026 - 08 - 25),
            planned_time: Some(time!(18:30)),
            slot: MealSlot::Dinner,
            components: vec![measured(food.id, 600)],
            participants: Some(
                vec![h.member_id, taylor]
                    .into_iter()
                    .map(|member_id| crate::domain::NewMealParticipant {
                        id: None,
                        member_id,
                        allocations: Vec::new(),
                    })
                    .collect(),
            ),
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap();

    h.service
        .opt_out(
            household.entry.id,
            household.entry.revision,
            h.actor_id,
            taylor,
        )
        .await
        .unwrap();

    let attendance = h
        .service
        .slot_attendance(date!(2026 - 08 - 25), MealSlot::Dinner, None)
        .await
        .unwrap();
    let by_member: std::collections::HashMap<_, _> = attendance
        .into_iter()
        .map(|(member, state, claimed)| (member, (state, claimed)))
        .collect();
    assert_eq!(
        by_member[&morgan],
        (
            crate::domain::SlotAttendance::SelfCatering,
            Some(time!(19:00))
        )
    );
    assert_eq!(
        by_member[&taylor],
        (crate::domain::SlotAttendance::OptedOut, None)
    );
    assert_eq!(
        by_member[&h.member_id],
        (
            crate::domain::SlotAttendance::Participating,
            Some(time!(18:30))
        )
    );
}

#[tokio::test]
async fn one_member_resolving_does_not_freeze_the_meal_for_a_manager() {
    let h = harness();
    h.settings.set_default_all_members_participate(true);
    let morgan = h.add_member("Morgan");
    let food = product("Gratin", 150);
    let extra = product("Salad", 20);
    h.products.seed(food.clone());
    h.products.seed(extra.clone());
    let household = household_dinner(&h, food.id, 400).await;
    let component = household.components[0].component.clone();

    h.service
        .mark_component_eaten_backdated(
            household.entry.id,
            component.id,
            component.revision,
            ConfirmMealPlanComponent {
                consumed_on: date!(2026 - 08 - 25),
                consumed_at: None,
                amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(200, 0), Unit::Gram)),
                actor_id: h.actor_id,
                subject_member_id: Some(h.member_id),
            },
        )
        .await
        .unwrap();

    let current = h.service.get(household.entry.id).await.unwrap();
    assert_eq!(current.status, MealPlanStatus::PartiallyResolved);

    let updated = h
        .service
        .update(
            current.entry.id,
            current.entry.revision,
            MealPlanEntryPatch {
                components: Some(vec![
                    NewMealPlanComponent {
                        id: Some(component.id),
                        item: component.item,
                        amount: component.amount,
                    },
                    measured(extra.id, 120),
                ]),
                ..Default::default()
            },
            h.actor_id,
        )
        .await
        .unwrap();
    assert_eq!(updated.components.len(), 2);
    assert!(morgan != h.member_id);
}

async fn planned_at(
    h: &Harness,
    on: time::Date,
    at: Option<time::Time>,
    slot: MealSlot,
    components: Vec<NewMealPlanComponent>,
) -> MealPlanEntryView {
    h.service
        .create_backdated(NewMealPlanEntry {
            id: None,
            scope: MealPlanScope::Member,
            member_id: Some(h.member_id),
            planned_on: on,
            planned_time: at,
            slot,
            components,
            participants: None,
            guest_groups: Vec::new(),
            actor_id: h.actor_id,
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn a_meal_whose_time_has_passed_reads_as_assumed() {
    let h = harness();
    h.settings.set_assume_eaten_when_time_passes(true);
    let food = product("Porridge", 150);
    h.products.seed(food.clone());

    let entry = planned_at(
        &h,
        date!(2026 - 08 - 24),
        Some(time!(08:00)),
        MealSlot::Breakfast,
        vec![measured(food.id, 100)],
    )
    .await;

    let view = h.service.get(entry.entry.id).await.unwrap();
    assert_eq!(view.status, MealPlanStatus::Assumed);
    assert_eq!(view.subject_status, MealPlanStatus::Assumed);
    assert!(
        view.components
            .iter()
            .all(|component| component.subject_status == MealPlanStatus::Assumed)
    );
}

#[tokio::test]
async fn a_meal_still_to_come_stays_planned() {
    let h = harness();
    h.settings.set_assume_eaten_when_time_passes(true);
    let food = product("Stew", 150);
    h.products.seed(food.clone());

    let entry = planned_at(
        &h,
        date!(2026 - 08 - 24),
        Some(time!(18:00)),
        MealSlot::Dinner,
        vec![measured(food.id, 100)],
    )
    .await;

    let view = h.service.get(entry.entry.id).await.unwrap();
    assert_eq!(view.status, MealPlanStatus::Planned);
}

#[tokio::test]
async fn turning_the_household_setting_off_returns_everything_to_planned() {
    let h = harness();
    h.settings.set_assume_eaten_when_time_passes(true);
    let food = product("Porridge", 150);
    h.products.seed(food.clone());

    let entry = planned_at(
        &h,
        date!(2026 - 08 - 24),
        Some(time!(08:00)),
        MealSlot::Breakfast,
        vec![measured(food.id, 100)],
    )
    .await;
    assert_eq!(
        h.service.get(entry.entry.id).await.unwrap().status,
        MealPlanStatus::Assumed
    );

    h.settings.set_assume_eaten_when_time_passes(false);
    assert_eq!(
        h.service.get(entry.entry.id).await.unwrap().status,
        MealPlanStatus::Planned
    );
}

#[tokio::test]
async fn an_assumed_meal_can_still_be_edited() {
    let h = harness();
    h.settings.set_assume_eaten_when_time_passes(true);
    let food = product("Porridge", 150);
    let extra = product("Honey", 300);
    h.products.seed(food.clone());
    h.products.seed(extra.clone());

    let entry = planned_at(
        &h,
        date!(2026 - 08 - 24),
        Some(time!(08:00)),
        MealSlot::Breakfast,
        vec![measured(food.id, 100)],
    )
    .await;
    assert_eq!(
        h.service.get(entry.entry.id).await.unwrap().status,
        MealPlanStatus::Assumed
    );

    let updated = h
        .service
        .update(
            entry.entry.id,
            entry.entry.revision,
            MealPlanEntryPatch {
                components: Some(vec![measured(food.id, 100), measured(extra.id, 20)]),
                ..Default::default()
            },
            h.actor_id,
        )
        .await
        .unwrap();
    assert_eq!(updated.components.len(), 2);
}

#[tokio::test]
async fn confirming_an_assumed_meal_records_it_as_eaten() {
    let h = harness();
    h.settings.set_assume_eaten_when_time_passes(true);
    let food = product("Porridge", 150);
    h.products.seed(food.clone());

    let entry = planned_at(
        &h,
        date!(2026 - 08 - 24),
        Some(time!(08:00)),
        MealSlot::Breakfast,
        vec![measured(food.id, 100)],
    )
    .await;

    let confirmed = h
        .service
        .mark_eaten(
            entry.entry.id,
            entry.entry.revision,
            ConfirmMealPlanEntry {
                consumed_on: date!(2026 - 08 - 24),
                consumed_at: None,
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

    assert_eq!(confirmed.status, MealPlanStatus::Eaten);
    assert_eq!(h.records.count(), 1);
}

#[tokio::test]
async fn rejecting_an_assumed_meal_creates_no_record() {
    let h = harness();
    h.settings.set_assume_eaten_when_time_passes(true);
    let food = product("Porridge", 150);
    h.products.seed(food.clone());

    let entry = planned_at(
        &h,
        date!(2026 - 08 - 24),
        Some(time!(08:00)),
        MealSlot::Breakfast,
        vec![measured(food.id, 100)],
    )
    .await;

    let rejected = h
        .service
        .mark_not_eaten(
            entry.entry.id,
            entry.entry.revision,
            OutcomeActor::own(h.actor_id),
        )
        .await
        .unwrap();

    assert_eq!(rejected.status, MealPlanStatus::NotEaten);
    assert_eq!(h.records.count(), 0);
}

#[tokio::test]
async fn recording_different_food_links_it_to_the_meal_it_replaced() {
    let h = harness();
    h.settings.set_assume_eaten_when_time_passes(true);
    let planned_food = product("Porridge", 150);
    let actual_food = product("Bacon Sandwich", 450);
    h.products.seed(planned_food.clone());
    h.products.seed(actual_food.clone());

    let entry = planned_at(
        &h,
        date!(2026 - 08 - 24),
        Some(time!(08:00)),
        MealSlot::Breakfast,
        vec![measured(planned_food.id, 100)],
    )
    .await;

    let reviewed = h
        .service
        .review_outcomes(
            entry.entry.id,
            entry.entry.revision,
            ReviewMealOutcomes {
                consumed_on: date!(2026 - 08 - 24),
                consumed_at: None,
                members: vec![ReviewedMemberOutcome {
                    member_id: h.member_id,
                    outcome: ReviewedMealOutcome::Changed(ChangedMealOutcome {
                        components: Vec::new(),
                        replacements: vec![ReplacementItem {
                            item: MealItemRef::product(actual_food.id),
                            amount: ConsumedAmount::Measure(Quantity::new(
                                Decimal::new(200, 0),
                                Unit::Gram,
                            )),
                        }],
                    }),
                }],
                guests: Vec::new(),
                actor_id: h.actor_id,
            },
        )
        .await
        .unwrap();

    assert_eq!(h.records.count(), 1);
    let records = h
        .records
        .list_for_meal_plan_entry(entry.entry.id)
        .await
        .unwrap();
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.item, MealItemRef::product(actual_food.id));
    assert_eq!(record.meal_plan_entry_id, Some(entry.entry.id));
    assert_eq!(record.meal_plan_component_id, None);

    assert_eq!(reviewed.value.status, MealPlanStatus::NotEaten);
    assert!(
        reviewed
            .value
            .entry
            .participants
            .iter()
            .flat_map(|participant| participant.allocations.iter())
            .all(|allocation| allocation.status == ParticipantStatus::NotEaten)
    );
}

#[tokio::test]
async fn a_changed_outcome_with_nothing_in_it_is_rejected() {
    let h = harness();
    let food = product("Porridge", 150);
    h.products.seed(food.clone());

    let entry = planned_at(
        &h,
        date!(2026 - 08 - 24),
        Some(time!(08:00)),
        MealSlot::Breakfast,
        vec![measured(food.id, 100)],
    )
    .await;

    let error = h
        .service
        .review_outcomes(
            entry.entry.id,
            entry.entry.revision,
            ReviewMealOutcomes {
                consumed_on: date!(2026 - 08 - 24),
                consumed_at: None,
                members: vec![ReviewedMemberOutcome {
                    member_id: h.member_id,
                    outcome: ReviewedMealOutcome::Changed(ChangedMealOutcome::default()),
                }],
                guests: Vec::new(),
                actor_id: h.actor_id,
            },
        )
        .await;
    assert!(error.is_err());
}

#[tokio::test]
async fn needs_review_lists_unresolved_assumptions_oldest_first() {
    let h = harness();
    h.settings.set_assume_eaten_when_time_passes(true);
    let food = product("Porridge", 150);
    h.products.seed(food.clone());

    let older = planned_at(
        &h,
        date!(2025 - 08 - 20),
        Some(time!(08:00)),
        MealSlot::Breakfast,
        vec![measured(food.id, 100)],
    )
    .await;
    let newer = planned_at(
        &h,
        date!(2026 - 08 - 22),
        Some(time!(08:00)),
        MealSlot::Breakfast,
        vec![measured(food.id, 100)],
    )
    .await;
    planned_at(
        &h,
        date!(2026 - 08 - 24),
        Some(time!(18:00)),
        MealSlot::Dinner,
        vec![measured(food.id, 100)],
    )
    .await;

    let review = h.service.needs_review(h.member_id, false).await.unwrap();
    let ids: Vec<_> = review.personal.iter().map(|view| view.entry.id).collect();
    assert_eq!(ids, vec![older.entry.id, newer.entry.id]);
    assert!(review.household.is_empty());
}

#[tokio::test]
async fn needs_review_is_empty_when_assumptions_are_switched_off() {
    let h = harness();
    let food = product("Porridge", 150);
    h.products.seed(food.clone());
    planned_at(
        &h,
        date!(2026 - 08 - 20),
        Some(time!(08:00)),
        MealSlot::Breakfast,
        vec![measured(food.id, 100)],
    )
    .await;

    let review = h.service.needs_review(h.member_id, true).await.unwrap();
    assert!(review.personal.is_empty());
}

fn ingredient_line(ingredient_id: crate::domain::IngredientId, grams: i64) -> RecipeComponent {
    RecipeComponent {
        id: crate::domain::RecipeComponentId::new(),
        requirement: crate::domain::RecipeRequirement::Ingredient { ingredient_id },
        source_text: None,
        amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(grams, 0), Unit::Gram)),
        position: 0,
    }
}

fn mapped_product(name: &str, ingredient_id: crate::domain::IngredientId) -> Product {
    let mut product = product(name, 100);
    product.mapped_ingredient_id = Some(ingredient_id);
    product
}

fn dated_stock(
    h: &Harness,
    product_id: ProductId,
    grams: i64,
    use_by: time::Date,
) -> crate::domain::StockItemId {
    let item = StockItem {
        id: crate::domain::StockItemId::new(),
        subject: StockSubject::product(product_id),
        level: StockLevel::Exact {
            quantity: Quantity::new(Decimal::new(grams, 0), Unit::Gram),
        },
        storage_location: StorageLocation::Chilled,
        source_date: None,
        usability_deadline: Some(crate::domain::UsabilityDeadline {
            date: use_by,
            basis: None,
        }),
        note: None,
        revision: Revision::INITIAL,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
        archived_at: None,
    };
    let id = item.id;
    h.stock.seed(item);
    id
}

#[tokio::test]
async fn a_pooled_ingredient_draw_spans_two_products_in_use_by_order() {
    let h = harness();
    let rice_id = crate::domain::IngredientId::new();
    let tesco = mapped_product("Tesco Basmati", rice_id);
    let sains = mapped_product("Sainsbury's Basmati", rice_id);
    h.products.seed(tesco.clone());
    h.products.seed(sains.clone());
    let older = dated_stock(&h, tesco.id, 60, date!(2026 - 08 - 26));
    let newer = dated_stock(&h, sains.id, 200, date!(2026 - 09 - 30));

    let curry = seed_recipe(&h, "Curry", 4, vec![ingredient_line(rice_id, 400)]).await;
    let entry = planned(&h, vec![servings_of(curry.id, 1)]).await;
    let component = entry.components[0].component.clone();
    cook(&h, entry.entry.id, component.id, curry.id, 1).await;

    let outcome = confirm_component(
        &h,
        entry.entry.id,
        component.id,
        component.revision,
        ConsumedAmount::Servings(Decimal::ONE),
    )
    .await;

    assert!(
        outcome.stock.is_empty(),
        "100 g was available across the pool"
    );
    assert_eq!(h.stock_grams(older).await, dgrams(0));
    assert_eq!(h.stock_grams(newer).await, dgrams(160));
}

#[tokio::test]
async fn reopening_a_recipe_meal_returns_the_serving_but_not_the_raw_ingredients() {
    let h = harness();
    let rice_id = crate::domain::IngredientId::new();
    let tesco = mapped_product("Tesco Basmati", rice_id);
    let sains = mapped_product("Sainsbury's Basmati", rice_id);
    h.products.seed(tesco.clone());
    h.products.seed(sains.clone());
    let older = dated_stock(&h, tesco.id, 60, date!(2026 - 08 - 26));
    let newer = dated_stock(&h, sains.id, 200, date!(2026 - 09 - 30));

    let curry = seed_recipe(&h, "Curry", 4, vec![ingredient_line(rice_id, 400)]).await;
    let entry = planned(&h, vec![servings_of(curry.id, 1)]).await;
    let component = entry.components[0].component.clone();
    cook(&h, entry.entry.id, component.id, curry.id, 1).await;

    let after_eating = confirm_component(
        &h,
        entry.entry.id,
        component.id,
        component.revision,
        ConsumedAmount::Servings(Decimal::ONE),
    )
    .await;

    let portion = h.portion_for(component.id).await;
    assert_eq!(h.stock_servings(portion).await, Decimal::ZERO);

    h.service
        .reopen_component(
            entry.entry.id,
            component.id,
            after_eating.value.components[0].component.revision,
            OutcomeActor::own(h.actor_id),
        )
        .await
        .unwrap();

    assert_eq!(h.stock_servings(portion).await, Decimal::ONE);
    assert_eq!(h.stock_grams(older).await, dgrams(0));
    assert_eq!(h.stock_grams(newer).await, dgrams(160));
}

#[tokio::test]
async fn a_recipe_pinning_a_product_and_needing_its_ingredient_draws_both() {
    let h = harness();
    let rice_id = crate::domain::IngredientId::new();
    let tesco = mapped_product("Tesco Basmati", rice_id);
    h.products.seed(tesco.clone());
    let item = h.seed_stock_grams(tesco.id, 500);

    let curry = seed_recipe(
        &h,
        "Curry",
        1,
        vec![recipe_line(tesco.id, 100), ingredient_line(rice_id, 50)],
    )
    .await;
    let entry = planned(&h, vec![servings_of(curry.id, 1)]).await;
    let component = entry.components[0].component.clone();
    cook(&h, entry.entry.id, component.id, curry.id, 1).await;

    confirm_component(
        &h,
        entry.entry.id,
        component.id,
        component.revision,
        ConsumedAmount::Servings(Decimal::ONE),
    )
    .await;

    assert_eq!(h.stock_grams(item).await, dgrams(350));
}

#[tokio::test]
async fn a_recipe_cannot_be_eaten_before_it_has_been_cooked() {
    let h = harness();
    let rice_id = crate::domain::IngredientId::new();
    let tesco = mapped_product("Tesco Basmati", rice_id);
    h.products.seed(tesco.clone());
    let rice = h.seed_stock_grams(tesco.id, 1000);

    let curry = seed_recipe(&h, "Curry", 4, vec![ingredient_line(rice_id, 400)]).await;
    let entry = planned(&h, vec![servings_of(curry.id, 4)]).await;
    let component = entry.components[0].component.clone();

    let error = h
        .service
        .mark_component_eaten_backdated(
            entry.entry.id,
            component.id,
            component.revision,
            ConfirmMealPlanComponent {
                consumed_on: date!(2026 - 08 - 25),
                consumed_at: None,
                amount: ConsumedAmount::Servings(Decimal::ONE),
                actor_id: h.actor_id,
                subject_member_id: None,
            },
        )
        .await
        .unwrap_err();

    assert!(
        error.to_string().contains("cooked"),
        "expected a prompt to record the cook, got {error}"
    );
    assert_eq!(
        h.stock_grams(rice).await,
        dgrams(1000),
        "eating must not move raw stock on its own"
    );
    assert_eq!(h.batches.count(), 0, "and it must not silently cook either");
}

#[tokio::test]
async fn cooking_a_recipe_consumes_raw_stock_and_leaves_the_uneaten_servings_as_a_portion() {
    let h = harness();
    let rice_id = crate::domain::IngredientId::new();
    let tesco = mapped_product("Tesco Basmati", rice_id);
    h.products.seed(tesco.clone());
    let rice = h.seed_stock_grams(tesco.id, 1000);

    let curry = seed_recipe(&h, "Curry", 4, vec![ingredient_line(rice_id, 400)]).await;
    let entry = planned(&h, vec![servings_of(curry.id, 4)]).await;
    let component = entry.components[0].component.clone();
    cook(&h, entry.entry.id, component.id, curry.id, 4).await;

    confirm_component(
        &h,
        entry.entry.id,
        component.id,
        component.revision,
        ConsumedAmount::Servings(Decimal::ONE),
    )
    .await;

    assert_eq!(
        h.stock_grams(rice).await,
        dgrams(600),
        "cooking four servings should draw the recipe's full 400 g of rice"
    );
    let portion = h.portion_for(component.id).await;
    assert_eq!(
        h.stock_servings(portion).await,
        Decimal::new(3, 0),
        "one of the four cooked servings was eaten, three remain as leftovers"
    );
}

#[tokio::test]
async fn cooked_food_pools_across_cooks_and_the_oldest_is_eaten_first() {
    let h = harness();
    let rice_id = crate::domain::IngredientId::new();
    let tesco = mapped_product("Tesco Basmati", rice_id);
    h.products.seed(tesco.clone());
    h.seed_stock_grams(tesco.id, 4000);

    let curry = seed_recipe(&h, "Curry", 4, vec![ingredient_line(rice_id, 400)]).await;
    let older = cook_standalone(&h, curry.id, 2, StorageLocation::Chilled).await;
    let newer = cook_standalone(&h, curry.id, 3, StorageLocation::Frozen).await;

    let entry = planned(
        &h,
        vec![NewMealPlanComponent {
            id: None,
            item: crate::domain::MealItemRef::dish(curry.id),
            amount: ConsumedAmount::Servings(Decimal::new(3, 0)),
        }],
    )
    .await;
    let component = entry.components[0].component.clone();

    confirm_component(
        &h,
        entry.entry.id,
        component.id,
        component.revision,
        ConsumedAmount::Servings(Decimal::new(3, 0)),
    )
    .await;

    assert_eq!(
        h.stock_servings(h.portion_for_batch(older.id).await).await,
        Decimal::ZERO,
        "the older cook is emptied first"
    );
    assert_eq!(
        h.stock_servings(h.portion_for_batch(newer.id).await).await,
        Decimal::new(2, 0),
        "the remaining serving comes out of the newer cook"
    );
}

#[tokio::test]
async fn cooked_food_availability_pools_every_cook_and_nets_off_planned_dishes() {
    let h = harness();
    let rice_id = crate::domain::IngredientId::new();
    let tesco = mapped_product("Tesco Basmati", rice_id);
    h.products.seed(tesco.clone());
    h.seed_stock_grams(tesco.id, 4000);

    let curry = seed_recipe(&h, "Curry", 4, vec![ingredient_line(rice_id, 400)]).await;
    cook_standalone(&h, curry.id, 2, StorageLocation::Chilled).await;
    cook_standalone(&h, curry.id, 3, StorageLocation::Frozen).await;

    planned(
        &h,
        vec![NewMealPlanComponent {
            id: None,
            item: crate::domain::MealItemRef::dish(curry.id),
            amount: ConsumedAmount::Servings(Decimal::new(3, 0)),
        }],
    )
    .await;

    let stock_service = crate::services::StockService::new(
        Arc::new(h.stock.clone()),
        Arc::new(h.products.clone()),
        Arc::new(h.ingredients.clone()),
        Arc::new(InMemoryPreparedMealRepository::new()),
        Arc::new(h.plans.clone()),
        Arc::new(h.recipes.clone()),
        Arc::new(h.batches.clone()),
        Arc::new(h.members.clone()),
        Arc::new(h.settings.clone()),
        Arc::new(FixedClock::new(datetime!(2026-08-24 09:00 UTC))),
    );

    let report = stock_service
        .availability_overview(date!(2026 - 08 - 25), date!(2026 - 08 - 25))
        .await
        .unwrap();

    let cooked = report
        .cooked_food
        .iter()
        .find(|row| row.recipe_id == curry.id)
        .expect("cooked curry should appear once, however many times it was cooked");
    assert_eq!(
        report.cooked_food.len(),
        1,
        "the two cooks pool into one row"
    );
    match &cooked.availability {
        crate::domain::Availability::Quantified {
            on_hand,
            planned_demand,
            unallocated,
            ..
        } => {
            assert_eq!(on_hand.amount, Decimal::new(5, 0));
            assert_eq!(planned_demand.amount, Decimal::new(3, 0));
            assert_eq!(unallocated.amount, Decimal::new(2, 0));
        }
        other => panic!("expected a quantified level, got {other:?}"),
    }
}

#[tokio::test]
async fn a_dish_can_be_planned_and_eaten_without_cooking_the_recipe_again() {
    let h = harness();
    let rice_id = crate::domain::IngredientId::new();
    let tesco = mapped_product("Tesco Basmati", rice_id);
    h.products.seed(tesco.clone());
    let rice = h.seed_stock_grams(tesco.id, 2000);

    let curry = seed_recipe(&h, "Curry", 4, vec![ingredient_line(rice_id, 400)]).await;
    let cooked = h
        .preparation
        .record(crate::services::RecordPreparation {
            recipe_id: curry.id,
            source: crate::domain::PreparationSource::Standalone,
            servings_produced: Decimal::new(4, 0),
            placements: vec![crate::domain::PortionPlacement::new(
                StorageLocation::Frozen,
                Decimal::new(4, 0),
            )],
            prepared_at: None,
            actor: h.actor_id,
        })
        .await
        .unwrap()
        .into_value();

    let raw_after_cooking = h.stock_grams(rice).await;

    let entry = planned(
        &h,
        vec![NewMealPlanComponent {
            id: None,
            item: crate::domain::MealItemRef::dish(curry.id),
            amount: ConsumedAmount::Servings(Decimal::ONE),
        }],
    )
    .await;
    let component = entry.components[0].component.clone();
    assert_eq!(entry.components[0].item_name, "Curry");

    confirm_component(
        &h,
        entry.entry.id,
        component.id,
        component.revision,
        ConsumedAmount::Servings(Decimal::ONE),
    )
    .await;

    assert_eq!(
        h.stock_grams(rice).await,
        raw_after_cooking,
        "eating a dish must not shop for the recipe's ingredients again"
    );
    assert_eq!(
        h.stock_servings(h.portion_for_batch(cooked.id).await).await,
        Decimal::new(3, 0),
        "one of the four frozen servings was eaten"
    );
}

#[tokio::test]
async fn a_forecast_above_the_head_count_is_spare_rather_than_bigger_portions() {
    let h = harness();
    let other = h.add_member("Sam");
    let food = product("Curry", 100);
    h.products.seed(food.clone());
    let curry = seed_recipe(&h, "Curry", 4, vec![recipe_line(food.id, 200)]).await;

    let entry = household_planned(&h, vec![servings_of(curry.id, 5)], &[h.member_id, other]).await;

    for participant in &entry.entry.participants {
        assert_eq!(
            participant.allocations[0].allocated,
            ConsumedAmount::Servings(Decimal::ONE),
            "a serving is already one person's portion"
        );
    }
    assert_eq!(
        entry.components[0].preparation.unallocated,
        Some(ConsumedAmount::Servings(Decimal::new(3, 0)))
    );
}

#[tokio::test]
async fn cooking_more_than_planned_leaves_the_surplus_unallocated() {
    let h = harness();
    let rice_id = crate::domain::IngredientId::new();
    let tesco = mapped_product("Tesco Basmati", rice_id);
    h.products.seed(tesco.clone());
    h.seed_stock_grams(tesco.id, 2000);

    let curry = seed_recipe(&h, "Curry", 4, vec![ingredient_line(rice_id, 400)]).await;
    let entry = planned(&h, vec![servings_of(curry.id, 2)]).await;
    let component = entry.components[0].component.clone();
    cook(&h, entry.entry.id, component.id, curry.id, 5).await;

    let after = h.service.get(entry.entry.id).await.unwrap();
    let prep = &after.components[0].preparation;
    assert_eq!(prep.prepared, ConsumedAmount::Servings(Decimal::new(5, 0)));
    assert_eq!(
        prep.unallocated,
        Some(ConsumedAmount::Servings(Decimal::new(4, 0)))
    );
    assert!(!prep.shortage);
}

#[tokio::test]
async fn cooking_less_than_the_people_eating_is_reported_as_a_shortage() {
    let h = harness();
    let sam = h.add_member("Sam");
    let ash = h.add_member("Ash");
    let rice_id = crate::domain::IngredientId::new();
    let tesco = mapped_product("Tesco Basmati", rice_id);
    h.products.seed(tesco.clone());
    h.seed_stock_grams(tesco.id, 2000);

    let curry = seed_recipe(&h, "Curry", 4, vec![ingredient_line(rice_id, 400)]).await;
    let entry =
        household_planned(&h, vec![servings_of(curry.id, 6)], &[h.member_id, sam, ash]).await;
    let component = entry.components[0].component.clone();
    cook(&h, entry.entry.id, component.id, curry.id, 1).await;

    let after = h.service.get(entry.entry.id).await.unwrap();
    let prep = &after.components[0].preparation;
    assert_eq!(prep.prepared, ConsumedAmount::Servings(Decimal::ONE));
    assert_eq!(
        prep.allocated,
        Some(ConsumedAmount::Servings(Decimal::new(3, 0)))
    );
    assert!(prep.shortage);
}

#[tokio::test]
async fn a_second_eater_draws_from_the_portion_without_cooking_the_recipe_again() {
    let h = harness();
    let rice_id = crate::domain::IngredientId::new();
    let tesco = mapped_product("Tesco Basmati", rice_id);
    h.products.seed(tesco.clone());
    let rice = h.seed_stock_grams(tesco.id, 1000);
    let other = h.add_member("Sam");

    let curry = seed_recipe(&h, "Curry", 4, vec![ingredient_line(rice_id, 400)]).await;
    let entry = household_planned(&h, vec![servings_of(curry.id, 4)], &[h.member_id, other]).await;
    let component = entry.components[0].component.clone();
    cook(&h, entry.entry.id, component.id, curry.id, 4).await;

    let after_first = confirm_component_for(
        &h,
        entry.entry.id,
        component.id,
        component.revision,
        ConsumedAmount::Servings(Decimal::ONE),
        h.member_id,
    )
    .await;
    confirm_component_for(
        &h,
        entry.entry.id,
        component.id,
        after_first.value.components[0].component.revision,
        ConsumedAmount::Servings(Decimal::ONE),
        other,
    )
    .await;

    assert_eq!(
        h.stock_grams(rice).await,
        dgrams(600),
        "the raw rice should move exactly once, however many people eat"
    );
    assert_eq!(h.batches.count(), 1, "the meal should be cooked only once");
    let portion = h.portion_for(component.id).await;
    assert_eq!(h.stock_servings(portion).await, Decimal::new(2, 0));
}

#[tokio::test]
async fn a_plain_product_component_still_draws_its_stock_on_first_confirmation() {
    let h = harness();
    let yoghurt = product("Yoghurt", 60);
    h.products.seed(yoghurt.clone());
    let pot = h.seed_stock_grams(yoghurt.id, 500);

    let entry = planned(&h, vec![measured(yoghurt.id, 150)]).await;
    let component = entry.components[0].component.clone();

    confirm_component(
        &h,
        entry.entry.id,
        component.id,
        component.revision,
        ConsumedAmount::Measure(Quantity::new(Decimal::new(150, 0), Unit::Gram)),
    )
    .await;

    assert_eq!(h.stock_grams(pot).await, dgrams(350));
    assert_eq!(
        h.batches.count(),
        0,
        "a product is not cooked, so nothing should be prepared"
    );
}

#[tokio::test]
async fn eating_a_generic_food_records_the_product_actually_drawn_not_the_average() {
    let h = harness();
    let lasagne_id = crate::domain::IngredientId::new();
    h.ingredients.seed(crate::domain::Ingredient {
        id: lasagne_id,
        name: "Frozen lasagne".to_owned(),
        default_unit: Unit::Item,
        shopping_section: None,
        track_stock: None,
        provenance: Provenance::local(),
        revision: Revision::INITIAL,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
        archived_at: None,
    });
    let mut cheap = product("Value Lasagne", 100);
    cheap.mapped_ingredient_id = Some(lasagne_id);
    let mut posh = product("Finest Lasagne", 400);
    posh.mapped_ingredient_id = Some(lasagne_id);
    h.products.seed(cheap.clone());
    h.products.seed(posh.clone());
    h.seed_stock_grams(posh.id, 500);

    let entry = planned(
        &h,
        vec![NewMealPlanComponent {
            id: None,
            item: MealItemRef::ingredient(lasagne_id),
            amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(100, 0), Unit::Gram)),
        }],
    )
    .await;
    let component = entry.components[0].component.clone();

    confirm_component(
        &h,
        entry.entry.id,
        component.id,
        component.revision,
        component.amount,
    )
    .await;

    assert_eq!(h.records.count(), 1);
    let record = h
        .records
        .list_for_meal_plan_entry(entry.entry.id)
        .await
        .unwrap()
        .remove(0);
    assert_eq!(
        record.nutrition.energy_kcal,
        Some(Decimal::new(400, 0)),
        "the record should reflect the posh lasagne actually in stock, not the mean of both"
    );
    assert_eq!(record.quality, crate::domain::NutritionQuality::Known);
}
