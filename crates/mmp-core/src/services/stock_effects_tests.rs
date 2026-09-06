use rust_decimal::Decimal;
use time::macros::{date, datetime};

use super::{
    StockAffected, component_release, name_outcomes, product_deduction, record_deduction,
    record_release, requirement_deduction,
};
use crate::domain::{
    Confidence, ConsumedAmount, ConsumptionRecord, ConsumptionRecordId, DeductionCandidates,
    DeductionTarget, DemandSubject, HouseholdMemberId, Ingredient, IngredientId, MealItemRef,
    MealPlanComponentId, MealSlot, NutritionFacts, NutritionQuality, Product, ProductId,
    Provenance, Quantity, Revision, Shortfall, StockEffectSource, StockOutcome, Unit, UserId,
};
use crate::testing::{
    InMemoryIngredientRepository, InMemoryPreparedBatchRepository, InMemoryProductRepository,
};

fn d(value: i64) -> Decimal {
    Decimal::from(value)
}

fn grams(value: i64) -> Quantity {
    Quantity::new(d(value), Unit::Gram)
}

fn product(name: &str) -> Product {
    Product {
        id: ProductId::new(),
        name: name.to_owned(),
        brand: None,
        barcode: None,
        retailer: None,
        shopping_section: None,
        track_stock: None,
        package_quantity: None,
        servings_per_pack: None,
        mapped_ingredient_id: None,
        nutrition: NutritionFacts::default(),
        provenance: Provenance::local(),
        revision: Revision::INITIAL,
        created_at: datetime!(2026-09-04 09:00 UTC),
        updated_at: datetime!(2026-09-04 09:00 UTC),
        archived_at: None,
    }
}

fn ingredient(name: &str) -> Ingredient {
    Ingredient {
        id: IngredientId::new(),
        name: name.to_owned(),
        default_unit: Unit::Gram,
        shopping_section: None,
        track_stock: None,
        provenance: Provenance::local(),
        revision: Revision::INITIAL,
        created_at: datetime!(2026-09-04 09:00 UTC),
        updated_at: datetime!(2026-09-04 09:00 UTC),
        archived_at: None,
    }
}

fn record(
    product_id: ProductId,
    member: HouseholdMemberId,
    by: Option<UserId>,
) -> ConsumptionRecord {
    ConsumptionRecord {
        id: ConsumptionRecordId::new(),
        member_id: member,
        item: MealItemRef::product(product_id),
        recorded_by: by,
        meal_plan_entry_id: None,
        meal_plan_component_id: None,
        slot: MealSlot::Lunch,
        amount: ConsumedAmount::Measure(grams(150)),
        consumed_on: date!(2026 - 09 - 04),
        consumed_at: None,
        nutrition: NutritionFacts::default(),
        quality: NutritionQuality::Known,
        revision: Revision::INITIAL,
        created_at: datetime!(2026-09-04 09:00 UTC),
        updated_at: datetime!(2026-09-04 09:00 UTC),
    }
}

fn outcome(subject: DemandSubject) -> StockOutcome {
    StockOutcome {
        subject,
        wanted: grams(100),
        deducted: grams(100),
        shortfall: Shortfall::Covered,
        unresolved_release: false,
    }
}

#[test]
fn a_product_deduction_targets_only_that_product_and_carries_no_detail() {
    let milk = product("Milk");
    let source = ConsumptionRecordId::new().as_uuid();

    let deduction = product_deduction(
        StockEffectSource::ConsumptionRecord,
        source,
        &milk,
        &ConsumedAmount::Measure(grams(150)),
        "Lunch".to_owned(),
        None,
        None,
    )
    .expect("a measured amount always resolves");

    assert_eq!(deduction.source_id, source);
    assert_eq!(deduction.source_detail_id, None);
    assert_eq!(deduction.target, DeductionTarget::product(milk.id));
    assert_eq!(deduction.want, grams(150));
}

#[test]
fn a_product_deduction_is_dropped_when_the_amount_cannot_be_resolved() {
    let milk = product("Milk");

    let deduction = product_deduction(
        StockEffectSource::ConsumptionRecord,
        ConsumptionRecordId::new().as_uuid(),
        &milk,
        &ConsumedAmount::Packs(d(2)),
        "Lunch".to_owned(),
        None,
        None,
    );

    assert!(deduction.is_none());
}

#[test]
fn a_requirement_deduction_keeps_the_line_it_came_from() {
    let ingredient_id = IngredientId::new();
    let first = ProductId::new();
    let second = ProductId::new();
    let source = ConsumptionRecordId::new().as_uuid();
    let detail = MealPlanComponentId::new().as_uuid();

    let deduction = requirement_deduction(
        StockEffectSource::MealPlanComponent,
        source,
        detail,
        DeductionTarget::pool(ingredient_id, vec![first, second]),
        grams(400),
        "Porridge".to_owned(),
        None,
        None,
    );

    assert_eq!(deduction.source_id, source);
    assert_eq!(deduction.source_detail_id, Some(detail));
    assert_eq!(
        deduction.target.candidates,
        DeductionCandidates::Products(vec![first, second])
    );
}

#[test]
fn a_record_deduction_is_attributed_to_the_member_it_was_recorded_for() {
    let milk = product("Milk");
    let member = HouseholdMemberId::new();
    let actor = UserId::new();
    let consumed = record(milk.id, member, Some(actor));

    let deduction = record_deduction(&consumed, &milk, "Lunch".to_owned())
        .expect("a measured amount always resolves");

    assert_eq!(deduction.source_kind, StockEffectSource::ConsumptionRecord);
    assert_eq!(deduction.source_id, consumed.id.as_uuid());
    assert_eq!(deduction.subject_member_id, Some(member));
    assert_eq!(deduction.actor_user_id, Some(actor));
    assert_eq!(deduction.want, grams(150));
}

#[test]
fn a_record_release_matches_the_deduction_it_reverses() {
    let milk = product("Milk");
    let member = HouseholdMemberId::new();
    let actor = UserId::new();
    let consumed = record(milk.id, member, Some(actor));

    let deduction = record_deduction(&consumed, &milk, "Lunch".to_owned()).unwrap();
    let release = record_release(&consumed, "Lunch".to_owned());

    assert_eq!(release.source_kind, deduction.source_kind);
    assert_eq!(release.source_id, deduction.source_id);
    assert_eq!(release.subject_member_id, deduction.subject_member_id);
    assert_eq!(release.actor_user_id, deduction.actor_user_id);
}

#[test]
fn a_component_release_is_keyed_to_the_meal_plan_component() {
    let component = MealPlanComponentId::new().as_uuid();

    let release = component_release(component, None, "Dinner".to_owned(), None, None);

    assert_eq!(release.source_kind, StockEffectSource::MealPlanComponent);
    assert_eq!(release.source_id, component);
}

#[tokio::test]
async fn naming_nothing_asks_the_repositories_nothing() {
    let products = InMemoryProductRepository::new();
    let ingredients = InMemoryIngredientRepository::new();
    let batches = InMemoryPreparedBatchRepository::new();

    let named = name_outcomes(&products, &ingredients, &batches, Vec::new())
        .await
        .unwrap();

    assert!(named.is_empty());
}

#[tokio::test]
async fn outcomes_are_named_from_whichever_side_the_subject_came_from() {
    let products = InMemoryProductRepository::new();
    let ingredients = InMemoryIngredientRepository::new();
    let batches = InMemoryPreparedBatchRepository::new();
    let milk = product("Sample Whole Milk");
    let oats = ingredient("Oats");
    products.seed(milk.clone());
    ingredients.seed(oats.clone());

    let named = name_outcomes(
        &products,
        &ingredients,
        &batches,
        vec![
            outcome(DemandSubject::product(milk.id)),
            outcome(DemandSubject::ingredient(oats.id)),
        ],
    )
    .await
    .unwrap();

    assert_eq!(named[0].name, "Sample Whole Milk");
    assert_eq!(named[1].name, "Oats");
}

#[tokio::test]
async fn a_subject_we_cannot_name_is_said_to_be_unknown_rather_than_dropped() {
    let products = InMemoryProductRepository::new();
    let ingredients = InMemoryIngredientRepository::new();
    let batches = InMemoryPreparedBatchRepository::new();

    let named = name_outcomes(
        &products,
        &ingredients,
        &batches,
        vec![
            outcome(DemandSubject::product(ProductId::new())),
            outcome(DemandSubject::ingredient(IngredientId::new())),
        ],
    )
    .await
    .unwrap();

    assert_eq!(named.len(), 2);
    assert_eq!(named[0].name, "Unknown product");
    assert_eq!(named[1].name, "Unknown ingredient");
}

#[tokio::test]
async fn naming_preserves_the_figures_and_the_order_it_was_given() {
    let products = InMemoryProductRepository::new();
    let ingredients = InMemoryIngredientRepository::new();
    let batches = InMemoryPreparedBatchRepository::new();
    let milk = product("Milk");
    products.seed(milk.clone());

    let short = StockOutcome {
        subject: DemandSubject::product(milk.id),
        wanted: grams(400),
        deducted: grams(250),
        shortfall: Shortfall::Short {
            amount: grams(150),
            confidence: Confidence::Exact,
        },
        unresolved_release: true,
    };

    let named = name_outcomes(
        &products,
        &ingredients,
        &batches,
        vec![outcome(DemandSubject::product(milk.id)), short],
    )
    .await
    .unwrap();

    assert_eq!(named.len(), 2);
    assert_eq!(named[1].wanted, short.wanted);
    assert_eq!(named[1].deducted, short.deducted);
    assert_eq!(named[1].shortfall, short.shortfall);
    assert!(named[1].unresolved_release);
}

#[test]
fn a_bare_stock_affected_value_reports_nothing_moved() {
    let affected = StockAffected::bare(42_i32);

    assert!(affected.stock.is_empty());
    assert_eq!(*affected, 42);
    assert_eq!(affected.into_value(), 42);
}
