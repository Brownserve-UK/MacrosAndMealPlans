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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HouseholdSettings {
    pub meal_times: MealTimes,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HouseholdSettingsPatch {
    pub breakfast_time: Option<Time>,
    pub lunch_time: Option<Time>,
    pub dinner_time: Option<Time>,
}

impl HouseholdSettingsPatch {
    pub fn is_empty(&self) -> bool {
        self.breakfast_time.is_none() && self.lunch_time.is_none() && self.dinner_time.is_none()
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
