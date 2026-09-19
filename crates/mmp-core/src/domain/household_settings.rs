use super::str_enum::str_enum;

use time::{OffsetDateTime, Time};

use super::{MealSlot, Revision, ShoppingSection};
use crate::error::{Result, ValidationErrors};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseholdSettings {
    pub meal_times: MealTimes,
    pub timezone: String,
    pub missing_stock_interpretation: MissingStockInterpretation,
    pub assume_eaten_when_time_passes: bool,
    pub section_order: SectionOrder,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionOrder([ShoppingSection; ShoppingSection::ALL.len()]);

impl Default for SectionOrder {
    fn default() -> Self {
        Self(ShoppingSection::ALL)
    }
}

impl SectionOrder {
    pub fn new(order: [ShoppingSection; ShoppingSection::ALL.len()]) -> Result<Self> {
        let mut seen = order;
        seen.sort_by_key(|section| section.code());
        let mut all = ShoppingSection::ALL;
        all.sort_by_key(|section| section.code());
        if seen != all {
            let mut errors = ValidationErrors::new();
            errors.push("section_order", "List every aisle exactly once.");
            return errors.into_result().map(|()| Self(order));
        }
        Ok(Self(order))
    }

    pub fn from_slice(order: &[ShoppingSection]) -> Result<Self> {
        let Ok(exact) = <[ShoppingSection; ShoppingSection::ALL.len()]>::try_from(order) else {
            let mut errors = ValidationErrors::new();
            errors.push("section_order", "List every aisle exactly once.");
            return errors.into_result().map(|()| Self::default());
        };
        Self::new(exact)
    }

    pub fn sections(&self) -> &[ShoppingSection] {
        &self.0
    }

    pub fn rank(&self, section: ShoppingSection) -> usize {
        self.0
            .iter()
            .position(|held| *held == section)
            .unwrap_or(self.0.len())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HouseholdSettingsPatch {
    pub breakfast_time: Option<Time>,
    pub lunch_time: Option<Time>,
    pub dinner_time: Option<Time>,
    pub timezone: Option<String>,
    pub missing_stock_interpretation: Option<MissingStockInterpretation>,
    pub assume_eaten_when_time_passes: Option<bool>,
    pub section_order: Option<Vec<ShoppingSection>>,
}

impl HouseholdSettingsPatch {
    pub fn is_empty(&self) -> bool {
        self.breakfast_time.is_none()
            && self.lunch_time.is_none()
            && self.dinner_time.is_none()
            && self.timezone.is_none()
            && self.missing_stock_interpretation.is_none()
            && self.assume_eaten_when_time_passes.is_none()
            && self.section_order.is_none()
    }

    pub fn validate(&self) -> Result<()> {
        let mut errors = ValidationErrors::new();
        if let Some(timezone) = &self.timezone
            && crate::ports::resolve_timezone(timezone).is_none()
        {
            errors.push("timezone", "That is not a timezone we recognise.");
        }
        errors.into_result()
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
