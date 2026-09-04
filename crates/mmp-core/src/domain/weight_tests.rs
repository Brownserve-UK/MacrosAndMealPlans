use super::*;
use time::macros::{date, datetime};

fn kg(amount: &str) -> Quantity {
    Quantity::new(Decimal::from_str(amount).unwrap(), Unit::Kilogram)
}

fn goal(
    objective: WeightObjective,
    starting: &str,
    target: Option<&str>,
    rate: Option<&str>,
) -> WeightGoal {
    let now = datetime!(2026-09-03 08:00 UTC);
    WeightGoal {
        id: WeightGoalId::seeded("goal"),
        member_id: HouseholdMemberId::seeded("member"),
        objective,
        starting_weight_kg: Decimal::from_str(starting).unwrap(),
        target_weight_kg: target.map(|value| Decimal::from_str(value).unwrap()),
        planned_rate_kg_per_week: rate.map(|value| Decimal::from_str(value).unwrap()),
        started_on: date!(2026 - 08 - 01),
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
    }
}

fn record(id: &str, on: Date, at: Option<OffsetDateTime>, weight: &str) -> WeightRecord {
    WeightRecord {
        id: WeightRecordId::seeded(id),
        member_id: HouseholdMemberId::seeded("member"),
        weight_kg: Decimal::from_str(weight).unwrap(),
        recorded_on: on,
        recorded_at: at,
        source: WeightSource::Manual,
        recorded_by: None,
        revision: Revision::INITIAL,
        created_at: datetime!(2026-09-03 08:00 UTC),
        updated_at: datetime!(2026-09-03 08:00 UTC),
    }
}

fn new_record(weight: Quantity) -> NewWeightRecord {
    NewWeightRecord {
        member_id: HouseholdMemberId::seeded("member"),
        weight,
        recorded_on: date!(2026 - 09 - 03),
        recorded_at: None,
        source: WeightSource::Manual,
        recorded_by: None,
    }
}

fn new_goal(
    objective: WeightObjective,
    starting: Quantity,
    target: Option<Quantity>,
    rate: Option<Quantity>,
) -> NewWeightGoal {
    NewWeightGoal {
        member_id: HouseholdMemberId::seeded("member"),
        objective,
        starting_weight: starting,
        target_weight: target,
        planned_rate: rate,
        started_on: date!(2026 - 08 - 01),
    }
}

#[test]
fn a_weight_in_pounds_is_stored_in_kilograms() {
    let pounds = Quantity::new(Decimal::from(160), Unit::Pound);
    let converted = new_record(pounds).weight_kg().unwrap();
    assert_eq!(converted, Decimal::from_str("72.575").unwrap());
}

#[test]
fn a_weight_that_is_not_a_mass_is_rejected() {
    let volume = Quantity::new(Decimal::from(70), Unit::Litre);
    let error = new_record(volume).validate().unwrap_err();
    assert!(matches!(error, CoreError::Validation(_)));
}

#[test]
fn a_weight_of_zero_is_rejected() {
    let error = new_record(kg("0")).validate().unwrap_err();
    assert!(matches!(error, CoreError::Validation(_)));
}

#[test]
fn an_implausible_weight_is_rejected() {
    let error = new_record(kg("700")).validate().unwrap_err();
    assert!(matches!(error, CoreError::Validation(_)));
}

#[test]
fn a_losing_goal_needs_a_target_below_the_start() {
    let error = new_goal(
        WeightObjective::Lose,
        kg("80"),
        Some(kg("85")),
        Some(kg("0.5")),
    )
    .validate()
    .unwrap_err();
    assert!(matches!(error, CoreError::Validation(_)));
}

#[test]
fn a_gaining_goal_needs_a_target_above_the_start() {
    let error = new_goal(
        WeightObjective::Gain,
        kg("80"),
        Some(kg("75")),
        Some(kg("0.5")),
    )
    .validate()
    .unwrap_err();
    assert!(matches!(error, CoreError::Validation(_)));
}

#[test]
fn a_losing_goal_needs_a_rate() {
    let error = new_goal(WeightObjective::Lose, kg("80"), Some(kg("72")), None)
        .validate()
        .unwrap_err();
    assert!(matches!(error, CoreError::Validation(_)));
}

#[test]
fn a_maintenance_goal_carries_neither_target_nor_rate() {
    assert!(
        new_goal(WeightObjective::Maintain, kg("80"), None, None)
            .validate()
            .is_ok()
    );
    assert!(
        new_goal(WeightObjective::Maintain, kg("80"), None, Some(kg("0.5")))
            .validate()
            .is_err()
    );
    assert!(
        new_goal(WeightObjective::Maintain, kg("80"), Some(kg("75")), None)
            .validate()
            .is_err()
    );
}

#[test]
fn a_rate_faster_than_is_safe_is_rejected() {
    let error = new_goal(
        WeightObjective::Lose,
        kg("80"),
        Some(kg("72")),
        Some(kg("6")),
    )
    .validate()
    .unwrap_err();
    assert!(matches!(error, CoreError::Validation(_)));
}

#[test]
fn losing_on_plan_projects_a_date() {
    let goal = goal(WeightObjective::Lose, "80", Some("76"), Some("0.5"));
    let today = date!(2026 - 09 - 03);

    let projection = project_goal(&goal, Some(Decimal::from(78)), today);

    assert_eq!(
        projection,
        GoalProjection::Projected {
            on: date!(2026 - 10 - 01),
            remaining_kg: Decimal::from(2),
        }
    );
}

#[test]
fn a_goal_with_no_weigh_ins_projects_from_the_starting_weight() {
    let goal = goal(WeightObjective::Lose, "80", Some("76"), Some("0.5"));
    let today = date!(2026 - 09 - 03);

    let projection = project_goal(&goal, None, today);

    assert_eq!(
        projection,
        GoalProjection::Projected {
            on: date!(2026 - 10 - 29),
            remaining_kg: Decimal::from(4),
        }
    );
}

#[test]
fn a_part_week_rounds_up_to_whole_days() {
    let goal = goal(WeightObjective::Lose, "80", Some("76"), Some("0.5"));
    let today = date!(2026 - 09 - 03);

    let projection = project_goal(&goal, Some(Decimal::from_str("76.1").unwrap()), today);

    assert_eq!(
        projection,
        GoalProjection::Projected {
            on: date!(2026 - 09 - 05),
            remaining_kg: Decimal::from_str("0.1").unwrap(),
        }
    );
}

#[test]
fn reaching_the_target_is_reported_as_reached() {
    let goal = goal(WeightObjective::Lose, "80", Some("76"), Some("0.5"));
    let today = date!(2026 - 09 - 03);

    assert_eq!(
        project_goal(&goal, Some(Decimal::from(76)), today),
        GoalProjection::Reached
    );
    assert_eq!(
        project_goal(&goal, Some(Decimal::from(74)), today),
        GoalProjection::Reached
    );
}

#[test]
fn drifting_away_from_the_target_just_takes_longer() {
    let goal = goal(WeightObjective::Lose, "80", Some("76"), Some("0.5"));
    let today = date!(2026 - 09 - 03);

    let projection = project_goal(&goal, Some(Decimal::from(82)), today);

    assert_eq!(
        projection,
        GoalProjection::Projected {
            on: date!(2026 - 11 - 26),
            remaining_kg: Decimal::from(6),
        }
    );
}

#[test]
fn a_maintenance_goal_has_nothing_to_project() {
    let goal = goal(WeightObjective::Maintain, "80", None, None);
    assert_eq!(
        project_goal(&goal, Some(Decimal::from(81)), date!(2026 - 09 - 03)),
        GoalProjection::Steady
    );
}

#[test]
fn gaining_projects_towards_a_higher_target() {
    let goal = goal(WeightObjective::Gain, "70", Some("74"), Some("0.25"));
    let today = date!(2026 - 09 - 03);

    let projection = project_goal(&goal, Some(Decimal::from(72)), today);

    assert_eq!(
        projection,
        GoalProjection::Projected {
            on: date!(2026 - 10 - 29),
            remaining_kg: Decimal::from(2),
        }
    );
}

#[test]
fn the_last_reading_of_a_day_stands_for_that_day() {
    let day = date!(2026 - 09 - 01);
    let records = vec![
        record("morning", day, Some(datetime!(2026-09-01 07:00 UTC)), "80"),
        record("evening", day, Some(datetime!(2026-09-01 21:00 UTC)), "81"),
    ];

    let collapsed = latest_per_day(&records);

    assert_eq!(collapsed.len(), 1);
    assert_eq!(collapsed[0].weight_kg, Decimal::from(81));
}

#[test]
fn two_untimed_readings_on_a_day_fall_back_to_the_order_they_were_entered() {
    let day = date!(2026 - 09 - 01);
    let mut first = record("first", day, None, "80");
    first.created_at = datetime!(2026-09-01 09:00 UTC);
    let mut second = record("second", day, None, "81");
    second.created_at = datetime!(2026-09-01 10:00 UTC);

    let records = vec![second, first];
    let collapsed = latest_per_day(&records);

    assert_eq!(collapsed.len(), 1);
    assert_eq!(collapsed[0].weight_kg, Decimal::from(81));
}

#[test]
fn each_day_keeps_its_own_reading_and_the_days_stay_in_order() {
    let records = vec![
        record("third", date!(2026 - 09 - 03), None, "79"),
        record("first", date!(2026 - 09 - 01), None, "81"),
        record("second", date!(2026 - 09 - 02), None, "80"),
    ];

    let collapsed = latest_per_day(&records);

    let weights: Vec<Decimal> = collapsed.iter().map(|record| record.weight_kg).collect();
    assert_eq!(
        weights,
        vec![Decimal::from(81), Decimal::from(80), Decimal::from(79)]
    );
}

#[test]
fn the_current_weight_is_the_newest_reading() {
    let records = vec![
        record("old", date!(2026 - 09 - 01), None, "81"),
        record("new", date!(2026 - 09 - 03), None, "79"),
    ];

    assert_eq!(
        current_weight(&records).map(|record| record.weight_kg),
        Some(Decimal::from(79))
    );
    assert!(current_weight(&[]).is_none());
}

#[test]
fn a_maintenance_goal_targets_the_weight_it_started_at() {
    let goal = goal(WeightObjective::Maintain, "80", None, None);
    assert_eq!(goal.effective_target_kg(), Decimal::from(80));
}

#[test]
fn every_weight_enum_round_trips_through_its_code() {
    for source in WeightSource::ALL {
        assert_eq!(WeightSource::from_str(source.code()).unwrap(), source);
    }
    for objective in WeightObjective::ALL {
        assert_eq!(
            WeightObjective::from_str(objective.code()).unwrap(),
            objective
        );
    }
    for display in WeightDisplay::ALL {
        assert_eq!(WeightDisplay::from_str(display.code()).unwrap(), display);
    }
    assert!(WeightSource::from_str("telepathy").is_err());
}
