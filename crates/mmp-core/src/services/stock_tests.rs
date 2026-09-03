use std::sync::Arc;

use rust_decimal::Decimal;
use time::OffsetDateTime;
use time::macros::{date, datetime};

use super::*;
use crate::domain::{
    Availability, Confidence, ConsumedAmount, DemandGap, HouseholdMember, HouseholdMemberId,
    MealItemRef, MealPlanComponent, MealPlanEntry, MealSlot, MissingStockInterpretation,
    NewStockItem, Product, ProductId, Provenance, Quantity, Revision, StockLevel, StorageLocation,
    Unit, UserId,
};
use crate::ports::{FixedClock, MealPlanRepository, StockQuery};
use crate::testing::{
    InMemoryHouseholdMemberRepository, InMemoryHouseholdSettingsRepository,
    InMemoryIngredientRepository, InMemoryMealPlanRepository, InMemoryProductRepository,
    InMemoryRecipeRepository, InMemoryStockRepository,
};

struct Harness {
    service: StockService,
    stock: InMemoryStockRepository,
    products: InMemoryProductRepository,
    ingredients: InMemoryIngredientRepository,
    recipes: InMemoryRecipeRepository,
    meal_plans: InMemoryMealPlanRepository,
    settings: InMemoryHouseholdSettingsRepository,
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
        Arc::new(ingredients.clone()),
        Arc::new(meal_plans.clone()),
        Arc::new(recipes.clone()),
        Arc::new(members),
        Arc::new(settings.clone()),
        Arc::new(FixedClock::new(datetime!(2026-08-24 09:00 UTC))),
    );
    Harness {
        service,
        stock,
        products,
        ingredients,
        recipes,
        meal_plans,
        settings,
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
        track_stock: None,
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
    plan_measured_on(h, product_id, g, date!(2026 - 08 - 25)).await
}

async fn plan_measured_on(h: &Harness, product_id: ProductId, g: i64, on: time::Date) {
    let now = OffsetDateTime::UNIX_EPOCH;
    let entry = MealPlanEntry {
        id: crate::domain::MealPlanEntryId::new(),
        scope: crate::domain::MealPlanScope::Member,
        member_id: Some(h.member_id),
        planned_on: on,
        planned_time: None,
        slot: MealSlot::Dinner,
        portioning: crate::domain::Portioning::Equal,
        components: vec![MealPlanComponent {
            id: crate::domain::MealPlanComponentId::new(),
            item: MealItemRef::product(product_id),
            amount: ConsumedAmount::Measure(grams(g)),
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

    match &result.products[0].availability {
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
                    quantity: Quantity::new(Decimal::new(200, 0), Unit::Gram),
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

    match &result.products[0].availability {
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

    assert_eq!(
        result.products[0].availability,
        Availability::AssumedAvailable
    );
    assert!(!result.products[0].availability.is_short());
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

    assert_eq!(result.products[0].availability, Availability::Unknown);
}

#[tokio::test]
async fn a_planned_recipe_we_cannot_load_leaves_demand_incomplete() {
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
        portioning: crate::domain::Portioning::Equal,
        components: vec![MealPlanComponent {
            id: crate::domain::MealPlanComponentId::new(),
            item: MealItemRef::recipe(crate::domain::RecipeId::new()),
            amount: ConsumedAmount::Servings(Decimal::new(1, 0)),
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

    let result = h
        .service
        .availability(&[p.id], date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    // The recipe id points at nothing, so there is no ingredient to blame: the gap has to travel
    // at the report level rather than on a row.
    assert_eq!(result.demand_gaps, vec![DemandGap::RecipeMissing]);
    assert!(result.products[0].demand_gaps.is_empty());
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

#[tokio::test]
async fn a_planned_meal_keeps_holding_stock_after_its_time_has_passed() {
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
    plan_measured_on(&h, p.id, 250, date!(2026 - 08 - 20)).await;

    let result = h
        .service
        .availability(&[p.id], date!(2026 - 08 - 18), date!(2026 - 08 - 31))
        .await
        .unwrap();

    match &result.products[0].availability {
        Availability::Quantified {
            planned_demand,
            unallocated,
            ..
        } => {
            assert_eq!(*planned_demand, grams(250));
            assert_eq!(*unallocated, grams(750));
        }
        other => panic!("expected a quantified availability, got {other:?}"),
    }
}

fn mapped(name: &str, ingredient_id: crate::domain::IngredientId) -> Product {
    let mut p = product();
    p.id = ProductId::new();
    p.name = name.to_owned();
    p.mapped_ingredient_id = Some(ingredient_id);
    p
}

fn seed_ingredient(h: &Harness, id: crate::domain::IngredientId, name: &str) {
    let now = OffsetDateTime::UNIX_EPOCH;
    h.ingredients.seed(crate::domain::Ingredient {
        id,
        name: name.to_owned(),
        default_unit: Unit::Gram,
        shopping_section: None,
        track_stock: None,
        provenance: Provenance::local(),
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    });
}

fn ingredient_line(
    ingredient_id: crate::domain::IngredientId,
    g: i64,
) -> crate::domain::RecipeComponent {
    crate::domain::RecipeComponent {
        id: crate::domain::RecipeComponentId::new(),
        requirement: crate::domain::RecipeRequirement::Ingredient { ingredient_id },
        source_text: None,
        amount: ConsumedAmount::Measure(grams(g)),
        position: 0,
    }
}

fn seed_recipe(
    h: &Harness,
    servings: i32,
    lines: Vec<crate::domain::RecipeComponent>,
) -> crate::domain::Recipe {
    let now = OffsetDateTime::UNIX_EPOCH;
    let recipe = crate::domain::Recipe {
        id: crate::domain::RecipeId::new(),
        name: "Curry".to_owned(),
        description: None,
        notes: None,
        preparation_minutes: None,
        cooking_minutes: None,
        servings,
        components: lines,
        instructions: Vec::new(),
        meal_categories: Vec::new(),
        country_categories: Vec::new(),
        tags: Vec::new(),
        photo_version: None,
        owner_id: h.actor_id,
        visibility: crate::domain::RecipeVisibility::Private,
        created_by: h.actor_id,
        updated_by: h.actor_id,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    };
    h.recipes.seed(recipe.clone());
    recipe
}

async fn plan_servings(h: &Harness, recipe_id: crate::domain::RecipeId, servings: i64) {
    let now = OffsetDateTime::UNIX_EPOCH;
    let entry = MealPlanEntry {
        id: crate::domain::MealPlanEntryId::new(),
        scope: crate::domain::MealPlanScope::Member,
        member_id: Some(h.member_id),
        planned_on: date!(2026 - 08 - 25),
        planned_time: None,
        slot: MealSlot::Dinner,
        portioning: crate::domain::Portioning::Equal,
        components: vec![MealPlanComponent {
            id: crate::domain::MealPlanComponentId::new(),
            item: MealItemRef::recipe(recipe_id),
            amount: ConsumedAmount::Servings(Decimal::new(servings, 0)),
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

#[tokio::test]
async fn a_planned_recipe_ingredient_counts_demand_across_its_whole_product_pool() {
    let h = harness();
    let rice = crate::domain::IngredientId::new();
    let a = mapped("Tesco Basmati", rice);
    let b = mapped("Sainsbury's Basmati", rice);
    h.products.seed(a.clone());
    h.products.seed(b.clone());
    for p in [&a, &b] {
        h.service
            .create(
                new_item(
                    p.id,
                    StockLevel::Exact {
                        quantity: grams(300),
                    },
                ),
                h.actor_id,
                None,
            )
            .await
            .unwrap();
    }

    // 400 g over 4 servings, two servings planned, so the pool owes 200 g.
    let curry = seed_recipe(&h, 4, vec![ingredient_line(rice, 400)]);
    plan_servings(&h, curry.id, 2).await;

    let report = h
        .service
        .availability_overview(date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    assert!(report.demand_gaps.is_empty());
    let ingredient = report
        .ingredients
        .iter()
        .find(|row| row.ingredient_id == rice)
        .expect("the pooled ingredient is reported");
    let Availability::Quantified {
        on_hand,
        planned_demand,
        unallocated,
        ..
    } = ingredient.availability
    else {
        panic!(
            "expected a quantified pool, got {:?}",
            ingredient.availability
        );
    };
    assert_eq!(on_hand, grams(600));
    assert_eq!(planned_demand, grams(200));
    assert_eq!(unallocated, grams(400));

    // And the product rows carry the same 200 g between them rather than double-counting it.
    let per_product: Decimal = report
        .products
        .iter()
        .filter_map(|row| match row.availability {
            Availability::Quantified { planned_demand, .. } => Some(planned_demand.amount),
            _ => None,
        })
        .sum();
    assert_eq!(per_product, Decimal::new(200, 0));
}

#[tokio::test]
async fn an_ingredient_with_no_mapped_products_reports_a_gap_rather_than_being_satisfied() {
    let h = harness();
    let rice = crate::domain::IngredientId::new();
    let curry = seed_recipe(&h, 1, vec![ingredient_line(rice, 100)]);
    plan_servings(&h, curry.id, 1).await;

    let report = h
        .service
        .availability_overview(date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    let ingredient = report
        .ingredients
        .iter()
        .find(|row| row.ingredient_id == rice)
        .expect("the unmappable ingredient still gets a row");
    assert_eq!(
        ingredient.demand_gaps,
        vec![DemandGap::IngredientHasNoProducts]
    );
    assert!(ingredient.demand_incomplete());
}

#[tokio::test]
async fn an_ingredient_we_hold_no_stock_of_still_shows_its_demand() {
    let h = harness();
    let rice = crate::domain::IngredientId::new();
    let a = mapped("Tesco Basmati", rice);
    h.products.seed(a.clone());
    // Deliberately no stock item: the product rows cannot express this, only the ingredient row can.

    let curry = seed_recipe(&h, 1, vec![ingredient_line(rice, 250)]);
    plan_servings(&h, curry.id, 1).await;

    let report = h
        .service
        .availability_overview(date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    assert!(report.products.is_empty());
    let ingredient = report
        .ingredients
        .iter()
        .find(|row| row.ingredient_id == rice)
        .expect("demand with no stock is still reported");
    assert_eq!(ingredient.availability, Availability::Unknown);
    assert!(ingredient.demand_gaps.is_empty());
}

#[tokio::test]
async fn an_ingredient_we_hold_with_nothing_planned_is_still_reported() {
    let h = harness();
    let milk = crate::domain::IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Tesco Whole Milk", milk);
    let b = mapped("Value Whole Milk", milk);
    h.products.seed(a.clone());
    h.products.seed(b.clone());
    for (p, amount) in [(&a, 150), (&b, 600)] {
        h.service
            .create(
                new_item(
                    p.id,
                    StockLevel::Exact {
                        quantity: grams(amount),
                    },
                ),
                h.actor_id,
                None,
            )
            .await
            .unwrap();
    }

    let report = h
        .service
        .availability_overview(date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    let ingredient = report
        .ingredients
        .iter()
        .find(|row| row.ingredient_id == milk)
        .expect("an ingredient we hold is reported even with no demand");
    assert_eq!(ingredient.name, "Whole Milk");
    let Availability::Quantified {
        on_hand,
        planned_demand,
        unallocated,
        ..
    } = ingredient.availability
    else {
        panic!(
            "expected a quantified pool, got {:?}",
            ingredient.availability
        );
    };
    assert_eq!(on_hand, grams(750));
    assert_eq!(planned_demand, grams(0));
    assert_eq!(unallocated, grams(750));
    assert!(ingredient.demand_gaps.is_empty());
}

#[tokio::test]
async fn a_pool_member_we_hold_no_stock_of_still_joins_the_pool() {
    let h = harness();
    let milk = crate::domain::IngredientId::new();
    let held = mapped("Tesco Whole Milk", milk);
    let empty = mapped("Value Whole Milk", milk);
    h.products.seed(held.clone());
    h.products.seed(empty.clone());
    h.service
        .create(
            new_item(
                held.id,
                StockLevel::Exact {
                    quantity: grams(150),
                },
            ),
            h.actor_id,
            None,
        )
        .await
        .unwrap();

    let report = h
        .service
        .availability_overview(date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    let ingredient = report
        .ingredients
        .iter()
        .find(|row| row.ingredient_id == milk)
        .expect("the pool is reported");
    match &ingredient.availability {
        Availability::Quantified { on_hand, .. } => assert_eq!(*on_hand, grams(150)),
        other => panic!("expected a quantified pool, got {other:?}"),
    }
}

#[tokio::test]
async fn stock_of_an_unmapped_product_produces_no_ingredient_row() {
    let h = harness();
    let p = product();
    h.products.seed(p.clone());
    h.service
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

    let report = h
        .service
        .availability_overview(date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    assert_eq!(report.products.len(), 1);
    assert!(report.ingredients.is_empty());
}

#[tokio::test]
async fn a_pools_demand_includes_meals_planned_directly_against_its_products() {
    let h = harness();
    let milk = crate::domain::IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Tesco Whole Milk", milk);
    let b = mapped("Value Whole Milk", milk);
    h.products.seed(a.clone());
    h.products.seed(b.clone());
    for p in [&a, &b] {
        h.service
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
    }

    // 200 g wanted through the recipe's generic ingredient line...
    let curry = seed_recipe(&h, 1, vec![ingredient_line(milk, 200)]);
    plan_servings(&h, curry.id, 1).await;
    // ...and 500 g planned straight onto one of the products.
    plan_measured(&h, a.id, 500).await;

    let report = h
        .service
        .availability_overview(date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    let ingredient = report
        .ingredients
        .iter()
        .find(|row| row.ingredient_id == milk)
        .expect("the pool is reported");
    let Availability::Quantified {
        on_hand,
        planned_demand,
        unallocated,
        ..
    } = ingredient.availability
    else {
        panic!(
            "expected a quantified pool, got {:?}",
            ingredient.availability
        );
    };
    assert_eq!(on_hand, grams(800));
    // The pool owes both, not just the 200 g asked for by name.
    assert_eq!(planned_demand, grams(700));
    assert_eq!(unallocated, grams(100));
}

fn planned_demand_for(report: &crate::domain::AvailabilityReport, product: ProductId) -> Quantity {
    let row = report
        .products
        .iter()
        .find(|row| row.product_id == product)
        .expect("the product is reported");
    match &row.availability {
        Availability::Quantified { planned_demand, .. } => *planned_demand,
        other => panic!("expected a quantified product, got {other:?}"),
    }
}

#[tokio::test]
async fn a_pool_share_skips_a_member_its_own_planned_meals_have_already_emptied() {
    let h = harness();
    let milk = crate::domain::IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Tesco Whole Milk", milk);
    let b = mapped("Value Whole Milk", milk);
    h.products.seed(a.clone());
    h.products.seed(b.clone());
    for p in [&a, &b] {
        h.service
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
    }

    plan_measured(&h, a.id, 500).await;
    let curry = seed_recipe(&h, 1, vec![ingredient_line(milk, 200)]);
    plan_servings(&h, curry.id, 1).await;

    let report = h
        .service
        .availability_overview(date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    assert_eq!(planned_demand_for(&report, a.id), grams(500));
    assert_eq!(planned_demand_for(&report, b.id), grams(200));
}

#[tokio::test]
async fn a_fully_pinned_pool_leaves_the_shared_want_on_the_ingredient_row() {
    let h = harness();
    let milk = crate::domain::IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Tesco Whole Milk", milk);
    let b = mapped("Value Whole Milk", milk);
    h.products.seed(a.clone());
    h.products.seed(b.clone());
    for p in [&a, &b] {
        h.service
            .create(
                new_item(
                    p.id,
                    StockLevel::Exact {
                        quantity: grams(100),
                    },
                ),
                h.actor_id,
                None,
            )
            .await
            .unwrap();
    }

    plan_measured(&h, a.id, 100).await;
    plan_measured(&h, b.id, 100).await;
    let curry = seed_recipe(&h, 1, vec![ingredient_line(milk, 300)]);
    plan_servings(&h, curry.id, 1).await;

    let report = h
        .service
        .availability_overview(date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    assert_eq!(planned_demand_for(&report, a.id), grams(100));
    assert_eq!(planned_demand_for(&report, b.id), grams(100));

    let ingredient = report
        .ingredients
        .iter()
        .find(|row| row.ingredient_id == milk)
        .expect("the pool is reported");
    let Availability::Quantified {
        on_hand,
        planned_demand,
        unallocated,
        ..
    } = ingredient.availability
    else {
        panic!("expected a quantified pool");
    };
    assert_eq!(on_hand, grams(200));
    assert_eq!(planned_demand, grams(500));
    assert_eq!(unallocated, grams(-300));
}

#[tokio::test]
async fn a_products_claims_name_both_the_meals_that_asked_for_it_and_the_ones_that_asked_for_its_ingredient()
 {
    let h = harness();
    let milk = crate::domain::IngredientId::new();
    seed_ingredient(&h, milk, "Whole Milk");
    let a = mapped("Tesco Whole Milk", milk);
    let b = mapped("Value Whole Milk", milk);
    h.products.seed(a.clone());
    h.products.seed(b.clone());
    for p in [&a, &b] {
        h.service
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
    }

    plan_measured(&h, a.id, 250).await;
    let curry = seed_recipe(&h, 1, vec![ingredient_line(milk, 200)]);
    plan_servings(&h, curry.id, 1).await;

    let report = h
        .service
        .availability(&[a.id], date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    let pinned: Vec<_> = report
        .claims
        .iter()
        .filter(|claim| claim.subject == DemandSubject::product(a.id))
        .collect();
    assert_eq!(pinned.len(), 1);
    assert_eq!(pinned[0].quantity, grams(250));
    assert_eq!(pinned[0].recipe_name, None);

    let shared: Vec<_> = report
        .claims
        .iter()
        .filter(|claim| claim.subject == DemandSubject::ingredient(milk))
        .collect();
    assert_eq!(shared.len(), 1);
    assert_eq!(shared[0].quantity, grams(200));
    assert_eq!(shared[0].recipe_name.as_deref(), Some(curry.name.as_str()));

    let overview = h
        .service
        .availability_overview(date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();
    assert!(overview.claims.is_empty());
}

fn household_component(product_id: ProductId, g: i64) -> MealPlanComponent {
    MealPlanComponent {
        id: crate::domain::MealPlanComponentId::new(),
        item: MealItemRef::product(product_id),
        amount: ConsumedAmount::Measure(grams(g)),
        position: 0,
        snapshot: None,
        revision: Revision::INITIAL,
        display_order: uuid::Uuid::nil(),
    }
}

async fn plan_household_shared(
    h: &Harness,
    product_id: ProductId,
    total_g: i64,
    statuses: [crate::domain::ParticipantStatus; 2],
    on: time::Date,
) {
    let now = OffsetDateTime::UNIX_EPOCH;
    let component = household_component(product_id, total_g);
    let participants = statuses
        .into_iter()
        .map(|status| crate::domain::MealParticipant {
            id: crate::domain::MealParticipantId::new(),
            member_id: HouseholdMemberId::new(),
            allocations: vec![crate::domain::MealParticipantAllocation {
                id: crate::domain::MealParticipantAllocationId::new(),
                component_id: component.id,
                allocated: ConsumedAmount::Measure(grams(total_g / 2)),
                status,
                consumption_record_id: None,
                resolved_by: None,
                resolved_at: None,
            }],
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
        })
        .collect();

    let entry = MealPlanEntry {
        id: crate::domain::MealPlanEntryId::new(),
        scope: crate::domain::MealPlanScope::Household,
        member_id: None,
        planned_on: on,
        planned_time: None,
        slot: MealSlot::Dinner,
        portioning: crate::domain::Portioning::Equal,
        components: vec![component],
        participants,
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

#[tokio::test]
async fn a_partly_confirmed_household_meal_keeps_the_unconfirmed_share_as_demand() {
    use crate::domain::ParticipantStatus;
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
            Some(h.member_id),
        )
        .await
        .unwrap();

    plan_household_shared(
        &h,
        p.id,
        400,
        [ParticipantStatus::Eaten, ParticipantStatus::Planned],
        date!(2026 - 08 - 25),
    )
    .await;

    let report = h
        .service
        .availability(&[p.id], date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    assert_eq!(planned_demand_for(&report, p.id), grams(200));
}

#[tokio::test]
async fn a_household_meal_everyone_has_resolved_stops_counting_as_demand() {
    use crate::domain::ParticipantStatus;
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
            Some(h.member_id),
        )
        .await
        .unwrap();

    plan_household_shared(
        &h,
        p.id,
        400,
        [ParticipantStatus::Eaten, ParticipantStatus::NotEaten],
        date!(2026 - 08 - 25),
    )
    .await;

    let report = h
        .service
        .availability(&[p.id], date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    assert_eq!(planned_demand_for(&report, p.id), grams(0));
}

#[tokio::test]
async fn an_assumed_meal_keeps_its_full_hold_and_flags_its_claims() {
    let h = harness();
    h.settings.set_assume_eaten_when_time_passes(true);
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
            Some(h.member_id),
        )
        .await
        .unwrap();

    plan_measured_on(&h, p.id, 300, date!(2026 - 08 - 23)).await;
    plan_measured_on(&h, p.id, 200, date!(2026 - 08 - 25)).await;

    let report = h
        .service
        .availability(&[p.id], date!(2026 - 08 - 23), date!(2026 - 08 - 31))
        .await
        .unwrap();

    assert_eq!(planned_demand_for(&report, p.id), grams(500));

    let assumed: Vec<bool> = report.claims.iter().map(|claim| claim.assumed).collect();
    assert_eq!(assumed.len(), 2);
    assert!(assumed.contains(&true), "yesterday's meal is assumed");
    assert!(assumed.contains(&false), "tomorrow's meal is not");
}

#[tokio::test]
async fn an_untracked_ingredient_with_no_stock_is_assumed_available() {
    let h = harness();
    let paprika = crate::domain::IngredientId::new();
    seed_ingredient(&h, paprika, "Paprika");
    h.ingredients.set_track_stock(paprika, Some(false));
    h.products.seed(mapped("Paprika 50g", paprika));

    let curry = seed_recipe(&h, 1, vec![ingredient_line(paprika, 10)]);
    plan_servings(&h, curry.id, 1).await;

    let report = h
        .service
        .availability_overview(date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    let ingredient = report
        .ingredients
        .iter()
        .find(|row| row.ingredient_id == paprika)
        .expect("demand with no stock is still reported");
    assert_eq!(ingredient.availability, Availability::AssumedAvailable);
}

#[tokio::test]
async fn a_tracked_ingredient_with_no_stock_is_absent_rather_than_unknown() {
    let h = harness();
    let apples = crate::domain::IngredientId::new();
    seed_ingredient(&h, apples, "Apples");
    h.ingredients.set_track_stock(apples, Some(true));
    h.products.seed(mapped("Braeburn", apples));

    let crumble = seed_recipe(&h, 1, vec![ingredient_line(apples, 200)]);
    plan_servings(&h, crumble.id, 1).await;

    let report = h
        .service
        .availability_overview(date!(2026 - 08 - 24), date!(2026 - 08 - 31))
        .await
        .unwrap();

    let ingredient = report
        .ingredients
        .iter()
        .find(|row| row.ingredient_id == apples)
        .expect("demand with no stock is still reported");
    assert_eq!(ingredient.availability, Availability::Absent);
}

#[tokio::test]
async fn a_products_own_answer_beats_the_ingredients() {
    let h = harness();
    let oil = crate::domain::IngredientId::new();
    seed_ingredient(&h, oil, "Olive Oil");
    h.ingredients.set_track_stock(oil, Some(false));
    let mut p = mapped("Extra virgin 500ml", oil);
    p.track_stock = Some(true);
    h.products.seed(p.clone());
    plan_measured(&h, p.id, 30).await;

    let result = h
        .service
        .availability(&[p.id], date!(2026 - 08 - 18), date!(2026 - 08 - 31))
        .await
        .unwrap();

    assert_eq!(result.products[0].availability, Availability::Absent);
}

#[tokio::test]
async fn with_nobody_having_said_the_household_setting_still_decides() {
    let h = harness();
    let p = product();
    h.products.seed(p.clone());
    plan_measured(&h, p.id, 200).await;

    let result = h
        .service
        .availability(&[p.id], date!(2026 - 08 - 18), date!(2026 - 08 - 31))
        .await
        .unwrap();
    assert_eq!(result.products[0].availability, Availability::Unknown);

    h.settings
        .set_missing_stock_interpretation(MissingStockInterpretation::Absent);
    let result = h
        .service
        .availability(&[p.id], date!(2026 - 08 - 18), date!(2026 - 08 - 31))
        .await
        .unwrap();
    assert_eq!(result.products[0].availability, Availability::Absent);
}
