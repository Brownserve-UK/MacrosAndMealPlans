use std::sync::Arc;

use time::OffsetDateTime;
use time::macros::{date, datetime, time};

use super::*;
use crate::domain::{
    ConsumedAmount, MealItemRef, MealOccasionId, MealPlanEntry, MealSlot, ProductId, Quantity, Unit,
};
use crate::ports::{Clock, FixedClock};
use crate::testing::{InMemoryMealPlanRepository, InMemoryMealTemplateRepository};

struct Harness {
    service: MealTemplateService,
    plans: InMemoryMealPlanRepository,
    owner: UserId,
}

fn harness() -> Harness {
    let templates = InMemoryMealTemplateRepository::new();
    let consumption = crate::testing::InMemoryConsumptionRecordRepository::new();
    let plans = InMemoryMealPlanRepository::new(consumption);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock::new(datetime!(2026-09-08 09:00 UTC)));
    let service = MealTemplateService::new(Arc::new(templates), Arc::new(plans.clone()), clock);
    Harness {
        service,
        plans,
        owner: UserId::new(),
    }
}

fn measured(product_id: ProductId, grams: i64) -> NewMealTemplateComponent {
    NewMealTemplateComponent {
        item: MealItemRef::product(product_id),
        amount: ConsumedAmount::Measure(Quantity::new(
            rust_decimal::Decimal::new(grams, 0),
            Unit::Gram,
        )),
    }
}

#[tokio::test]
async fn creates_a_template_owned_by_the_actor() {
    let h = harness();
    let template = h
        .service
        .create(NewMealTemplate {
            id: None,
            owner_id: h.owner,
            name: "Fish fingers, chips and peas".to_owned(),
            components: vec![measured(ProductId::new(), 100)],
        })
        .await
        .unwrap();

    assert_eq!(template.owner_id, h.owner);
    assert_eq!(template.components.len(), 1);
    assert_eq!(template.components[0].position, 0);
}

#[tokio::test]
async fn another_user_cannot_read_a_private_template() {
    let h = harness();
    let template = h
        .service
        .create(NewMealTemplate {
            id: None,
            owner_id: h.owner,
            name: "Fish fingers".to_owned(),
            components: vec![measured(ProductId::new(), 100)],
        })
        .await
        .unwrap();

    let other = UserId::new();
    let result = h.service.get(template.id, other).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn updating_replaces_the_component_set() {
    let h = harness();
    let first = ProductId::new();
    let second = ProductId::new();
    let template = h
        .service
        .create(NewMealTemplate {
            id: None,
            owner_id: h.owner,
            name: "Fish fingers, chips and peas".to_owned(),
            components: vec![measured(first, 100), measured(second, 70)],
        })
        .await
        .unwrap();

    let updated = h
        .service
        .update(
            template.id,
            h.owner,
            template.revision,
            MealTemplatePatch {
                name: None,
                components: Some(vec![measured(first, 100)]),
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.components.len(), 1);
    assert_eq!(updated.revision, template.revision.next());
}

#[tokio::test]
async fn deleting_removes_the_template() {
    let h = harness();
    let template = h
        .service
        .create(NewMealTemplate {
            id: None,
            owner_id: h.owner,
            name: "Fish fingers".to_owned(),
            components: vec![measured(ProductId::new(), 100)],
        })
        .await
        .unwrap();

    h.service
        .delete(template.id, h.owner, template.revision)
        .await
        .unwrap();

    assert!(h.service.get(template.id, h.owner).await.is_err());
}

#[tokio::test]
async fn from_entry_keeps_foods_and_drops_cooked_food() {
    let h = harness();
    let fish_fingers = ProductId::new();
    let entry = MealPlanEntry {
        id: crate::domain::MealPlanEntryId::new(),
        occasion_id: MealOccasionId::new(),
        planned_on: date!(2026 - 09 - 08),
        planned_time: Some(time!(18:30)),
        slot: MealSlot::Dinner,
        label: None,
        ad_hoc: None,
        components: vec![
            crate::domain::MealPlanComponent {
                id: crate::domain::MealPlanComponentId::new(),
                item: MealItemRef::product(fish_fingers),
                amount: ConsumedAmount::Measure(Quantity::new(
                    rust_decimal::Decimal::new(4, 0),
                    Unit::Item,
                )),
                position: 0,
                snapshot: None,
                cooking_servings: None,
                revision: Revision::INITIAL,
                display_order: uuid::Uuid::now_v7(),
            },
            crate::domain::MealPlanComponent {
                id: crate::domain::MealPlanComponentId::new(),
                item: MealItemRef::dish(crate::domain::RecipeId::new()),
                amount: ConsumedAmount::Servings(rust_decimal::Decimal::ONE),
                position: 1,
                snapshot: None,
                cooking_servings: None,
                revision: Revision::INITIAL,
                display_order: uuid::Uuid::now_v7(),
            },
        ],
        everyone: true,
        participants: Vec::new(),
        guest_groups: Vec::new(),
        created_by: h.owner,
        updated_by: h.owner,
        revision: Revision::INITIAL,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    };
    h.plans.seed_entry(entry.clone());

    let template = h
        .service
        .from_entry(entry.id, h.owner, "Fish fingers dinner".to_owned())
        .await
        .unwrap();

    assert_eq!(template.components.len(), 1, "the dish is not saved");
    assert_eq!(
        template.components[0].item,
        MealItemRef::product(fish_fingers)
    );
}
