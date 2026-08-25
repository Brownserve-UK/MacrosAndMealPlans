use std::sync::Arc;

use super::*;
use crate::domain::{NutritionGoals, NutritionGoalsPatch, Patch};
use crate::error::CoreError;
use crate::ports::FixedClock;
use crate::testing::InMemoryNutritionTargetRepository;
use rust_decimal::Decimal;
use time::macros::{date, datetime};

struct Harness {
    service: NutritionTargetService,
    targets: InMemoryNutritionTargetRepository,
}

fn harness() -> Harness {
    let targets = InMemoryNutritionTargetRepository::new();
    let service = NutritionTargetService::new(
        Arc::new(targets.clone()),
        Arc::new(FixedClock::new(datetime!(2026-08-25 09:00 UTC))),
    );
    Harness { service, targets }
}

fn member() -> HouseholdMemberId {
    HouseholdMemberId::seeded("planner")
}

fn goals_kcal(value: i64) -> NutritionGoals {
    NutritionGoals {
        energy_kcal: Some(Decimal::new(value, 0)),
        ..Default::default()
    }
}

async fn create_at(h: &Harness, effective: time::Date, goals: NutritionGoals) -> NutritionTarget {
    h.service
        .create(NewNutritionTarget {
            member_id: member(),
            effective_from: effective,
            goals,
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn create_persists_a_target() {
    let h = harness();
    let created = create_at(&h, date!(2026 - 08 - 25), goals_kcal(2000)).await;
    assert_eq!(created.revision, Revision::INITIAL);
    assert_eq!(h.targets.count(), 1);
    let stored = h.service.get(created.id).await.unwrap();
    assert_eq!(stored.goals.energy_kcal, Some(Decimal::new(2000, 0)));
}

#[tokio::test]
async fn create_rejects_an_empty_target() {
    let h = harness();
    let error = h
        .service
        .create(NewNutritionTarget {
            member_id: member(),
            effective_from: date!(2026 - 08 - 25),
            goals: NutritionGoals::default(),
        })
        .await
        .unwrap_err();
    assert!(matches!(error, CoreError::Validation(_)));
}

#[tokio::test]
async fn create_rejects_a_negative_goal() {
    let h = harness();
    let goals = NutritionGoals {
        protein_g: Some(Decimal::new(-5, 0)),
        ..Default::default()
    };
    let error = h
        .service
        .create(NewNutritionTarget {
            member_id: member(),
            effective_from: date!(2026 - 08 - 25),
            goals,
        })
        .await
        .unwrap_err();
    assert!(matches!(error, CoreError::Validation(_)));
}

#[tokio::test]
async fn a_duplicate_effective_date_is_rejected() {
    let h = harness();
    create_at(&h, date!(2026 - 08 - 25), goals_kcal(2000)).await;
    let error = h
        .service
        .create(NewNutritionTarget {
            member_id: member(),
            effective_from: date!(2026 - 08 - 25),
            goals: goals_kcal(1800),
        })
        .await
        .unwrap_err();
    assert!(matches!(error, CoreError::Duplicate { .. }));
}

#[tokio::test]
async fn list_orders_by_effective_date() {
    let h = harness();
    create_at(&h, date!(2026 - 06 - 01), goals_kcal(2200)).await;
    create_at(&h, date!(2026 - 01 - 01), goals_kcal(2400)).await;
    let listed = h.service.list(member()).await.unwrap();
    let dates: Vec<_> = listed.iter().map(|t| t.effective_from).collect();
    assert_eq!(dates, vec![date!(2026 - 01 - 01), date!(2026 - 06 - 01)]);
}

#[tokio::test]
async fn update_sets_clears_and_preserves_fields() {
    let h = harness();
    let created = create_at(
        &h,
        date!(2026 - 08 - 25),
        NutritionGoals {
            energy_kcal: Some(Decimal::new(2000, 0)),
            protein_g: Some(Decimal::new(120, 0)),
            fat_g: Some(Decimal::new(70, 0)),
            ..Default::default()
        },
    )
    .await;

    let patch = NutritionTargetPatch {
        effective_from: None,
        goals: NutritionGoalsPatch {
            energy_kcal: Patch::Set(Decimal::new(1800, 0)),
            protein_g: Patch::Clear,
            ..Default::default()
        },
    };
    let updated = h
        .service
        .update(created.id, created.revision, patch)
        .await
        .unwrap();
    assert_eq!(updated.goals.energy_kcal, Some(Decimal::new(1800, 0)));
    assert_eq!(updated.goals.protein_g, None);
    assert_eq!(updated.goals.fat_g, Some(Decimal::new(70, 0)));
    assert_eq!(updated.revision, Revision::INITIAL.next());
}

#[tokio::test]
async fn update_rejects_clearing_the_last_goal() {
    let h = harness();
    let created = create_at(&h, date!(2026 - 08 - 25), goals_kcal(2000)).await;
    let patch = NutritionTargetPatch {
        effective_from: None,
        goals: NutritionGoalsPatch {
            energy_kcal: Patch::Clear,
            ..Default::default()
        },
    };
    let error = h
        .service
        .update(created.id, created.revision, patch)
        .await
        .unwrap_err();
    assert!(matches!(error, CoreError::Validation(_)));
}

#[tokio::test]
async fn update_with_a_stale_revision_conflicts() {
    let h = harness();
    let created = create_at(&h, date!(2026 - 08 - 25), goals_kcal(2000)).await;
    let stale = Revision::new(created.revision.get() + 5);
    let patch = NutritionTargetPatch {
        effective_from: None,
        goals: NutritionGoalsPatch {
            energy_kcal: Patch::Set(Decimal::new(1900, 0)),
            ..Default::default()
        },
    };
    let error = h
        .service
        .update(created.id, stale, patch)
        .await
        .unwrap_err();
    assert!(matches!(error, CoreError::RevisionMismatch { .. }));
}

#[tokio::test]
async fn delete_removes_a_target() {
    let h = harness();
    let created = create_at(&h, date!(2026 - 08 - 25), goals_kcal(2000)).await;
    h.service
        .delete(created.id, created.revision)
        .await
        .unwrap();
    assert_eq!(h.targets.count(), 0);
}

#[tokio::test]
async fn delete_with_a_stale_revision_conflicts() {
    let h = harness();
    let created = create_at(&h, date!(2026 - 08 - 25), goals_kcal(2000)).await;
    let stale = Revision::new(created.revision.get() + 1);
    let error = h.service.delete(created.id, stale).await.unwrap_err();
    assert!(matches!(error, CoreError::RevisionMismatch { .. }));
}
