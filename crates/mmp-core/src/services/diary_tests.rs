use std::sync::Arc;

use super::*;
use crate::domain::{
    ConsumptionRecordId, HouseholdMemberId, MealItemRef, MealSlot, NutritionQuality, Provenance,
    Quantity, Unit,
};
use crate::ports::FixedClock;
use crate::testing::{
    InMemoryConsumptionRecordRepository, InMemoryProductRepository, InMemoryRecipeRepository,
};
use rust_decimal::Decimal;
use time::OffsetDateTime;
use time::macros::{date, datetime};

struct Harness {
    service: DiaryService,
    records: InMemoryConsumptionRecordRepository,
    products: InMemoryProductRepository,
    recipes: InMemoryRecipeRepository,
}

fn harness() -> Harness {
    harness_at(datetime!(2026-08-22 09:00 UTC))
}

fn harness_at(now: OffsetDateTime) -> Harness {
    let records = InMemoryConsumptionRecordRepository::new();
    let products = InMemoryProductRepository::new();
    let recipes = InMemoryRecipeRepository::new();
    let service = DiaryService::new(
        Arc::new(records.clone()),
        Arc::new(products.clone()),
        Arc::new(recipes.clone()),
        Arc::new(FixedClock::new(now)),
    );
    Harness {
        service,
        records,
        products,
        recipes,
    }
}

fn seed_product(h: &Harness, nutrition: NutritionFacts) -> Product {
    let now = OffsetDateTime::now_utc();
    let product = Product {
        id: ProductId::new(),
        name: "Whole Milk".to_owned(),
        brand: None,
        barcode: None,
        retailer: None,
        shopping_section: None,
        package_quantity: Some(Quantity::new(Decimal::new(650, 0), Unit::Gram)),
        servings_per_pack: None,
        mapped_ingredient_id: None,
        nutrition,
        provenance: Provenance::local(),
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    };
    h.products.seed(product.clone());
    product
}

fn known_nutrition() -> NutritionFacts {
    NutritionFacts {
        basis: Some(Quantity::new(Decimal::new(100, 0), Unit::Gram)),
        energy_kcal: Some(Decimal::new(200, 0)),
        protein_g: Some(Decimal::new(10, 0)),
        carbohydrate_g: Some(Decimal::new(20, 0)),
        sugar_g: Some(Decimal::new(5, 0)),
        fat_g: Some(Decimal::new(3, 0)),
        saturated_fat_g: Some(Decimal::new(1, 0)),
        fibre_g: Some(Decimal::new(2, 0)),
        salt_g: Some(Decimal::new(1, 1)),
        cholesterol_mg: Some(Decimal::new(0, 0)),
        extra: Default::default(),
    }
}

fn measure_150g(product_id: ProductId, member_id: HouseholdMemberId) -> NewConsumptionRecord {
    NewConsumptionRecord {
        id: None,
        member_id,
        item: MealItemRef::product(product_id),
        recorded_by: None,
        meal_plan_entry_id: None,
        meal_plan_component_id: None,
        slot: MealSlot::Breakfast,
        amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(150, 0), Unit::Gram)),
        consumed_on: date!(2026 - 08 - 22),
        consumed_at: None,
    }
}

#[tokio::test]
async fn records_a_consumption_at_the_initial_revision_with_scaled_nutrition() {
    let h = harness();
    let product = seed_product(&h, known_nutrition());
    let member = HouseholdMemberId::new();

    let recorded = h
        .service
        .record(measure_150g(product.id, member))
        .await
        .unwrap();

    assert_eq!(recorded.revision, Revision::INITIAL);
    assert_eq!(recorded.slot, MealSlot::Breakfast);
    assert_eq!(recorded.consumed_at, None);
    assert_eq!(recorded.nutrition.energy_kcal, Some(Decimal::new(300, 0)));
    assert_eq!(recorded.quality, NutritionQuality::Known);
    assert_eq!(h.records.count(), 1);
}

#[tokio::test]
async fn an_explicit_consumption_time_can_be_cleared() {
    let h = harness();
    let product = seed_product(&h, known_nutrition());
    let member = HouseholdMemberId::new();
    let mut input = measure_150g(product.id, member);
    input.consumed_at = Some(datetime!(2026-08-22 08:30 UTC));
    let recorded = h.service.record(input).await.unwrap();

    let amended = h
        .service
        .amend(
            recorded.id,
            recorded.revision,
            ConsumptionRecordPatch {
                consumed_at: Some(None),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(amended.consumed_at, None);
}

#[tokio::test]
async fn recording_against_an_archived_product_is_refused() {
    let h = harness();
    let mut product = seed_product(&h, known_nutrition());
    product.archived_at = Some(OffsetDateTime::now_utc());
    h.products.seed(product.clone());
    let member = HouseholdMemberId::new();

    let err = h
        .service
        .record(measure_150g(product.id, member))
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn recording_a_serving_against_a_product_with_no_serving_count_is_refused() {
    let h = harness();
    let product = seed_product(&h, known_nutrition());
    let member = HouseholdMemberId::new();

    let mut input = measure_150g(product.id, member);
    input.amount = ConsumedAmount::Servings(Decimal::ONE);

    let err = h.service.record(input).await.unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn amending_the_amount_recomputes_the_snapshot() {
    let h = harness();
    let product = seed_product(&h, known_nutrition());
    let member = HouseholdMemberId::new();
    let recorded = h
        .service
        .record(measure_150g(product.id, member))
        .await
        .unwrap();

    let patch = ConsumptionRecordPatch {
        amount: Some(ConsumedAmount::Measure(Quantity::new(
            Decimal::new(300, 0),
            Unit::Gram,
        ))),
        ..Default::default()
    };
    let amended = h
        .service
        .amend(recorded.id, recorded.revision, patch)
        .await
        .unwrap();

    assert_eq!(amended.nutrition.energy_kcal, Some(Decimal::new(600, 0)));
    assert_eq!(amended.revision, recorded.revision.next());
}

#[tokio::test]
async fn amending_only_the_day_does_not_recompute_nutrition() {
    let h = harness();
    let product = seed_product(&h, known_nutrition());
    let member = HouseholdMemberId::new();
    let recorded = h
        .service
        .record(measure_150g(product.id, member))
        .await
        .unwrap();

    let patch = ConsumptionRecordPatch {
        consumed_on: Some(date!(2026 - 08 - 21)),
        ..Default::default()
    };
    let amended = h
        .service
        .amend(recorded.id, recorded.revision, patch)
        .await
        .unwrap();

    assert_eq!(amended.nutrition, recorded.nutrition);
    assert_eq!(amended.consumed_on, date!(2026 - 08 - 21));
}

#[tokio::test]
async fn amending_the_meal_slot_moves_the_record() {
    let h = harness();
    let product = seed_product(&h, known_nutrition());
    let member = HouseholdMemberId::new();
    let recorded = h
        .service
        .record(measure_150g(product.id, member))
        .await
        .unwrap();

    let amended = h
        .service
        .amend(
            recorded.id,
            recorded.revision,
            ConsumptionRecordPatch {
                slot: Some(MealSlot::Lunch),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(amended.slot, MealSlot::Lunch);
    assert_eq!(amended.nutrition, recorded.nutrition);
}

#[tokio::test]
async fn amending_with_a_stale_revision_conflicts() {
    let h = harness();
    let product = seed_product(&h, known_nutrition());
    let member = HouseholdMemberId::new();
    let recorded = h
        .service
        .record(measure_150g(product.id, member))
        .await
        .unwrap();

    let err = h
        .service
        .amend(
            recorded.id,
            Revision::new(999),
            ConsumptionRecordPatch {
                consumed_on: Some(date!(2026 - 08 - 21)),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::RevisionMismatch { .. }));
}

#[tokio::test]
async fn removing_a_record_deletes_it() {
    let h = harness();
    let product = seed_product(&h, known_nutrition());
    let member = HouseholdMemberId::new();
    let recorded = h
        .service
        .record(measure_150g(product.id, member))
        .await
        .unwrap();

    h.service
        .remove(recorded.id, recorded.revision)
        .await
        .unwrap();
    assert_eq!(h.records.count(), 0);
}

#[tokio::test]
async fn removing_a_missing_record_is_not_found() {
    let h = harness();
    let err = h
        .service
        .remove(ConsumptionRecordId::new(), Revision::INITIAL)
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::NotFound { .. }));
}

#[tokio::test]
async fn removing_with_a_stale_revision_conflicts() {
    let h = harness();
    let product = seed_product(&h, known_nutrition());
    let member = HouseholdMemberId::new();
    let recorded = h
        .service
        .record(measure_150g(product.id, member))
        .await
        .unwrap();

    let err = h
        .service
        .remove(recorded.id, Revision::new(999))
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::RevisionMismatch { .. }));
}

#[tokio::test]
async fn day_totals_sum_nutrition_and_count_by_quality() {
    let h = harness();
    let known_product = seed_product(&h, known_nutrition());
    let mut partial_nutrition = known_nutrition();
    partial_nutrition.fibre_g = None;
    let partial_product = seed_product(&h, partial_nutrition);
    let unknown_product = seed_product(&h, NutritionFacts::default());
    let member = HouseholdMemberId::new();

    h.service
        .record(measure_150g(known_product.id, member))
        .await
        .unwrap();
    h.service
        .record(measure_150g(partial_product.id, member))
        .await
        .unwrap();
    h.service
        .record(measure_150g(unknown_product.id, member))
        .await
        .unwrap();

    let day = h.service.day(member, date!(2026 - 08 - 22)).await.unwrap();

    assert_eq!(day.totals.entry_count, 3);
    assert_eq!(day.totals.partial_count, 1);
    assert_eq!(day.totals.unknown_count, 1);
    assert_eq!(day.totals.nutrition.energy_kcal, Some(Decimal::new(600, 0)));
}

#[tokio::test]
async fn day_only_returns_entries_for_the_requested_member_and_date() {
    let h = harness();
    let product = seed_product(&h, known_nutrition());
    let member = HouseholdMemberId::new();
    let other_member = HouseholdMemberId::new();

    h.service
        .record(measure_150g(product.id, member))
        .await
        .unwrap();
    h.service
        .record(measure_150g(product.id, other_member))
        .await
        .unwrap();
    let mut yesterday = measure_150g(product.id, member);
    yesterday.consumed_on = date!(2026 - 08 - 21);
    h.service.record(yesterday).await.unwrap();

    let day = h.service.day(member, date!(2026 - 08 - 22)).await.unwrap();
    assert_eq!(day.entries.len(), 1);
}

#[tokio::test]
async fn recording_food_in_the_future_is_refused() {
    let h = harness();
    let product = seed_product(&h, known_nutrition());
    let member = HouseholdMemberId::new();

    let mut next_week = measure_150g(product.id, member);
    next_week.consumed_on = date!(2026 - 08 - 29);
    let err = h.service.record(next_week).await.unwrap_err();

    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn recording_food_a_day_ahead_is_allowed_for_timezone_slack() {
    let h = harness();
    let product = seed_product(&h, known_nutrition());
    let member = HouseholdMemberId::new();

    let mut tomorrow = measure_150g(product.id, member);
    tomorrow.consumed_on = date!(2026 - 08 - 23);
    h.service.record(tomorrow).await.unwrap();
}

#[tokio::test]
async fn amending_a_record_into_the_future_is_refused() {
    let h = harness();
    let product = seed_product(&h, known_nutrition());
    let member = HouseholdMemberId::new();
    let recorded = h
        .service
        .record(measure_150g(product.id, member))
        .await
        .unwrap();

    let patch = ConsumptionRecordPatch {
        consumed_on: Some(date!(2026 - 08 - 29)),
        ..Default::default()
    };
    let err = h
        .service
        .amend(recorded.id, recorded.revision, patch)
        .await
        .unwrap_err();

    assert!(matches!(err, CoreError::Validation(_)));
}

fn seed_recipe_from(h: &Harness, owner: crate::domain::UserId) -> crate::domain::Recipe {
    let now = OffsetDateTime::now_utc();
    let product = seed_product(h, known_nutrition());
    let line = crate::domain::RecipeComponent {
        id: crate::domain::RecipeComponentId::new(),
        requirement: crate::domain::RecipeRequirement::Product {
            product_id: product.id,
        },
        source_text: None,
        amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(200, 0), Unit::Gram)),
        position: 0,
    };
    let recipe = crate::domain::Recipe {
        id: crate::domain::RecipeId::new(),
        name: "Porridge".to_owned(),
        description: None,
        servings: 2,
        preparation_minutes: None,
        cooking_minutes: None,
        notes: None,
        components: vec![line],
        instructions: Vec::new(),
        meal_categories: Vec::new(),
        country_categories: Vec::new(),
        tags: Vec::new(),
        photo_version: None,
        owner_id: owner,
        visibility: crate::domain::RecipeVisibility::Private,
        created_by: owner,
        updated_by: owner,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    };
    h.recipes.seed(recipe.clone());
    recipe
}

#[tokio::test]
async fn logs_a_recipe_serving_with_derived_nutrition() {
    let h = harness();
    let actor = crate::domain::UserId::new();
    let recipe = seed_recipe_from(&h, actor);
    let member = HouseholdMemberId::new();

    let recorded = h
        .service
        .record(NewConsumptionRecord {
            id: None,
            member_id: member,
            item: MealItemRef::recipe(recipe.id),
            recorded_by: Some(actor),
            meal_plan_entry_id: None,
            meal_plan_component_id: None,
            slot: MealSlot::Breakfast,
            amount: ConsumedAmount::Servings(Decimal::ONE),
            consumed_on: date!(2026 - 08 - 22),
            consumed_at: None,
        })
        .await
        .unwrap();

    assert_eq!(recorded.item, MealItemRef::recipe(recipe.id));
    assert_eq!(recorded.nutrition.energy_kcal, Some(Decimal::new(200, 0)));

    let day = h.service.day(member, date!(2026 - 08 - 22)).await.unwrap();
    assert_eq!(day.entries[0].product_name, "Porridge");
}

#[tokio::test]
async fn logging_a_recipe_serving_measured_in_grams_is_refused() {
    let h = harness();
    let actor = crate::domain::UserId::new();
    let recipe = seed_recipe_from(&h, actor);

    let error = h
        .service
        .record(NewConsumptionRecord {
            id: None,
            member_id: HouseholdMemberId::new(),
            item: MealItemRef::recipe(recipe.id),
            recorded_by: Some(actor),
            meal_plan_entry_id: None,
            meal_plan_component_id: None,
            slot: MealSlot::Breakfast,
            amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(100, 0), Unit::Gram)),
            consumed_on: date!(2026 - 08 - 22),
            consumed_at: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Validation { .. }));
}

#[tokio::test]
async fn logging_a_recipe_owned_by_someone_else_is_refused() {
    let h = harness();
    let recipe = seed_recipe_from(&h, crate::domain::UserId::new());

    let error = h
        .service
        .record(NewConsumptionRecord {
            id: None,
            member_id: HouseholdMemberId::new(),
            item: MealItemRef::recipe(recipe.id),
            recorded_by: Some(crate::domain::UserId::new()),
            meal_plan_entry_id: None,
            meal_plan_component_id: None,
            slot: MealSlot::Breakfast,
            amount: ConsumedAmount::Servings(Decimal::ONE),
            consumed_on: date!(2026 - 08 - 22),
            consumed_at: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::NotFound { .. }));
}
