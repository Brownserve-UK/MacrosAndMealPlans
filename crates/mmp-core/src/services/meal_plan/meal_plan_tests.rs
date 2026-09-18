use std::sync::Arc;

use rust_decimal::Decimal;
use time::OffsetDateTime;
use time::macros::{date, datetime, time};

use super::*;
use crate::domain::{
    ActualMealPlanComponent, AdHocKind, ChangedMealOutcome, ConfirmMealPlanComponent,
    ConfirmMealPlanEntry, ConsumedAmount, HouseholdMember, HouseholdMemberId, MealAttendance,
    MealGroupPatch, MealItemRef, MealOccasionPatch, MealPlanStatus, MealSlot, NewConsumptionRecord,
    NewMealGroup, NewMealGuestGroup, NewMealOccasion, NewMealParticipant, NewMealPlanComponent,
    NewNutritionTarget, NutritionFacts, NutritionGoals, NutritionQuality, OutcomeActor,
    ParticipantStatus, Product, ProductId, Provenance, Quantity, Recipe, RecipeComponent, RecipeId,
    RecipeVisibility, ReplacementItem, ReviewMealOutcomes, ReviewedMealOutcome,
    ReviewedMemberOutcome, Revision, StockItem, StockLevel, StockSubject, StorageLocation, Unit,
    UserId, WeightDisplay,
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
        Arc::new(settings.clone()),
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
        Arc::new(settings.clone()),
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

fn everyone(components: Vec<NewMealPlanComponent>) -> NewMealGroup {
    NewMealGroup::for_everyone(components)
}

fn only(members: &[HouseholdMemberId], components: Vec<NewMealPlanComponent>) -> NewMealGroup {
    NewMealGroup {
        everyone: false,
        participants: members
            .iter()
            .map(|member_id| NewMealParticipant::member(*member_id))
            .collect(),
        ..NewMealGroup::for_everyone(components)
    }
}

fn occasion_input(
    h: &Harness,
    on: time::Date,
    at: Option<time::Time>,
    slot: MealSlot,
    group: NewMealGroup,
) -> NewMealOccasion {
    NewMealOccasion {
        id: None,
        planned_on: on,
        slot,
        planned_time: at,
        note: None,
        group,
        actor_id: h.actor_id,
    }
}

fn last_group(view: MealOccasionView) -> MealPlanEntryView {
    view.groups
        .into_iter()
        .last()
        .expect("the occasion should hold at least one group")
        .entry
}

async fn try_plan(
    h: &Harness,
    on: time::Date,
    at: Option<time::Time>,
    slot: MealSlot,
    group: NewMealGroup,
) -> Result<MealPlanEntryView> {
    h.service
        .create_occasion(occasion_input(h, on, at, slot, group))
        .await
        .map(last_group)
}

async fn plan(
    h: &Harness,
    on: time::Date,
    at: Option<time::Time>,
    slot: MealSlot,
    group: NewMealGroup,
) -> MealPlanEntryView {
    try_plan(h, on, at, slot, group).await.unwrap()
}

async fn planned(h: &Harness, components: Vec<NewMealPlanComponent>) -> MealPlanEntryView {
    plan(
        h,
        date!(2026 - 08 - 25),
        Some(time!(18:30)),
        MealSlot::Dinner,
        everyone(components),
    )
    .await
}

async fn update_components(
    h: &Harness,
    entry: &MealPlanEntryView,
    components: Vec<NewMealPlanComponent>,
) -> Result<MealPlanEntryView> {
    h.service
        .update_group(
            entry.entry.id,
            entry.entry.revision,
            MealGroupPatch {
                components: Some(components),
                ..Default::default()
            },
            h.actor_id,
        )
        .await?;
    h.service.get(entry.entry.id).await
}

async fn set_participants(
    h: &Harness,
    entry: &MealPlanEntryView,
    participants: Vec<NewMealParticipant>,
) -> Result<MealPlanEntryView> {
    h.service
        .update_group(
            entry.entry.id,
            entry.entry.revision,
            MealGroupPatch {
                everyone: Some(false),
                participants: Some(participants),
                ..Default::default()
            },
            h.actor_id,
        )
        .await?;
    h.service.get(entry.entry.id).await
}

#[tokio::test]
async fn a_second_meal_in_the_same_cell_joins_the_occasion_as_another_group() {
    let h = harness();
    let sam = h.add_member("Sam");
    let food = product("Food", 200);
    h.products.seed(food.clone());
    let first = planned(&h, vec![measured(food.id, 100)]).await;

    let second = plan(
        &h,
        date!(2026 - 08 - 25),
        Some(time!(19:00)),
        MealSlot::Dinner,
        only(&[sam], vec![measured(food.id, 50)]),
    )
    .await;

    assert_eq!(second.entry.occasion_id, first.entry.occasion_id);
    assert_eq!(h.plans.occasion_count(), 1);
    let occasion = h
        .service
        .get_occasion(first.entry.occasion_id)
        .await
        .unwrap();
    assert_eq!(occasion.groups.len(), 2);
    assert_eq!(
        occasion.occasion.planned_time,
        Some(time!(18:30)),
        "joining a cell keeps the occasion's own time"
    );
    let seated: Vec<Vec<HouseholdMemberId>> = occasion
        .groups
        .iter()
        .map(|group| group.diners.iter().map(|diner| diner.member_id).collect())
        .collect();
    assert_eq!(seated, vec![vec![h.member_id], vec![sam]]);
}

#[tokio::test]
async fn snacks_are_one_occasion_per_day_with_no_usual_time() {
    let h = harness();
    let food = product("Food", 200);
    h.products.seed(food.clone());
    let snack = plan(
        &h,
        date!(2026 - 08 - 25),
        None,
        MealSlot::Snacks,
        everyone(vec![measured(food.id, 100)]),
    )
    .await;
    let occasion = h
        .service
        .get_occasion(snack.entry.occasion_id)
        .await
        .unwrap();
    assert_eq!(occasion.effective_time, None);

    let timed = h
        .service
        .update_occasion(
            occasion.occasion.id,
            occasion.occasion.revision,
            MealOccasionPatch {
                planned_time: Some(Some(time!(21:00))),
                ..Default::default()
            },
            h.actor_id,
        )
        .await
        .unwrap();
    assert_eq!(timed.effective_time, Some(time!(21:00)));
    assert_eq!(timed.groups[0].entry.entry.planned_time, Some(time!(21:00)));

    let second_everyone = try_plan(
        &h,
        date!(2026 - 08 - 25),
        Some(time!(15:00)),
        MealSlot::Snacks,
        everyone(vec![measured(food.id, 25)]),
    )
    .await;
    assert!(matches!(second_everyone, Err(CoreError::Conflict { .. })));

    let another = plan(
        &h,
        date!(2026 - 08 - 25),
        Some(time!(15:00)),
        MealSlot::Snacks,
        only(&[h.member_id], vec![measured(food.id, 25)]),
    )
    .await;
    assert_eq!(another.entry.occasion_id, snack.entry.occasion_id);
    assert_eq!(h.plans.occasion_count(), 1);
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

    let updated = update_components(&h, &entry, components).await.unwrap();

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
    update_components(&h, &entry, components).await.unwrap();

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

    let error = update_components(&h, &resolved, vec![measured(food.id, 120)])
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

    let retained = update_components(&h, &entry, vec![measured(food.id, 120)]).await;
    assert!(retained.is_ok());

    let newly_added = try_plan(
        &h,
        date!(2026 - 08 - 26),
        None,
        MealSlot::Lunch,
        everyone(vec![measured(food.id, 100)]),
    )
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

    let edited = update_components(&h, &reopened, vec![measured(food.id, 150)])
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
    let error = try_plan(
        &h,
        date!(2026 - 08 - 20),
        None,
        MealSlot::Dinner,
        everyone(vec![measured(food.id, 100)]),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, CoreError::Validation(_)));
}

#[tokio::test]
async fn date_policy_allows_a_one_day_grace_into_the_past() {
    let h = harness();
    let food = product("Food", 200);
    h.products.seed(food.clone());
    plan(
        &h,
        date!(2026 - 08 - 23),
        None,
        MealSlot::Dinner,
        everyone(vec![measured(food.id, 100)]),
    )
    .await;
}

#[tokio::test]
async fn date_policy_forbids_moving_a_plan_into_the_past() {
    let h = harness();
    let food = product("Food", 200);
    h.products.seed(food.clone());
    let entry = planned(&h, vec![measured(food.id, 100)]).await;
    let occasion = h
        .service
        .get_occasion(entry.entry.occasion_id)
        .await
        .unwrap();

    let error = h
        .service
        .move_occasion(
            occasion.occasion.id,
            occasion.occasion.revision,
            date!(2026 - 08 - 20),
            MealSlot::Dinner,
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
    let entry = plan(
        &h,
        date!(2026 - 08 - 30),
        None,
        MealSlot::Dinner,
        everyone(vec![measured(food.id, 100)]),
    )
    .await;

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

    let entry = plan(
        &h,
        date!(2026 - 08 - 25),
        Some(time!(18:30)),
        MealSlot::Dinner,
        NewMealGroup {
            cooking_servings: Some(2),
            ..everyone(vec![servings_of(curry.id, 2)])
        },
    )
    .await;

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

    let error = try_plan(
        &h,
        date!(2026 - 08 - 25),
        Some(time!(18:30)),
        MealSlot::Dinner,
        everyone(vec![servings_of(curry.id, 1)]),
    )
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

    let error = try_plan(
        &h,
        date!(2026 - 08 - 25),
        Some(time!(18:30)),
        MealSlot::Dinner,
        everyone(vec![NewMealPlanComponent {
            id: None,
            item: MealItemRef::recipe(curry.id),
            amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(200, 0), Unit::Gram)),
        }]),
    )
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
async fn an_everyone_group_seats_every_active_member() {
    let h = harness();
    let morgan = h.add_member("Morgan");
    let taylor = h.add_member("Taylor");
    let food = product("Stew", 200);
    h.products.seed(food.clone());

    let entry = planned(&h, vec![measured(food.id, 600)]).await;

    let member_ids: std::collections::HashSet<_> = entry
        .participants
        .iter()
        .map(|participant| participant.member_id)
        .collect();
    assert_eq!(member_ids.len(), 3);
    assert!(member_ids.contains(&h.member_id));
    assert!(member_ids.contains(&morgan));
    assert!(member_ids.contains(&taylor));
    assert_eq!(entry.entry.serves(), 3);
}

#[tokio::test]
async fn a_member_who_joins_the_household_later_is_seated_at_everyone_meals() {
    let h = harness();
    let food = product("Stew", 200);
    h.products.seed(food.clone());
    let entry = planned(&h, vec![measured(food.id, 600)]).await;
    assert_eq!(entry.participants.len(), 1);

    let newcomer = h.add_member("Newcomer");

    let reloaded = h.service.get(entry.entry.id).await.unwrap();
    assert!(
        reloaded
            .participants
            .iter()
            .any(|participant| participant.member_id == newcomer)
    );
    assert_eq!(reloaded.entry.serves(), 2);
}

#[tokio::test]
async fn an_explicit_group_seats_only_its_members() {
    let h = harness();
    h.add_member("Morgan");
    let food = product("Toast", 120);
    h.products.seed(food.clone());

    let entry = plan(
        &h,
        date!(2026 - 08 - 25),
        Some(time!(18:30)),
        MealSlot::Dinner,
        only(&[h.member_id], vec![measured(food.id, 60)]),
    )
    .await;

    assert_eq!(entry.participants.len(), 1);
    assert_eq!(entry.participants[0].member_id, h.member_id);
    assert!(!entry.entry.everyone);
}

#[tokio::test]
async fn a_participant_sees_only_their_own_share_and_outcome() {
    let h = harness();
    let taylor = h.add_member("Taylor");
    let food = product("Curry", 100);
    h.products.seed(food.clone());
    let entry = planned(&h, vec![measured(food.id, 400)]).await;

    let with_taylor = set_participants(
        &h,
        &entry,
        vec![
            crate::domain::NewMealParticipant {
                id: None,
                member_id: h.member_id,
                note: None,
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
                note: None,
                allocations: vec![crate::domain::NewMealParticipantAllocation {
                    component_id: entry.components[0].component.id,
                    allocated: ConsumedAmount::Measure(Quantity::new(
                        Decimal::new(100, 0),
                        Unit::Gram,
                    )),
                }],
            },
        ],
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

    let shared = set_participants(
        &h,
        &entry,
        vec![
            crate::domain::NewMealParticipant {
                id: None,
                member_id: h.member_id,
                note: None,
                allocations: vec![crate::domain::NewMealParticipantAllocation {
                    component_id,
                    allocated: ConsumedAmount::Measure(Quantity::new(dgrams(300), Unit::Gram)),
                }],
            },
            crate::domain::NewMealParticipant {
                id: None,
                member_id: taylor,
                note: None,
                allocations: vec![crate::domain::NewMealParticipantAllocation {
                    component_id,
                    allocated: ConsumedAmount::Measure(Quantity::new(dgrams(100), Unit::Gram)),
                }],
            },
        ],
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
    cooking_servings: Option<i32>,
) -> MealPlanEntryView {
    plan(
        h,
        date!(2026 - 08 - 25),
        Some(time!(18:30)),
        MealSlot::Dinner,
        NewMealGroup {
            cooking_servings,
            ..only(members, components)
        },
    )
    .await
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
    let other = h.add_member("Other");
    let chicken = product("Chicken", 120);
    h.products.seed(chicken.clone());
    let item = h.seed_stock_grams(chicken.id, 500);

    let created = household_dinner(&h, chicken.id, 300).await;
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
    planned(h, vec![measured(product_id, grams)]).await
}

async fn occasion_of(h: &Harness, entry: &MealPlanEntryView) -> MealOccasionView {
    h.service
        .get_occasion(entry.entry.occasion_id)
        .await
        .unwrap()
}

fn diners_of(view: &MealOccasionView, index: usize) -> Vec<HouseholdMemberId> {
    view.groups[index]
        .diners
        .iter()
        .map(|diner| diner.member_id)
        .collect()
}

#[tokio::test]
async fn eating_elsewhere_takes_you_out_of_the_everyone_group() {
    let h = harness();
    let morgan = h.add_member("Morgan");
    let taylor = h.add_member("Taylor");
    let food = product("Chilli", 150);
    h.products.seed(food.clone());
    let household = household_dinner(&h, food.id, 900).await;
    assert_eq!(household.entry.participants.len(), 3);

    let after = h
        .service
        .set_attendance(
            household.entry.occasion_id,
            taylor,
            MealAttendance::Elsewhere,
            h.actor_id,
        )
        .await
        .unwrap();

    assert_eq!(after.absent_member_ids, vec![taylor]);
    assert!(after.unaccounted_member_ids.is_empty());
    let mut seated = diners_of(&after, 0);
    seated.sort();
    let mut expected = vec![h.member_id, morgan];
    expected.sort();
    assert_eq!(seated, expected);
    assert_eq!(after.groups[0].serves, 2);
    let prep = &after.groups[0].entry.components[0].preparation;
    assert_eq!(
        prep.allocated,
        Some(ConsumedAmount::Measure(Quantity::new(
            Decimal::new(900, 0),
            Unit::Gram
        ))),
        "the 900 g is reshared between the two people still eating"
    );
    assert!(
        after.groups[0]
            .entry
            .participants
            .iter()
            .all(|participant| participant.allocations[0].allocated
                == ConsumedAmount::Measure(Quantity::new(Decimal::new(450, 0), Unit::Gram)))
    );
    assert_eq!(
        after.occasion.attendance_of(taylor),
        MealAttendance::Elsewhere
    );
}

#[tokio::test]
async fn someone_marked_elsewhere_can_be_seated_again_with_a_note() {
    let h = harness();
    let taylor = h.add_member("Taylor");
    let food = product("Curry", 150);
    h.products.seed(food.clone());
    let household = household_dinner(&h, food.id, 600).await;

    h.service
        .set_attendance(
            household.entry.occasion_id,
            taylor,
            MealAttendance::Elsewhere,
            h.actor_id,
        )
        .await
        .unwrap();

    let seated = h
        .service
        .set_attendance(
            household.entry.occasion_id,
            taylor,
            MealAttendance::Eating {
                group_id: household.entry.id,
                note: Some("mild".to_owned()),
            },
            h.actor_id,
        )
        .await
        .unwrap();

    assert!(seated.absent_member_ids.is_empty());
    let diner = seated.groups[0]
        .diners
        .iter()
        .find(|diner| diner.member_id == taylor)
        .expect("Taylor is back at the table");
    assert_eq!(diner.note.as_deref(), Some("mild"));

    let renamed = h
        .service
        .set_attendance(
            household.entry.occasion_id,
            taylor,
            MealAttendance::Eating {
                group_id: household.entry.id,
                note: Some("extra hot".to_owned()),
            },
            h.actor_id,
        )
        .await
        .unwrap();
    let diner = renamed.groups[0]
        .diners
        .iter()
        .find(|diner| diner.member_id == taylor)
        .unwrap();
    assert_eq!(diner.note.as_deref(), Some("extra hot"));
    assert_eq!(renamed.groups[0].diners.len(), 2);
}

#[tokio::test]
async fn leaving_a_future_meal_is_allowed() {
    let h = harness();
    let food = product("Pie", 150);
    h.products.seed(food.clone());
    let household = plan(
        &h,
        date!(2026 - 09 - 20),
        Some(time!(18:30)),
        MealSlot::Dinner,
        everyone(vec![measured(food.id, 900)]),
    )
    .await;

    let after = h
        .service
        .set_attendance(
            household.entry.occasion_id,
            h.member_id,
            MealAttendance::Elsewhere,
            h.actor_id,
        )
        .await
        .unwrap();
    assert_eq!(after.absent_member_ids, vec![h.member_id]);
    assert!(after.groups[0].diners.is_empty());
}

#[tokio::test]
async fn a_second_group_takes_its_members_out_of_the_everyone_group() {
    let h = harness();
    let morgan = h.add_member("Morgan");
    let taylor = h.add_member("Taylor");
    let food = product("Chilli", 150);
    let lasagne = product("Lasagne", 150);
    h.products.seed(food.clone());
    h.products.seed(lasagne.clone());
    let household = household_dinner(&h, food.id, 900).await;

    let with_lasagne = h
        .service
        .add_group(
            household.entry.occasion_id,
            only(&[taylor], vec![measured(lasagne.id, 300)]),
            h.actor_id,
        )
        .await
        .unwrap();

    assert_eq!(with_lasagne.groups.len(), 2);
    let mut seated = diners_of(&with_lasagne, 0);
    seated.sort();
    let mut expected = vec![h.member_id, morgan];
    expected.sort();
    assert_eq!(seated, expected);
    assert_eq!(diners_of(&with_lasagne, 1), vec![taylor]);
    assert!(with_lasagne.unaccounted_member_ids.is_empty());
    assert_eq!(
        with_lasagne.occasion.attendance_of(taylor),
        MealAttendance::Eating {
            group_id: with_lasagne.groups[1].entry.entry.id,
            note: None,
        }
    );

    let second_everyone = h
        .service
        .add_group(
            household.entry.occasion_id,
            everyone(vec![measured(lasagne.id, 100)]),
            h.actor_id,
        )
        .await;
    assert!(matches!(second_everyone, Err(CoreError::Conflict { .. })));
}

#[tokio::test]
async fn attendance_cannot_change_once_the_portion_is_resolved() {
    let h = harness();
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

    let err = h
        .service
        .set_attendance(
            household.entry.occasion_id,
            h.member_id,
            MealAttendance::Elsewhere,
            h.actor_id,
        )
        .await;
    assert!(matches!(err, Err(CoreError::Conflict { .. })));
}

#[tokio::test]
async fn naming_someone_in_a_group_moves_them_there_and_clears_their_absence() {
    let h = harness();
    let taylor = h.add_member("Taylor");
    let food = product("Chilli", 150);
    let lasagne = product("Lasagne", 150);
    h.products.seed(food.clone());
    h.products.seed(lasagne.clone());
    let household = household_dinner(&h, food.id, 900).await;
    h.service
        .set_attendance(
            household.entry.occasion_id,
            taylor,
            MealAttendance::Elsewhere,
            h.actor_id,
        )
        .await
        .unwrap();

    let with_leftovers = h
        .service
        .add_group(
            household.entry.occasion_id,
            only(&[taylor], vec![measured(lasagne.id, 300)]),
            h.actor_id,
        )
        .await
        .unwrap();
    assert!(with_leftovers.absent_member_ids.is_empty());
    assert_eq!(diners_of(&with_leftovers, 0), vec![h.member_id]);
    assert_eq!(diners_of(&with_leftovers, 1), vec![taylor]);

    let curry = with_leftovers.groups[0].entry.entry.clone();
    let moved_back = h
        .service
        .update_group(
            curry.id,
            curry.revision,
            MealGroupPatch {
                everyone: Some(false),
                participants: Some(vec![
                    NewMealParticipant::member(h.member_id),
                    NewMealParticipant::member(taylor),
                ]),
                ..Default::default()
            },
            h.actor_id,
        )
        .await
        .unwrap();
    let mut seated = diners_of(&moved_back, 0);
    seated.sort();
    let mut expected = vec![h.member_id, taylor];
    expected.sort();
    assert_eq!(seated, expected);
}

#[tokio::test]
async fn a_resolved_participant_cannot_be_removed_even_if_it_would_orphan_the_group() {
    let h = harness();
    let taylor = h.add_member("Taylor");
    let lasagne = product("Lasagne", 150);
    h.products.seed(lasagne.clone());
    let leftovers = plan(
        &h,
        date!(2026 - 08 - 25),
        Some(time!(18:30)),
        MealSlot::Dinner,
        only(&[taylor], vec![measured(lasagne.id, 300)]),
    )
    .await;
    let component = leftovers.components[0].component.clone();
    h.service
        .mark_component_eaten_backdated(
            leftovers.entry.id,
            component.id,
            component.revision,
            ConfirmMealPlanComponent {
                consumed_on: date!(2026 - 08 - 25),
                consumed_at: None,
                amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(300, 0), Unit::Gram)),
                actor_id: h.actor_id,
                subject_member_id: Some(taylor),
            },
        )
        .await
        .unwrap();
    let fresh = h.service.get(leftovers.entry.id).await.unwrap();
    let err = h
        .service
        .update_group(
            fresh.entry.id,
            fresh.entry.revision,
            MealGroupPatch {
                participants: Some(vec![]),
                ..Default::default()
            },
            h.actor_id,
        )
        .await;
    assert!(matches!(err, Err(CoreError::Conflict { .. })));
}

#[tokio::test]
async fn releasing_the_last_member_removes_the_group() {
    let h = harness();
    let taylor = h.add_member("Taylor");
    let food = product("Chilli", 150);
    let lasagne = product("Lasagne", 150);
    h.products.seed(food.clone());
    h.products.seed(lasagne.clone());
    let household = household_dinner(&h, food.id, 900).await;
    let with_leftovers = h
        .service
        .add_group(
            household.entry.occasion_id,
            only(&[taylor], vec![measured(lasagne.id, 300)]),
            h.actor_id,
        )
        .await
        .unwrap();
    assert_eq!(with_leftovers.groups.len(), 2);
    assert_eq!(diners_of(&with_leftovers, 1), vec![taylor]);

    let after = h
        .service
        .set_attendance(
            household.entry.occasion_id,
            taylor,
            MealAttendance::Elsewhere,
            h.actor_id,
        )
        .await
        .unwrap();

    assert_eq!(
        after.groups.len(),
        1,
        "the leftovers group had nobody left on it, so it should be gone rather than left empty"
    );
    assert_eq!(after.groups[0].entry.entry.id, household.entry.id);
    assert_eq!(after.absent_member_ids, vec![taylor]);
}

#[tokio::test]
async fn emptying_the_occasions_only_group_keeps_it_rather_than_vanishing_under_the_caller() {
    let h = harness();
    let food = product("Soup", 150);
    h.products.seed(food.clone());
    let dinner = household_dinner(&h, food.id, 300).await;

    let after = h
        .service
        .set_attendance(
            dinner.entry.occasion_id,
            h.member_id,
            MealAttendance::Elsewhere,
            h.actor_id,
        )
        .await
        .unwrap();

    assert_eq!(
        after.groups.len(),
        1,
        "removing the occasion's only group is left to the explicit delete action, not this implicit cleanup"
    );
    assert!(diners_of(&after, 0).is_empty());
    assert!(
        h.service
            .get_occasion(dinner.entry.occasion_id)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn a_guests_only_group_survives_when_its_last_member_leaves() {
    let h = harness();
    let taylor = h.add_member("Taylor");
    let lasagne = product("Lasagne", 150);
    h.products.seed(lasagne.clone());
    let dinner = plan(
        &h,
        date!(2026 - 08 - 25),
        Some(time!(18:30)),
        MealSlot::Dinner,
        NewMealGroup {
            guest_groups: vec![NewMealGuestGroup::of(2)],
            ..only(&[taylor], vec![measured(lasagne.id, 300)])
        },
    )
    .await;
    assert_eq!(dinner.entry.guest_count(), 2);

    let after = h
        .service
        .set_attendance(
            dinner.entry.occasion_id,
            taylor,
            MealAttendance::Elsewhere,
            h.actor_id,
        )
        .await
        .unwrap();

    assert_eq!(after.groups.len(), 1);
    assert_eq!(after.groups[0].guest_count, 2);
    assert!(diners_of(&after, 0).is_empty());
}

#[tokio::test]
async fn deleting_the_last_group_removes_the_occasion() {
    let h = harness();
    let taylor = h.add_member("Taylor");
    let food = product("Chilli", 150);
    h.products.seed(food.clone());
    let household = household_dinner(&h, food.id, 900).await;
    let extra = h
        .service
        .add_group(
            household.entry.occasion_id,
            only(&[taylor], vec![measured(food.id, 100)]),
            h.actor_id,
        )
        .await
        .unwrap();
    let extra_group = extra.groups[1].entry.entry.clone();

    let remaining = h
        .service
        .delete_group(extra_group.id, extra_group.revision, h.actor_id)
        .await
        .unwrap()
        .expect("the everyone group is still there");
    assert_eq!(remaining.groups.len(), 1);
    assert!(
        remaining.groups[0]
            .diners
            .iter()
            .any(|diner| diner.member_id == taylor),
        "Taylor falls back into the everyone group"
    );

    let last = remaining.groups[0].entry.entry.clone();
    let gone = h
        .service
        .delete_group(last.id, last.revision, h.actor_id)
        .await
        .unwrap();
    assert!(gone.is_none());
    assert_eq!(h.plans.occasion_count(), 0);
}

#[tokio::test]
async fn moving_an_occasion_needs_an_empty_cell() {
    let h = harness();
    let food = product("Soup", 150);
    h.products.seed(food.clone());
    let dinner = planned(&h, vec![measured(food.id, 300)]).await;
    let lunch = plan(
        &h,
        date!(2026 - 08 - 26),
        None,
        MealSlot::Lunch,
        everyone(vec![measured(food.id, 200)]),
    )
    .await;
    let occasion = occasion_of(&h, &dinner).await;

    let clash = h
        .service
        .move_occasion(
            occasion.occasion.id,
            occasion.occasion.revision,
            date!(2026 - 08 - 26),
            MealSlot::Lunch,
            h.actor_id,
        )
        .await;
    assert!(matches!(clash, Err(CoreError::Conflict { .. })));

    let moved = h
        .service
        .move_occasion(
            occasion.occasion.id,
            occasion.occasion.revision,
            date!(2026 - 08 - 27),
            MealSlot::Lunch,
            h.actor_id,
        )
        .await
        .unwrap();
    assert_eq!(moved.occasion.planned_on, date!(2026 - 08 - 27));
    assert_eq!(moved.occasion.slot, MealSlot::Lunch);
    assert_eq!(
        moved.groups[0].entry.entry.planned_on,
        date!(2026 - 08 - 27)
    );
    assert_eq!(moved.groups[0].entry.entry.slot, MealSlot::Lunch);
    assert!(
        h.service
            .get_occasion(lunch.entry.occasion_id)
            .await
            .is_ok()
    );

    let week = h.service.planner_week(date!(2026 - 08 - 24)).await.unwrap();
    assert!(week.days[1].occasions[2].is_none());
    assert!(week.days[3].occasions[1].is_some());
}

#[tokio::test]
async fn copying_an_occasion_resets_guests_and_the_cooking_override() {
    let h = harness();
    let taylor = h.add_member("Taylor");
    let food = product("Roast", 150);
    h.products.seed(food.clone());
    let source = plan(
        &h,
        date!(2026 - 08 - 25),
        Some(time!(17:30)),
        MealSlot::Dinner,
        NewMealGroup {
            guest_groups: vec![NewMealGuestGroup::of(2)],
            cooking_servings: Some(8),
            participants: vec![NewMealParticipant {
                note: Some("mild".to_owned()),
                ..NewMealParticipant::member(taylor)
            }],
            ..everyone(vec![measured(food.id, 900)])
        },
    )
    .await;
    assert_eq!(source.entry.guest_count(), 2);
    assert_eq!(source.entry.serves(), 4);
    assert_eq!(source.entry.effective_cooking_servings(), 8);

    let copy = h
        .service
        .copy_occasion(
            source.entry.occasion_id,
            date!(2026 - 08 - 27),
            MealSlot::Dinner,
            h.actor_id,
        )
        .await
        .unwrap();

    assert_eq!(copy.occasion.planned_time, Some(time!(17:30)));
    let group = &copy.groups[0];
    assert!(group.entry.entry.everyone);
    assert_eq!(group.guest_count, 0);
    assert_eq!(group.cooking_servings, None);
    assert_eq!(group.serves, 2);
    assert_eq!(group.effective_cooking_servings, 2);
    let note = group
        .diners
        .iter()
        .find(|diner| diner.member_id == taylor)
        .and_then(|diner| diner.note.clone());
    assert_eq!(note.as_deref(), Some("mild"));
    assert_ne!(group.entry.entry.id, source.entry.id);
}

#[tokio::test]
async fn copying_a_week_fills_only_the_empty_cells() {
    let h = harness();
    let food = product("Soup", 150);
    h.products.seed(food.clone());
    planned(&h, vec![measured(food.id, 300)]).await;
    plan(
        &h,
        date!(2026 - 08 - 26),
        None,
        MealSlot::Lunch,
        everyone(vec![measured(food.id, 200)]),
    )
    .await;
    let already = plan(
        &h,
        date!(2026 - 09 - 01),
        None,
        MealSlot::Dinner,
        everyone(vec![measured(food.id, 50)]),
    )
    .await;

    let week = h
        .service
        .copy_week(date!(2026 - 08 - 31), date!(2026 - 08 - 24), h.actor_id)
        .await
        .unwrap();

    let tuesday_dinner = week.days[1].occasions[2].as_ref().unwrap();
    assert_eq!(tuesday_dinner.occasion.id, already.entry.occasion_id);
    let wednesday_lunch = week.days[2].occasions[1].as_ref().unwrap();
    assert_eq!(wednesday_lunch.occasion.planned_on, date!(2026 - 09 - 02));
    assert_eq!(h.plans.occasion_count(), 4);
}

#[tokio::test]
async fn a_meal_can_be_just_a_name_or_an_ad_hoc_kind() {
    let h = harness();
    let pizza = plan(
        &h,
        date!(2026 - 08 - 28),
        None,
        MealSlot::Dinner,
        NewMealGroup {
            label: Some("Pizza".to_owned()),
            ..everyone(Vec::new())
        },
    )
    .await;
    assert_eq!(pizza.name(), "Pizza");
    assert!(pizza.components.is_empty());
    assert_eq!(pizza.entry.serves(), 1);

    let out = plan(
        &h,
        date!(2026 - 08 - 29),
        None,
        MealSlot::Dinner,
        NewMealGroup {
            ad_hoc: Some(AdHocKind::EatingOut),
            ..everyone(Vec::new())
        },
    )
    .await;
    assert_eq!(out.name(), "Eating out");
    assert!(!out.entry.is_cooked());

    let nameless = try_plan(
        &h,
        date!(2026 - 08 - 30),
        None,
        MealSlot::Dinner,
        everyone(Vec::new()),
    )
    .await;
    assert!(matches!(nameless, Err(CoreError::Validation(_))));

    let food = product("Chips", 150);
    h.products.seed(food.clone());
    let takeaway_with_food = try_plan(
        &h,
        date!(2026 - 08 - 30),
        None,
        MealSlot::Dinner,
        NewMealGroup {
            ad_hoc: Some(AdHocKind::Takeaway),
            ..everyone(vec![measured(food.id, 100)])
        },
    )
    .await;
    assert!(matches!(takeaway_with_food, Err(CoreError::Validation(_))));
}

#[tokio::test]
async fn a_recipe_forecast_follows_the_cooking_servings() {
    let h = harness();
    let sam = h.add_member("Sam");
    let rice = product("Rice", 100);
    h.products.seed(rice.clone());
    let curry = seed_recipe(&h, "Curry", 4, vec![recipe_line(rice.id, 400)]).await;

    let entry = planned(&h, vec![servings_of(curry.id, 4)]).await;
    assert_eq!(
        entry.components[0].component.amount,
        ConsumedAmount::Servings(Decimal::new(2, 0)),
        "two diners, so the forecast is two servings however many the recipe makes"
    );

    let updated = h
        .service
        .update_group(
            entry.entry.id,
            entry.entry.revision,
            MealGroupPatch {
                cooking_servings: Some(Some(6)),
                ..Default::default()
            },
            h.actor_id,
        )
        .await
        .unwrap();
    assert_eq!(updated.groups[0].effective_cooking_servings, 6);
    assert_eq!(
        updated.groups[0].entry.components[0].component.amount,
        ConsumedAmount::Servings(Decimal::new(6, 0))
    );
    assert_eq!(
        updated.groups[0].entry.components[0]
            .preparation
            .unallocated,
        Some(ConsumedAmount::Servings(Decimal::new(4, 0)))
    );

    h.service
        .set_attendance(
            entry.entry.occasion_id,
            sam,
            MealAttendance::Elsewhere,
            h.actor_id,
        )
        .await
        .unwrap();
    let cleared = h
        .service
        .update_group(
            entry.entry.id,
            updated.groups[0].entry.entry.revision,
            MealGroupPatch {
                cooking_servings: Some(None),
                ..Default::default()
            },
            h.actor_id,
        )
        .await
        .unwrap();
    assert_eq!(cleared.groups[0].serves, 1);
    assert_eq!(
        cleared.groups[0].entry.components[0].component.amount,
        ConsumedAmount::Servings(Decimal::ONE)
    );
}

#[tokio::test]
async fn the_food_log_reports_where_a_member_is_for_each_occasion() {
    let h = harness();
    let taylor = h.add_member("Taylor");
    let food = product("Chilli", 150);
    h.products.seed(food.clone());
    let household = household_dinner(&h, food.id, 900).await;
    h.service
        .set_attendance(
            household.entry.occasion_id,
            taylor,
            MealAttendance::Elsewhere,
            h.actor_id,
        )
        .await
        .unwrap();

    let mine = h
        .service
        .week(h.member_id, date!(2026 - 08 - 24))
        .await
        .unwrap();
    let dinner = mine.days[1]
        .slots
        .iter()
        .find(|slot| slot.slot == MealSlot::Dinner)
        .unwrap();
    assert_eq!(dinner.occasion_id, Some(household.entry.occasion_id));
    assert!(matches!(
        dinner.attendance,
        Some(MealAttendance::Eating { .. })
    ));
    assert_eq!(dinner.group_name.as_deref(), Some("Chilli"));
    assert_eq!(dinner.items.len(), 1);

    let taylors = h.service.week(taylor, date!(2026 - 08 - 24)).await.unwrap();
    let dinner = taylors.days[1]
        .slots
        .iter()
        .find(|slot| slot.slot == MealSlot::Dinner)
        .unwrap();
    assert_eq!(dinner.attendance, Some(MealAttendance::Elsewhere));
    assert!(dinner.items.is_empty());
    assert_eq!(taylors.remaining_planned.nutrition.energy_kcal, None);
}

#[tokio::test]
async fn one_member_resolving_does_not_freeze_the_meal_for_a_manager() {
    let h = harness();
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

    let updated = update_components(
        &h,
        &current,
        vec![
            NewMealPlanComponent {
                id: Some(component.id),
                item: component.item,
                amount: component.amount,
            },
            measured(extra.id, 120),
        ],
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
        .create_occasion_backdated(occasion_input(h, on, at, slot, everyone(components)))
        .await
        .map(last_group)
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

    let updated = update_components(
        &h,
        &entry,
        vec![measured(food.id, 100), measured(extra.id, 20)],
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

    let entry = household_planned(
        &h,
        vec![servings_of(curry.id, 5)],
        &[h.member_id, other],
        Some(5),
    )
    .await;

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
    let entry = plan(
        &h,
        date!(2026 - 08 - 25),
        Some(time!(18:30)),
        MealSlot::Dinner,
        NewMealGroup {
            cooking_servings: Some(2),
            ..everyone(vec![servings_of(curry.id, 2)])
        },
    )
    .await;
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
    let entry = household_planned(
        &h,
        vec![servings_of(curry.id, 6)],
        &[h.member_id, sam, ash],
        Some(6),
    )
    .await;
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
    let entry = household_planned(
        &h,
        vec![servings_of(curry.id, 4)],
        &[h.member_id, other],
        Some(4),
    )
    .await;
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
