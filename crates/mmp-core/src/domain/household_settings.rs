use std::fmt;
use std::str::FromStr;

use time::{OffsetDateTime, Time};

use super::{MealSlot, Revision};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MealTimes {
    pub breakfast: Time,
    pub lunch: Time,
    pub dinner: Time,
}

impl MealTimes {
    pub fn for_slot(&self, slot: MealSlot) -> Option<Time> {
        match slot {
            MealSlot::Breakfast => Some(self.breakfast),
            MealSlot::Lunch => Some(self.lunch),
            MealSlot::Dinner => Some(self.dinner),
            MealSlot::Snacks => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingStockInterpretation {
    Absent,
    Unknown,
}

impl MissingStockInterpretation {
    pub const ALL: [MissingStockInterpretation; 2] = [
        MissingStockInterpretation::Absent,
        MissingStockInterpretation::Unknown,
    ];

    pub const fn code(&self) -> &'static str {
        match self {
            MissingStockInterpretation::Absent => "absent",
            MissingStockInterpretation::Unknown => "unknown",
        }
    }
}

impl fmt::Display for MissingStockInterpretation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known missing stock interpretation")]
pub struct UnknownMissingStockInterpretation(pub String);

impl FromStr for MissingStockInterpretation {
    type Err = UnknownMissingStockInterpretation;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        MissingStockInterpretation::ALL
            .into_iter()
            .find(|i| i.code() == s)
            .ok_or_else(|| UnknownMissingStockInterpretation(s.to_owned()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HouseholdSettings {
    pub meal_times: MealTimes,
    pub missing_stock_interpretation: MissingStockInterpretation,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HouseholdSettingsPatch {
    pub breakfast_time: Option<Time>,
    pub lunch_time: Option<Time>,
    pub dinner_time: Option<Time>,
    pub missing_stock_interpretation: Option<MissingStockInterpretation>,
}

impl HouseholdSettingsPatch {
    pub fn is_empty(&self) -> bool {
        self.breakfast_time.is_none()
            && self.lunch_time.is_none()
            && self.dinner_time.is_none()
            && self.missing_stock_interpretation.is_none()
    }

    pub fn apply(self, mut times: MealTimes) -> MealTimes {
        if let Some(value) = self.breakfast_time {
            times.breakfast = value;
        }
        if let Some(value) = self.lunch_time {
            times.lunch = value;
        }
        if let Some(value) = self.dinner_time {
            times.dinner = value;
        }
        times
    }
}

#[cfg(test)]
#[path = "household_settings_tests.rs"]
mod tests;
