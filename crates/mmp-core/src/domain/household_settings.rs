use super::str_enum::str_enum;

use time::{OffsetDateTime, Time};

use super::{MealSlot, Revision};

str_enum!(
    MissingStockInterpretation,
    UnknownMissingStockInterpretation,
    "missing stock interpretation"
);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HouseholdSettings {
    pub meal_times: MealTimes,
    pub missing_stock_interpretation: MissingStockInterpretation,
    pub default_all_members_participate: bool,
    pub assume_eaten_when_time_passes: bool,
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
    pub default_all_members_participate: Option<bool>,
    pub assume_eaten_when_time_passes: Option<bool>,
}

impl HouseholdSettingsPatch {
    pub fn is_empty(&self) -> bool {
        self.breakfast_time.is_none()
            && self.lunch_time.is_none()
            && self.dinner_time.is_none()
            && self.missing_stock_interpretation.is_none()
            && self.default_all_members_participate.is_none()
            && self.assume_eaten_when_time_passes.is_none()
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
