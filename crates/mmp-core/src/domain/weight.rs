use super::str_enum::str_enum;
use std::str::FromStr;

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use time::{Date, Duration, OffsetDateTime};

use super::{
    HouseholdMemberId, Patch, Quantity, Revision, Unit, UserId, WeightGoalId, WeightRecordId,
};
use crate::error::{CoreError, Result, ValidationErrors};

str_enum!(WeightDisplay, UnknownWeightDisplay, "weight display");
str_enum!(WeightObjective, UnknownWeightObjective, "weight objective");
str_enum!(WeightSource, UnknownWeightSource, "weight source");

const WEIGHT_DP: u32 = 3;
const MAX_WEIGHT_KG: i64 = 635;
const MIN_RATE_KG_PER_WEEK: &str = "0.05";
const MAX_RATE_KG_PER_WEEK: i64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeightSource {
    Manual,
    HealthConnect,
}

impl WeightSource {
    pub const ALL: [WeightSource; 2] = [WeightSource::Manual, WeightSource::HealthConnect];

    pub const fn code(&self) -> &'static str {
        match self {
            WeightSource::Manual => "manual",
            WeightSource::HealthConnect => "health_connect",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeightObjective {
    Lose,
    Maintain,
    Gain,
}

impl WeightObjective {
    pub const ALL: [WeightObjective; 3] = [
        WeightObjective::Lose,
        WeightObjective::Maintain,
        WeightObjective::Gain,
    ];

    pub const fn code(&self) -> &'static str {
        match self {
            WeightObjective::Lose => "lose",
            WeightObjective::Maintain => "maintain",
            WeightObjective::Gain => "gain",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeightDisplay {
    #[default]
    Kilograms,
    StonesPounds,
    Pounds,
}

impl WeightDisplay {
    pub const ALL: [WeightDisplay; 3] = [
        WeightDisplay::Kilograms,
        WeightDisplay::StonesPounds,
        WeightDisplay::Pounds,
    ];

    pub const fn code(&self) -> &'static str {
        match self {
            WeightDisplay::Kilograms => "kilograms",
            WeightDisplay::StonesPounds => "stones_pounds",
            WeightDisplay::Pounds => "pounds",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeightRecord {
    pub id: WeightRecordId,
    pub member_id: HouseholdMemberId,
    pub weight_kg: Decimal,
    pub recorded_on: Date,
    pub recorded_at: Option<OffsetDateTime>,
    pub source: WeightSource,
    pub recorded_by: Option<UserId>,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy)]
pub struct NewWeightRecord {
    pub member_id: HouseholdMemberId,
    pub weight: Quantity,
    pub recorded_on: Date,
    pub recorded_at: Option<OffsetDateTime>,
    pub source: WeightSource,
    pub recorded_by: Option<UserId>,
}

impl NewWeightRecord {
    pub fn weight_kg(&self) -> Result<Decimal> {
        weight_in_kilograms("weight", self.weight)
    }

    pub fn validate(&self) -> Result<()> {
        self.weight_kg().map(|_| ())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WeightRecordPatch {
    pub weight: Option<Quantity>,
    pub recorded_on: Option<Date>,
    pub recorded_at: Patch<OffsetDateTime>,
}

impl WeightRecordPatch {
    pub fn is_empty(&self) -> bool {
        self.weight.is_none() && self.recorded_on.is_none() && self.recorded_at.is_unchanged()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeightGoal {
    pub id: WeightGoalId,
    pub member_id: HouseholdMemberId,
    pub objective: WeightObjective,
    pub starting_weight_kg: Decimal,
    pub target_weight_kg: Option<Decimal>,
    pub planned_rate_kg_per_week: Option<Decimal>,
    pub started_on: Date,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl WeightGoal {
    pub fn effective_target_kg(&self) -> Decimal {
        self.target_weight_kg.unwrap_or(self.starting_weight_kg)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NewWeightGoal {
    pub member_id: HouseholdMemberId,
    pub objective: WeightObjective,
    pub starting_weight: Quantity,
    pub target_weight: Option<Quantity>,
    pub planned_rate: Option<Quantity>,
    pub started_on: Date,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoalAmounts {
    pub starting_kg: Decimal,
    pub target_kg: Option<Decimal>,
    pub rate_kg_per_week: Option<Decimal>,
}

impl NewWeightGoal {
    pub fn resolve(&self) -> Result<GoalAmounts> {
        let starting_kg = weight_in_kilograms("starting_weight", self.starting_weight)?;
        let target_kg = self
            .target_weight
            .map(|weight| weight_in_kilograms("target_weight", weight))
            .transpose()?;
        let rate_kg_per_week = self
            .planned_rate
            .map(|rate| rate_in_kilograms_per_week("planned_rate", rate))
            .transpose()?;

        let amounts = GoalAmounts {
            starting_kg,
            target_kg,
            rate_kg_per_week,
        };
        validate_goal(self.objective, &amounts)?;
        Ok(amounts)
    }

    pub fn validate(&self) -> Result<()> {
        self.resolve().map(|_| ())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WeightGoalPatch {
    pub objective: Option<WeightObjective>,
    pub starting_weight: Option<Quantity>,
    pub target_weight: Patch<Quantity>,
    pub planned_rate: Patch<Quantity>,
    pub started_on: Option<Date>,
}

impl WeightGoalPatch {
    pub fn is_empty(&self) -> bool {
        self.objective.is_none()
            && self.starting_weight.is_none()
            && self.target_weight.is_unchanged()
            && self.planned_rate.is_unchanged()
            && self.started_on.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalProjection {
    Reached,
    Steady,
    Projected { on: Date, remaining_kg: Decimal },
}

pub fn project_goal(goal: &WeightGoal, current_kg: Option<Decimal>, today: Date) -> GoalProjection {
    let current = current_kg.unwrap_or(goal.starting_weight_kg);
    let target = goal.effective_target_kg();

    let Some(rate) = goal.planned_rate_kg_per_week else {
        return GoalProjection::Steady;
    };

    let remaining = match goal.objective {
        WeightObjective::Lose => current - target,
        WeightObjective::Gain => target - current,
        WeightObjective::Maintain => return GoalProjection::Steady,
    };

    if remaining <= Decimal::ZERO {
        return GoalProjection::Reached;
    }
    if rate <= Decimal::ZERO {
        return GoalProjection::Steady;
    }

    let days = ((remaining / rate) * Decimal::from(7))
        .ceil()
        .to_i64()
        .unwrap_or(i64::MAX);
    let on = today.checked_add(Duration::days(days)).unwrap_or(Date::MAX);

    GoalProjection::Projected {
        on,
        remaining_kg: remaining,
    }
}

pub fn latest_per_day(records: &[WeightRecord]) -> Vec<&WeightRecord> {
    let mut ordered: Vec<&WeightRecord> = records.iter().collect();
    ordered.sort_by_key(|record| (record.recorded_on, record.recorded_at, record.created_at));

    let mut per_day: Vec<&WeightRecord> = Vec::new();
    for record in ordered {
        if per_day
            .last()
            .is_some_and(|last| last.recorded_on == record.recorded_on)
        {
            per_day.pop();
        }
        per_day.push(record);
    }
    per_day
}

pub fn current_weight(records: &[WeightRecord]) -> Option<&WeightRecord> {
    latest_per_day(records).last().copied()
}

fn validate_goal(objective: WeightObjective, amounts: &GoalAmounts) -> Result<()> {
    let mut errors = ValidationErrors::new();

    match objective {
        WeightObjective::Maintain => {
            if amounts.target_kg.is_some() {
                errors.push("target_weight", "A maintenance goal has no target weight");
            }
            if amounts.rate_kg_per_week.is_some() {
                errors.push("planned_rate", "A maintenance goal has no rate");
            }
        }
        WeightObjective::Lose | WeightObjective::Gain => {
            match (objective, amounts.target_kg) {
                (_, None) => errors.push("target_weight", "Required"),
                (WeightObjective::Lose, Some(target)) if target >= amounts.starting_kg => {
                    errors.push("target_weight", "Must be below your starting weight");
                }
                (WeightObjective::Gain, Some(target)) if target <= amounts.starting_kg => {
                    errors.push("target_weight", "Must be above your starting weight");
                }
                _ => {}
            }
            if amounts.rate_kg_per_week.is_none() {
                errors.push("planned_rate", "Required");
            } else if objective == WeightObjective::Gain
                && amounts.rate_kg_per_week == Some(Decimal::ZERO)
            {
                errors.push("planned_rate", "Must be more than zero");
            }
        }
    }

    errors.into_result()
}

fn weight_in_kilograms(field: &str, quantity: Quantity) -> Result<Decimal> {
    let kilograms = to_kilograms(field, quantity)?;
    let mut errors = ValidationErrors::new();
    if kilograms <= Decimal::ZERO {
        errors.push(field, "Must be more than zero");
    } else if kilograms > Decimal::from(MAX_WEIGHT_KG) {
        errors.push(field, "That does not look like a weight");
    }
    errors.into_result()?;
    Ok(kilograms)
}

fn rate_in_kilograms_per_week(field: &str, quantity: Quantity) -> Result<Decimal> {
    let kilograms = to_kilograms(field, quantity)?;
    let mut errors = ValidationErrors::new();
    let minimum = Decimal::from_str(MIN_RATE_KG_PER_WEEK).expect("a valid constant");
    if !kilograms.is_zero() && kilograms < minimum {
        errors.push(field, "Too slow to reach your goal");
    } else if kilograms > Decimal::from(MAX_RATE_KG_PER_WEEK) {
        errors.push(field, "Faster than is safe");
    }
    errors.into_result()?;
    Ok(kilograms)
}

fn to_kilograms(field: &str, quantity: Quantity) -> Result<Decimal> {
    quantity
        .convert_to(Unit::Kilogram)
        .map(|converted| converted.amount.round_dp(WEIGHT_DP))
        .map_err(|_| {
            let mut errors = ValidationErrors::new();
            errors.push(field, "Must be a weight");
            CoreError::Validation(errors)
        })
}

#[cfg(test)]
#[path = "weight_tests.rs"]
mod tests;
