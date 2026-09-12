use std::str::FromStr;

use rust_decimal::Decimal;
use time::{Date, OffsetDateTime};

use super::{
    CalorieCalculationId, HabitualActivity, HouseholdMemberId, NutritionTargetId, Revision, Sex,
    WeightObjective, age_on,
};
use crate::error::{Result, ValidationErrors};

pub const CALORIE_FORMULA: &str = "mifflin_st_jeor";
pub const ACTIVITY_SOURCE_SELF_REPORTED: &str = "self_reported";

const MALE_FLOOR_KCAL: i64 = 1500;
const FEMALE_FLOOR_KCAL: i64 = 1200;
const CALORIES_PER_KG_PER_DAY: i64 = 1100;
const RATE_STEP: &str = "0.05";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pace {
    Steady,
    Standard,
    Faster,
    Fastest,
}

impl Pace {
    pub fn rate_for(self, objective: WeightObjective) -> Result<Decimal> {
        let rate = match (objective, self) {
            (WeightObjective::Lose, Pace::Steady) | (WeightObjective::Gain, Pace::Steady) => "0.25",
            (WeightObjective::Lose, Pace::Standard) | (WeightObjective::Gain, Pace::Standard) => {
                "0.5"
            }
            (WeightObjective::Lose, Pace::Faster) => "0.75",
            (WeightObjective::Lose, Pace::Fastest) => "1.0",
            (WeightObjective::Maintain, _) => {
                return validation("pace", "A maintenance goal has no pace");
            }
            (WeightObjective::Gain, Pace::Faster | Pace::Fastest) => {
                return validation("pace", "That pace is not available for gaining weight");
            }
        };
        Ok(decimal(rate))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CalorieCalculationInput {
    pub id: CalorieCalculationId,
    pub member_id: HouseholdMemberId,
    pub nutrition_target_id: NutritionTargetId,
    pub calculated_on: Date,
    pub date_of_birth: Date,
    pub sex: Sex,
    pub height_cm: Decimal,
    pub weight_kg: Decimal,
    pub habitual_activity: HabitualActivity,
    pub objective: WeightObjective,
    pub pace: Option<Pace>,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalorieCalculation {
    pub id: CalorieCalculationId,
    pub member_id: HouseholdMemberId,
    pub nutrition_target_id: NutritionTargetId,
    pub calculated_on: Date,
    pub formula: String,
    pub activity_source: String,
    pub habitual_activity: HabitualActivity,
    pub age_years: i32,
    pub sex: Sex,
    pub height_cm: Decimal,
    pub weight_kg: Decimal,
    pub objective: WeightObjective,
    pub requested_rate_kg_per_week: Option<Decimal>,
    pub applied_rate_kg_per_week: Option<Decimal>,
    pub maintenance_kcal: Decimal,
    pub adjustment_kcal: Decimal,
    pub recommended_kcal: Decimal,
    pub floor_kcal: Decimal,
    pub eased: bool,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub fn calculate(input: CalorieCalculationInput) -> Result<CalorieCalculation> {
    let height_cm = input.height_cm.round_dp(1);
    validate_measurements(height_cm, input.weight_kg)?;
    let age_years = age_on(input.date_of_birth, input.calculated_on)?;
    let bmr = Decimal::from(10) * input.weight_kg + decimal("6.25") * height_cm
        - Decimal::from(5 * age_years)
        + match input.sex {
            Sex::Male => Decimal::from(5),
            Sex::Female => Decimal::from(-161),
        };
    let maintenance_kcal = round_to_ten(bmr * activity_multiplier(input.habitual_activity));
    if maintenance_kcal <= Decimal::ZERO {
        return validation("profile", "These details do not produce a calorie estimate");
    }

    let floor_kcal = Decimal::from(match input.sex {
        Sex::Male => MALE_FLOOR_KCAL,
        Sex::Female => FEMALE_FLOOR_KCAL,
    });
    let requested_rate = match (input.objective, input.pace) {
        (WeightObjective::Maintain, None) => None,
        (WeightObjective::Maintain, Some(_)) => {
            return validation("pace", "A maintenance goal has no pace");
        }
        (_, Some(pace)) => Some(pace.rate_for(input.objective)?),
        (_, None) => return validation("pace", "Required"),
    };

    let (applied_rate, recommended_kcal, eased) = match input.objective {
        WeightObjective::Maintain => (None, maintenance_kcal, false),
        WeightObjective::Gain => {
            let rate = requested_rate.expect("a gaining calculation has a rate");
            (
                Some(rate),
                maintenance_kcal + rate * Decimal::from(CALORIES_PER_KG_PER_DAY),
                false,
            )
        }
        WeightObjective::Lose => {
            let requested = requested_rate.expect("a losing calculation has a rate");
            let proposed = maintenance_kcal - requested * Decimal::from(CALORIES_PER_KG_PER_DAY);
            if proposed >= floor_kcal {
                (Some(requested), proposed, false)
            } else if maintenance_kcal <= floor_kcal {
                (Some(Decimal::ZERO), maintenance_kcal, true)
            } else {
                let safe_rate = ((maintenance_kcal - floor_kcal)
                    / Decimal::from(CALORIES_PER_KG_PER_DAY)
                    / decimal(RATE_STEP))
                .floor()
                    * decimal(RATE_STEP);
                (Some(safe_rate), floor_kcal, true)
            }
        }
    };

    Ok(CalorieCalculation {
        id: input.id,
        member_id: input.member_id,
        nutrition_target_id: input.nutrition_target_id,
        calculated_on: input.calculated_on,
        formula: CALORIE_FORMULA.to_owned(),
        activity_source: ACTIVITY_SOURCE_SELF_REPORTED.to_owned(),
        habitual_activity: input.habitual_activity,
        age_years,
        sex: input.sex,
        height_cm,
        weight_kg: input.weight_kg,
        objective: input.objective,
        requested_rate_kg_per_week: requested_rate,
        applied_rate_kg_per_week: applied_rate,
        maintenance_kcal,
        adjustment_kcal: recommended_kcal - maintenance_kcal,
        recommended_kcal,
        floor_kcal,
        eased,
        revision: input.revision,
        created_at: input.created_at,
        updated_at: input.updated_at,
    })
}

fn activity_multiplier(activity: HabitualActivity) -> Decimal {
    decimal(match activity {
        HabitualActivity::MostlySedentary => "1.40",
        HabitualActivity::LightlyActive => "1.55",
        HabitualActivity::Active => "1.75",
        HabitualActivity::VeryActive => "1.95",
    })
}

fn round_to_ten(value: Decimal) -> Decimal {
    (value / Decimal::from(10)).round() * Decimal::from(10)
}

fn validate_measurements(height_cm: Decimal, weight_kg: Decimal) -> Result<()> {
    let mut errors = ValidationErrors::new();
    if height_cm <= Decimal::from(50) || height_cm > Decimal::from(260) {
        errors.push("height_cm", "That does not look like a height");
    }
    if weight_kg <= Decimal::ZERO || weight_kg > Decimal::from(635) {
        errors.push("weight", "That does not look like a weight");
    }
    errors.into_result()
}

fn validation<T>(field: &str, message: &str) -> Result<T> {
    let mut errors = ValidationErrors::new();
    errors.push(field, message);
    Err(errors.into())
}

fn decimal(value: &str) -> Decimal {
    Decimal::from_str(value).expect("a valid decimal constant")
}

#[cfg(test)]
#[path = "calorie_target_tests.rs"]
mod tests;
