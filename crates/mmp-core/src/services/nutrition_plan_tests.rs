use std::str::FromStr;
use std::sync::Arc;

use super::*;
use crate::domain::{TargetSource, WeightSource};
use crate::ports::FixedClock;
use crate::testing::{
    InMemoryCalorieCalculationRepository, InMemoryHouseholdSettingsRepository,
    InMemoryMemberBodyProfileRepository, InMemoryNutritionTargetRepository,
    InMemoryWeightGoalRepository, InMemoryWeightRecordRepository,
};
use time::macros::{date, datetime};

struct Harness {
    service: NutritionPlanService,
    profiles: InMemoryMemberBodyProfileRepository,
    calculations: InMemoryCalorieCalculationRepository,
    targets: InMemoryNutritionTargetRepository,
    records: InMemoryWeightRecordRepository,
    goals: InMemoryWeightGoalRepository,
}

fn harness() -> Harness {
    let profiles = InMemoryMemberBodyProfileRepository::new();
    let calculations = InMemoryCalorieCalculationRepository::new();
    let targets = InMemoryNutritionTargetRepository::new();
    let records = InMemoryWeightRecordRepository::new();
    let goals = InMemoryWeightGoalRepository::new();
    let settings = Arc::new(InMemoryHouseholdSettingsRepository::new());
    let clock: Arc<dyn Clock> = Arc::new(FixedClock::new(datetime!(2026-09-12 09:00 UTC)));
    let target_service = NutritionTargetService::new(Arc::new(targets.clone()), clock.clone());
    let weight_service = WeightService::new(
        Arc::new(records.clone()),
        Arc::new(goals.clone()),
        settings.clone(),
        clock.clone(),
    );
    let service = NutritionPlanService::new(
        Arc::new(profiles.clone()),
        Arc::new(calculations.clone()),
        target_service,
        weight_service,
        settings,
        clock,
    );
    Harness {
        service,
        profiles,
        calculations,
        targets,
        records,
        goals,
    }
}

fn answers() -> NutritionPlanAnswers {
    NutritionPlanAnswers {
        member_id: HouseholdMemberId::seeded("guided"),
        date_of_birth: date!(1986 - 01 - 01),
        sex: Sex::Male,
        height_cm: Decimal::from(180),
        current_weight: Quantity::new(Decimal::from(80), Unit::Kilogram),
        habitual_activity: HabitualActivity::LightlyActive,
        objective: WeightObjective::Lose,
        target_weight: Some(Quantity::new(Decimal::from(75), Unit::Kilogram)),
        pace: Some(Pace::Standard),
        recorded_by: None,
    }
}

#[tokio::test]
async fn preview_calculates_without_writing() {
    let h = harness();

    let preview = h.service.preview(answers()).await.unwrap();

    assert_eq!(preview.recommended_kcal, Decimal::from(2130));
    assert_eq!(h.profiles.count(), 0);
    assert_eq!(h.calculations.count(), 0);
    assert_eq!(h.targets.count(), 0);
    assert_eq!(h.records.count(), 0);
    assert_eq!(h.goals.count(), 0);
}

#[tokio::test]
async fn guided_setup_writes_the_profile_weight_goal_target_and_provenance() {
    let h = harness();

    let plan = h.service.set_guided(answers()).await.unwrap();

    assert_eq!(plan.target.source, TargetSource::Calculated);
    assert_eq!(plan.target.effective_from, date!(2026 - 09 - 12));
    assert_eq!(plan.calculation.nutrition_target_id, plan.target.id);
    assert_eq!(plan.goal.planned_rate_kg_per_week, Some(Decimal::new(5, 1)));
    assert!(plan.weight_record.is_some());
    assert_eq!(h.profiles.count(), 1);
    assert_eq!(h.calculations.count(), 1);
    assert_eq!(h.targets.count(), 1);
    assert_eq!(h.records.count(), 1);
    assert_eq!(h.goals.count(), 1);
}

#[tokio::test]
async fn rerunning_on_the_same_day_updates_in_place() {
    let h = harness();
    let first = h.service.set_guided(answers()).await.unwrap();
    let mut changed = answers();
    changed.height_cm = Decimal::from(181);

    let second = h.service.set_guided(changed).await.unwrap();

    assert_eq!(second.target.id, first.target.id);
    assert_eq!(second.target.revision, first.target.revision.next());
    assert_eq!(second.calculation.id, first.calculation.id);
    assert_eq!(
        second.calculation.revision,
        first.calculation.revision.next()
    );
    assert_eq!(second.profile.revision, first.profile.revision.next());
    assert!(second.weight_record.is_none());
    assert_eq!(h.records.count(), 1);
}

#[tokio::test]
async fn a_changed_current_weight_becomes_a_new_weigh_in() {
    let h = harness();
    h.service.set_guided(answers()).await.unwrap();
    let mut changed = answers();
    changed.current_weight = Quantity::new(Decimal::from_str("79.5").unwrap(), Unit::Kilogram);
    changed.target_weight = Some(Quantity::new(Decimal::from(74), Unit::Kilogram));

    let plan = h.service.set_guided(changed).await.unwrap();

    assert_eq!(plan.weight_record.unwrap().source, WeightSource::Manual);
    assert_eq!(h.records.count(), 2);
}

#[tokio::test]
async fn maintenance_writes_no_goal_weight_or_rate() {
    let h = harness();
    let mut maintain = answers();
    maintain.objective = WeightObjective::Maintain;
    maintain.target_weight = None;
    maintain.pace = None;

    let plan = h.service.set_guided(maintain).await.unwrap();

    assert_eq!(
        plan.target.goals.energy_kcal,
        Some(plan.calculation.maintenance_kcal)
    );
    assert!(plan.goal.target_weight_kg.is_none());
    assert!(plan.goal.planned_rate_kg_per_week.is_none());
}

#[tokio::test]
async fn a_manual_target_replaces_calculated_provenance_for_today() {
    let h = harness();
    let guided = h.service.set_guided(answers()).await.unwrap();

    let manual = h
        .service
        .set_manual(answers().member_id, Decimal::from(900))
        .await
        .unwrap();

    assert_eq!(manual.id, guided.target.id);
    assert_eq!(manual.source, TargetSource::UserDefined);
    assert_eq!(manual.goals.energy_kcal, Some(Decimal::from(900)));
    assert_eq!(h.calculations.count(), 0);
}

#[tokio::test]
async fn changing_calories_preserves_other_targets_for_today() {
    let h = harness();
    h.targets.seed(NutritionTarget {
        id: NutritionTargetId::seeded("existing-target"),
        member_id: answers().member_id,
        effective_from: date!(2026 - 09 - 12),
        source: TargetSource::UserDefined,
        goals: NutritionGoals {
            energy_kcal: Some(Decimal::from(2000)),
            protein_g: Some(Decimal::from(120)),
            ..Default::default()
        },
        revision: Revision::INITIAL,
        created_at: datetime!(2026-09-01 09:00 UTC),
        updated_at: datetime!(2026-09-01 09:00 UTC),
    });

    let target = h
        .service
        .set_manual(answers().member_id, Decimal::from(1800))
        .await
        .unwrap();

    assert_eq!(target.goals.energy_kcal, Some(Decimal::from(1800)));
    assert_eq!(target.goals.protein_g, Some(Decimal::from(120)));
}
