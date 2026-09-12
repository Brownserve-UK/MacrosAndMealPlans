use std::sync::Arc;

use super::*;
use crate::domain::{Patch, Quantity, Unit, WeightObjective, WeightSource};
use crate::error::CoreError;
use crate::ports::FixedClock;
use crate::testing::{
    InMemoryHouseholdSettingsRepository, InMemoryWeightGoalRepository,
    InMemoryWeightRecordRepository,
};
use std::str::FromStr;
use time::macros::{date, datetime};

struct Harness {
    service: WeightService,
    records: InMemoryWeightRecordRepository,
    goals: InMemoryWeightGoalRepository,
}

fn harness() -> Harness {
    let records = InMemoryWeightRecordRepository::new();
    let goals = InMemoryWeightGoalRepository::new();
    let service = WeightService::new(
        Arc::new(records.clone()),
        Arc::new(goals.clone()),
        Arc::new(InMemoryHouseholdSettingsRepository::new()),
        Arc::new(FixedClock::new(datetime!(2026-09-03 09:00 UTC))),
    );
    Harness {
        service,
        records,
        goals,
    }
}

fn member() -> HouseholdMemberId {
    HouseholdMemberId::seeded("weigher")
}

fn kg(amount: &str) -> Quantity {
    Quantity::new(Decimal::from_str(amount).unwrap(), Unit::Kilogram)
}

async fn weigh_in(h: &Harness, on: Date, weight: &str) -> WeightRecord {
    h.service
        .record(NewWeightRecord {
            member_id: member(),
            weight: kg(weight),
            recorded_on: on,
            recorded_at: None,
            source: WeightSource::Manual,
            recorded_by: None,
        })
        .await
        .unwrap()
}

async fn losing_goal(h: &Harness) -> WeightGoal {
    h.service
        .set_goal(NewWeightGoal {
            member_id: member(),
            objective: WeightObjective::Lose,
            starting_weight: kg("80"),
            target_weight: Some(kg("76")),
            planned_rate: Some(kg("0.5")),
            started_on: date!(2026 - 08 - 01),
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn recording_a_weight_stamps_the_id_revision_and_timestamps() {
    let h = harness();

    let record = weigh_in(&h, date!(2026 - 09 - 03), "80.4").await;

    assert_eq!(record.member_id, member());
    assert_eq!(record.weight_kg, Decimal::from_str("80.4").unwrap());
    assert_eq!(record.revision, Revision::INITIAL);
    assert_eq!(record.created_at, datetime!(2026-09-03 09:00 UTC));
    assert_eq!(h.records.count(), 1);
}

#[tokio::test]
async fn a_weight_recorded_in_pounds_lands_in_kilograms() {
    let h = harness();

    let record = h
        .service
        .record(NewWeightRecord {
            member_id: member(),
            weight: Quantity::new(Decimal::from(160), Unit::Pound),
            recorded_on: date!(2026 - 09 - 03),
            recorded_at: None,
            source: WeightSource::Manual,
            recorded_by: None,
        })
        .await
        .unwrap();

    assert_eq!(record.weight_kg, Decimal::from_str("72.575").unwrap());
}

#[tokio::test]
async fn recording_an_impossible_weight_is_rejected() {
    let h = harness();

    let error = h
        .service
        .record(NewWeightRecord {
            member_id: member(),
            weight: kg("0"),
            recorded_on: date!(2026 - 09 - 03),
            recorded_at: None,
            source: WeightSource::Manual,
            recorded_by: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Validation(_)));
    assert_eq!(h.records.count(), 0);
}

#[tokio::test]
async fn several_readings_on_one_day_are_all_kept() {
    let h = harness();
    let day = date!(2026 - 09 - 03);

    weigh_in(&h, day, "80.4").await;
    weigh_in(&h, day, "80.9").await;

    assert_eq!(h.records.count(), 2);
    assert_eq!(h.service.list_records(member()).await.unwrap().len(), 2);
}

#[tokio::test]
async fn updating_a_reading_bumps_the_revision() {
    let h = harness();
    let record = weigh_in(&h, date!(2026 - 09 - 03), "80.4").await;

    let updated = h
        .service
        .update_record(
            record.id,
            record.revision,
            WeightRecordPatch {
                weight: Some(kg("79.8")),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.weight_kg, Decimal::from_str("79.8").unwrap());
    assert_eq!(updated.revision, record.revision.next());
}

#[tokio::test]
async fn updating_a_reading_with_a_stale_revision_conflicts() {
    let h = harness();
    let record = weigh_in(&h, date!(2026 - 09 - 03), "80.4").await;

    let error = h
        .service
        .update_record(
            record.id,
            record.revision.next(),
            WeightRecordPatch {
                weight: Some(kg("79.8")),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::RevisionMismatch { .. }));
}

#[tokio::test]
async fn a_reading_can_have_its_time_cleared() {
    let h = harness();
    let record = h
        .service
        .record(NewWeightRecord {
            member_id: member(),
            weight: kg("80"),
            recorded_on: date!(2026 - 09 - 03),
            recorded_at: Some(datetime!(2026-09-03 07:30 UTC)),
            source: WeightSource::Manual,
            recorded_by: None,
        })
        .await
        .unwrap();

    let updated = h
        .service
        .update_record(
            record.id,
            record.revision,
            WeightRecordPatch {
                recorded_at: Patch::Clear,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert!(updated.recorded_at.is_none());
}

#[tokio::test]
async fn deleting_a_reading_that_is_not_there_is_not_found() {
    let h = harness();

    let error = h
        .service
        .delete_record(WeightRecordId::seeded("ghost"), Revision::INITIAL)
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::NotFound { .. }));
}

#[tokio::test]
async fn a_member_can_only_have_one_goal() {
    let h = harness();
    losing_goal(&h).await;

    let error = h
        .service
        .set_goal(NewWeightGoal {
            member_id: member(),
            objective: WeightObjective::Gain,
            starting_weight: kg("80"),
            target_weight: Some(kg("84")),
            planned_rate: Some(kg("0.25")),
            started_on: date!(2026 - 09 - 01),
        })
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Duplicate { .. }));
    assert_eq!(h.goals.count(), 1);
}

#[tokio::test]
async fn a_goal_is_revalidated_as_a_whole_when_patched() {
    let h = harness();
    let goal = losing_goal(&h).await;

    let error = h
        .service
        .update_goal(
            goal.id,
            goal.revision,
            WeightGoalPatch {
                target_weight: Patch::Set(kg("84")),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Validation(_)));
}

#[tokio::test]
async fn switching_to_maintenance_drops_the_target_and_rate() {
    let h = harness();
    let goal = losing_goal(&h).await;

    let updated = h
        .service
        .update_goal(
            goal.id,
            goal.revision,
            WeightGoalPatch {
                objective: Some(WeightObjective::Maintain),
                target_weight: Patch::Clear,
                planned_rate: Patch::Clear,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.objective, WeightObjective::Maintain);
    assert!(updated.target_weight_kg.is_none());
    assert!(updated.planned_rate_kg_per_week.is_none());
}

#[tokio::test]
async fn clearing_a_goal_leaves_the_readings_alone() {
    let h = harness();
    weigh_in(&h, date!(2026 - 09 - 03), "80").await;
    let goal = losing_goal(&h).await;

    h.service.clear_goal(goal.id, goal.revision).await.unwrap();

    assert_eq!(h.goals.count(), 0);
    assert_eq!(h.records.count(), 1);
}

#[tokio::test]
async fn a_summary_with_nothing_in_it_says_so() {
    let h = harness();

    let summary = h.service.summary(member()).await.unwrap();

    assert!(summary.latest.is_none());
    assert!(summary.goal.is_none());
    assert!(summary.projection.is_none());
    assert!(summary.change_since_start_kg.is_none());
    assert!(summary.series.is_empty());
}

#[tokio::test]
async fn a_summary_collapses_each_day_to_one_point() {
    let h = harness();
    weigh_in(&h, date!(2026 - 09 - 01), "80.5").await;
    weigh_in(&h, date!(2026 - 09 - 01), "80.2").await;
    weigh_in(&h, date!(2026 - 09 - 02), "80.0").await;

    let summary = h.service.summary(member()).await.unwrap();

    assert_eq!(summary.series.len(), 2);
    assert_eq!(summary.series[0].on, date!(2026 - 09 - 01));
    assert_eq!(
        summary.series[1].weight_kg,
        Decimal::from_str("80.0").unwrap()
    );
    assert_eq!(
        summary.latest.unwrap().weight_kg,
        Decimal::from_str("80.0").unwrap()
    );
}

#[tokio::test]
async fn a_summary_projects_against_the_latest_reading() {
    let h = harness();
    losing_goal(&h).await;
    weigh_in(&h, date!(2026 - 09 - 02), "78").await;

    let summary = h.service.summary(member()).await.unwrap();

    assert_eq!(
        summary.projection,
        Some(GoalProjection::Projected {
            on: date!(2026 - 10 - 01),
            remaining_kg: Decimal::from(2),
        })
    );
    assert_eq!(summary.change_since_start_kg, Some(Decimal::from(-2)));
}

#[tokio::test]
async fn a_goal_with_no_readings_still_projects() {
    let h = harness();
    losing_goal(&h).await;

    let summary = h.service.summary(member()).await.unwrap();

    assert!(summary.latest.is_none());
    assert!(matches!(
        summary.projection,
        Some(GoalProjection::Projected { .. })
    ));
    assert!(summary.change_since_start_kg.is_none());
}
