use super::*;
use time::macros::{date, datetime};

fn input(
    sex: Sex,
    activity: HabitualActivity,
    objective: WeightObjective,
    pace: Option<Pace>,
) -> CalorieCalculationInput {
    let now = datetime!(2026-09-12 09:00 UTC);
    CalorieCalculationInput {
        id: CalorieCalculationId::seeded("calculation"),
        member_id: HouseholdMemberId::seeded("member"),
        nutrition_target_id: NutritionTargetId::seeded("target"),
        calculated_on: date!(2026 - 09 - 12),
        date_of_birth: date!(1986 - 01 - 01),
        sex,
        height_cm: Decimal::from(180),
        weight_kg: Decimal::from(80),
        habitual_activity: activity,
        objective,
        emphasis: NutritionEmphasis::General,
        pace,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn maintenance_uses_mifflin_st_jeor_and_rounds_to_ten() {
    let result = calculate(input(
        Sex::Male,
        HabitualActivity::MostlySedentary,
        WeightObjective::Maintain,
        None,
    ))
    .unwrap();

    assert_eq!(result.age_years, 40);
    assert_eq!(result.maintenance_kcal, Decimal::from(2420));
    assert_eq!(result.recommended_kcal, Decimal::from(2420));
    assert_eq!(result.adjustment_kcal, Decimal::ZERO);
    assert_eq!(result.floor_kcal, Decimal::from(1500));
}

#[test]
fn each_activity_level_uses_its_lifestyle_multiplier() {
    let levels = [
        (HabitualActivity::MostlySedentary, 2420),
        (HabitualActivity::LightlyActive, 2680),
        (HabitualActivity::Active, 3030),
        (HabitualActivity::VeryActive, 3370),
    ];
    for (activity, expected) in levels {
        let result =
            calculate(input(Sex::Male, activity, WeightObjective::Maintain, None)).unwrap();
        assert_eq!(result.maintenance_kcal, Decimal::from(expected));
    }
}

#[test]
fn losing_subtracts_the_selected_pace() {
    let result = calculate(input(
        Sex::Male,
        HabitualActivity::LightlyActive,
        WeightObjective::Lose,
        Some(Pace::Standard),
    ))
    .unwrap();

    assert_eq!(result.requested_rate_kg_per_week, Some(decimal("0.5")));
    assert_eq!(result.adjustment_kcal, Decimal::from(-550));
    assert_eq!(result.recommended_kcal, Decimal::from(2130));
    assert!(!result.eased);
}

#[test]
fn gaining_adds_the_selected_pace() {
    let result = calculate(input(
        Sex::Female,
        HabitualActivity::LightlyActive,
        WeightObjective::Gain,
        Some(Pace::Steady),
    ))
    .unwrap();

    assert_eq!(result.adjustment_kcal, Decimal::from(275));
    assert_eq!(result.recommended_kcal, Decimal::from(2695));
    assert_eq!(result.floor_kcal, Decimal::from(1200));
}

#[test]
fn a_deficit_that_crosses_the_floor_is_clamped_and_eased() {
    let mut candidate = input(
        Sex::Female,
        HabitualActivity::MostlySedentary,
        WeightObjective::Lose,
        Some(Pace::Fastest),
    );
    candidate.height_cm = Decimal::from(160);
    candidate.weight_kg = Decimal::from(55);
    candidate.date_of_birth = date!(1966 - 01 - 01);

    let result = calculate(candidate).unwrap();

    assert_eq!(result.maintenance_kcal, Decimal::from(1520));
    assert_eq!(result.recommended_kcal, Decimal::from(1200));
    assert_eq!(result.applied_rate_kg_per_week, Some(decimal("0.25")));
    assert!(result.eased);
}

#[test]
fn maintenance_below_the_floor_does_not_recommend_a_deficit() {
    let mut candidate = input(
        Sex::Female,
        HabitualActivity::MostlySedentary,
        WeightObjective::Lose,
        Some(Pace::Steady),
    );
    candidate.height_cm = Decimal::from(140);
    candidate.weight_kg = Decimal::from(40);
    candidate.date_of_birth = date!(1936 - 01 - 01);

    let result = calculate(candidate).unwrap();

    assert!(result.maintenance_kcal < result.floor_kcal);
    assert_eq!(result.recommended_kcal, result.maintenance_kcal);
    assert_eq!(result.applied_rate_kg_per_week, Some(Decimal::ZERO));
    assert!(result.eased);
}

#[test]
fn gaining_rejects_a_losing_only_pace() {
    let error = calculate(input(
        Sex::Male,
        HabitualActivity::Active,
        WeightObjective::Gain,
        Some(Pace::Faster),
    ))
    .unwrap_err();
    assert!(matches!(error, crate::error::CoreError::Validation(_)));
}
