use mmp_core::domain::{HabitualActivity, MemberBodyProfile, MemberBodyProfilePatch, Patch, Sex};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use super::common::iso_date;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SexDto {
    Male,
    Female,
}

impl From<Sex> for SexDto {
    fn from(value: Sex) -> Self {
        match value {
            Sex::Male => Self::Male,
            Sex::Female => Self::Female,
        }
    }
}

impl From<SexDto> for Sex {
    fn from(value: SexDto) -> Self {
        match value {
            SexDto::Male => Self::Male,
            SexDto::Female => Self::Female,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HabitualActivityDto {
    MostlySedentary,
    LightlyActive,
    Active,
    VeryActive,
}

impl From<HabitualActivity> for HabitualActivityDto {
    fn from(value: HabitualActivity) -> Self {
        match value {
            HabitualActivity::MostlySedentary => Self::MostlySedentary,
            HabitualActivity::LightlyActive => Self::LightlyActive,
            HabitualActivity::Active => Self::Active,
            HabitualActivity::VeryActive => Self::VeryActive,
        }
    }
}

impl From<HabitualActivityDto> for HabitualActivity {
    fn from(value: HabitualActivityDto) -> Self {
        match value {
            HabitualActivityDto::MostlySedentary => Self::MostlySedentary,
            HabitualActivityDto::LightlyActive => Self::LightlyActive,
            HabitualActivityDto::Active => Self::Active,
            HabitualActivityDto::VeryActive => Self::VeryActive,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MemberBodyProfileDto {
    pub member_id: Uuid,
    #[serde(
        default,
        with = "iso_date::option",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<String>, format = Date, example = "1986-01-01")]
    pub date_of_birth: Option<Date>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sex: Option<SexDto>,
    #[serde(
        with = "rust_decimal::serde::float_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<f64>, example = 180.0)]
    pub height_cm: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub habitual_activity: Option<HabitualActivityDto>,
    pub revision: i64,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
}

impl From<MemberBodyProfile> for MemberBodyProfileDto {
    fn from(value: MemberBodyProfile) -> Self {
        Self {
            member_id: value.member_id.as_uuid(),
            date_of_birth: value.date_of_birth,
            sex: value.sex.map(Into::into),
            height_cm: value.height_cm,
            habitual_activity: value.habitual_activity.map(Into::into),
            revision: value.revision.get(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateBodyProfileRequest {
    #[serde(default, with = "iso_date::patch")]
    #[schema(value_type = Option<String>, format = Date)]
    pub date_of_birth: Patch<Date>,
    #[serde(default)]
    #[schema(value_type = Option<SexDto>)]
    pub sex: Patch<SexDto>,
    #[serde(default)]
    #[schema(value_type = Option<f64>, example = 180.0)]
    pub height_cm: Patch<f64>,
    #[serde(default)]
    #[schema(value_type = Option<HabitualActivityDto>)]
    pub habitual_activity: Patch<HabitualActivityDto>,
}

impl From<UpdateBodyProfileRequest> for MemberBodyProfilePatch {
    fn from(value: UpdateBodyProfileRequest) -> Self {
        Self {
            date_of_birth: value.date_of_birth,
            sex: value.sex.map(Into::into),
            height_cm: value
                .height_cm
                .map(|height_cm| Decimal::from_f64(height_cm).unwrap_or(Decimal::ZERO)),
            habitual_activity: value.habitual_activity.map(Into::into),
        }
    }
}
