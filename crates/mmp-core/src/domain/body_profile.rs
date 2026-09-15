use rust_decimal::Decimal;
use time::{Date, OffsetDateTime};

use super::str_enum::str_enum;
use super::{HouseholdMemberId, Patch, Revision};
use crate::error::{Result, ValidationErrors};

str_enum!(Sex, UnknownSex, "sex");
str_enum!(
    HabitualActivity,
    UnknownHabitualActivity,
    "habitual activity"
);

const MIN_HEIGHT_CM: i64 = 50;
const MAX_HEIGHT_CM: i64 = 260;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sex {
    Male,
    Female,
}

impl Sex {
    pub const ALL: [Sex; 2] = [Sex::Male, Sex::Female];

    pub const fn code(&self) -> &'static str {
        match self {
            Sex::Male => "male",
            Sex::Female => "female",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HabitualActivity {
    MostlySedentary,
    LightlyActive,
    Active,
    VeryActive,
}

impl HabitualActivity {
    pub const ALL: [HabitualActivity; 4] = [
        HabitualActivity::MostlySedentary,
        HabitualActivity::LightlyActive,
        HabitualActivity::Active,
        HabitualActivity::VeryActive,
    ];

    pub const fn code(&self) -> &'static str {
        match self {
            HabitualActivity::MostlySedentary => "mostly_sedentary",
            HabitualActivity::LightlyActive => "lightly_active",
            HabitualActivity::Active => "active",
            HabitualActivity::VeryActive => "very_active",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberBodyProfile {
    pub member_id: HouseholdMemberId,
    pub date_of_birth: Option<Date>,
    pub sex: Option<Sex>,
    pub height_cm: Option<Decimal>,
    pub habitual_activity: Option<HabitualActivity>,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl MemberBodyProfile {
    pub fn validate(&self, on: Date) -> Result<()> {
        let mut errors = ValidationErrors::new();
        if let Some(date_of_birth) = self.date_of_birth
            && date_of_birth > on
        {
            errors.push("date_of_birth", "Cannot be in the future");
        }
        if let Some(height_cm) = self.height_cm
            && (height_cm <= Decimal::from(MIN_HEIGHT_CM)
                || height_cm > Decimal::from(MAX_HEIGHT_CM))
        {
            errors.push("height_cm", "That does not look like a height");
        }
        errors.into_result()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemberBodyProfilePatch {
    pub date_of_birth: Patch<Date>,
    pub sex: Patch<Sex>,
    pub height_cm: Patch<Decimal>,
    pub habitual_activity: Patch<HabitualActivity>,
}

impl MemberBodyProfilePatch {
    pub fn is_empty(&self) -> bool {
        self.date_of_birth.is_unchanged()
            && self.sex.is_unchanged()
            && self.height_cm.is_unchanged()
            && self.habitual_activity.is_unchanged()
    }

    pub fn apply(self, profile: &mut MemberBodyProfile) {
        profile.date_of_birth = self.date_of_birth.apply(profile.date_of_birth);
        profile.sex = self.sex.apply(profile.sex);
        profile.height_cm = self.height_cm.apply(profile.height_cm);
        profile.habitual_activity = self.habitual_activity.apply(profile.habitual_activity);
    }
}

pub fn age_on(date_of_birth: Date, on: Date) -> Result<i32> {
    let mut errors = ValidationErrors::new();
    if date_of_birth > on {
        errors.push("date_of_birth", "Cannot be in the future");
        return Err(errors.into());
    }

    let birthday_has_passed =
        (on.month(), on.day()) >= (date_of_birth.month(), date_of_birth.day());
    Ok(on.year() - date_of_birth.year() - i32::from(!birthday_has_passed))
}

#[cfg(test)]
#[path = "body_profile_tests.rs"]
mod tests;
