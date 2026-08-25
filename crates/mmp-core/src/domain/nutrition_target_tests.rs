use super::*;
use time::Month;

fn date(year: i32, month: Month, day: u8) -> Date {
    Date::from_calendar_date(year, month, day).unwrap()
}

fn goals_kcal(value: i64) -> NutritionGoals {
    NutritionGoals {
        energy_kcal: Some(Decimal::new(value, 0)),
        ..Default::default()
    }
}

fn target(id: &str, effective: Date, goals: NutritionGoals) -> NutritionTarget {
    let now = OffsetDateTime::now_utc();
    NutritionTarget {
        id: NutritionTargetId::seeded(id),
        member_id: HouseholdMemberId::seeded("someone"),
        effective_from: effective,
        goals,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn directions_match_the_agreed_defaults() {
    assert_eq!(direction_for("energy_kcal"), TargetDirection::AtMost);
    assert_eq!(direction_for("protein_g"), TargetDirection::AtLeast);
    assert_eq!(direction_for("fibre_g"), TargetDirection::AtLeast);
    assert_eq!(direction_for("carbohydrate_g"), TargetDirection::Around);
    assert_eq!(direction_for("fat_g"), TargetDirection::Around);
    assert_eq!(direction_for("sugar_g"), TargetDirection::AtMost);
    assert_eq!(direction_for("saturated_fat_g"), TargetDirection::AtMost);
    assert_eq!(direction_for("salt_g"), TargetDirection::AtMost);
    assert_eq!(direction_for("cholesterol_mg"), TargetDirection::AtMost);
}

#[test]
fn an_empty_target_is_rejected() {
    let mut errors = ValidationErrors::new();
    validate_goals(&NutritionGoals::default(), &mut errors);
    assert!(!errors.is_empty());
}

#[test]
fn a_negative_goal_is_rejected() {
    let goals = NutritionGoals {
        protein_g: Some(Decimal::new(-1, 0)),
        ..Default::default()
    };
    let mut errors = ValidationErrors::new();
    validate_goals(&goals, &mut errors);
    assert!(errors.iter().any(|e| e.field == "goals.protein_g"));
}

#[test]
fn a_patch_sets_clears_and_leaves_fields_alone() {
    let current = NutritionGoals {
        energy_kcal: Some(Decimal::new(2000, 0)),
        protein_g: Some(Decimal::new(120, 0)),
        fat_g: Some(Decimal::new(70, 0)),
        ..Default::default()
    };
    let patch = NutritionGoalsPatch {
        energy_kcal: Patch::Set(Decimal::new(1800, 0)),
        protein_g: Patch::Clear,
        ..Default::default()
    };
    let applied = patch.apply(current);
    assert_eq!(applied.energy_kcal, Some(Decimal::new(1800, 0)));
    assert_eq!(applied.protein_g, None);
    assert_eq!(applied.fat_g, Some(Decimal::new(70, 0)));
}

#[test]
fn resolution_picks_the_latest_target_already_in_force() {
    let targets = vec![
        target("first", date(2026, Month::January, 1), goals_kcal(2200)),
        target("second", date(2026, Month::June, 1), goals_kcal(2000)),
    ];
    let resolved = resolve_on(&targets, date(2026, Month::August, 20)).unwrap();
    assert_eq!(resolved.goals.energy_kcal, Some(Decimal::new(2000, 0)));
}

#[test]
fn resolution_ignores_targets_that_have_not_taken_effect_yet() {
    let targets = vec![target(
        "future",
        date(2026, Month::June, 1),
        goals_kcal(2000),
    )];
    assert!(resolve_on(&targets, date(2026, Month::May, 31)).is_none());
}
