use mmp_core::domain::{HouseholdSettings, HouseholdSettingsPatch, MissingStockInterpretation};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, Time};
use utoipa::ToSchema;

use super::common::iso_time;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealTimesDto {
    #[serde(with = "iso_time")]
    #[schema(value_type = String, example = "08:00")]
    pub breakfast: Time,
    #[serde(with = "iso_time")]
    #[schema(value_type = String, example = "12:30")]
    pub lunch: Time,
    #[serde(with = "iso_time")]
    #[schema(value_type = String, example = "18:00")]
    pub dinner: Time,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MissingStockInterpretationDto {
    Absent,
    Unknown,
}

impl From<MissingStockInterpretation> for MissingStockInterpretationDto {
    fn from(value: MissingStockInterpretation) -> Self {
        match value {
            MissingStockInterpretation::Absent => Self::Absent,
            MissingStockInterpretation::Unknown => Self::Unknown,
        }
    }
}

impl From<MissingStockInterpretationDto> for MissingStockInterpretation {
    fn from(value: MissingStockInterpretationDto) -> Self {
        match value {
            MissingStockInterpretationDto::Absent => Self::Absent,
            MissingStockInterpretationDto::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HouseholdSettingsDto {
    #[serde(flatten)]
    pub meal_times: MealTimesDto,
    pub missing_stock_interpretation: MissingStockInterpretationDto,
    pub default_all_members_participate: bool,
    pub revision: i64,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
}

impl From<HouseholdSettings> for HouseholdSettingsDto {
    fn from(value: HouseholdSettings) -> Self {
        Self {
            meal_times: MealTimesDto {
                breakfast: value.meal_times.breakfast,
                lunch: value.meal_times.lunch,
                dinner: value.meal_times.dinner,
            },
            missing_stock_interpretation: value.missing_stock_interpretation.into(),
            default_all_members_participate: value.default_all_members_participate,
            revision: value.revision.get(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateMealTimesRequest {
    #[serde(default, with = "iso_time::option")]
    #[schema(value_type = Option<String>, example = "08:00")]
    pub breakfast: Option<Time>,
    #[serde(default, with = "iso_time::option")]
    #[schema(value_type = Option<String>, example = "12:30")]
    pub lunch: Option<Time>,
    #[serde(default, with = "iso_time::option")]
    #[schema(value_type = Option<String>, example = "18:00")]
    pub dinner: Option<Time>,
    #[serde(default)]
    pub missing_stock_interpretation: Option<MissingStockInterpretationDto>,
    #[serde(default)]
    pub default_all_members_participate: Option<bool>,
}

impl From<UpdateMealTimesRequest> for HouseholdSettingsPatch {
    fn from(value: UpdateMealTimesRequest) -> Self {
        HouseholdSettingsPatch {
            breakfast_time: value.breakfast,
            lunch_time: value.lunch,
            dinner_time: value.dinner,
            missing_stock_interpretation: value.missing_stock_interpretation.map(Into::into),
            default_all_members_participate: value.default_all_members_participate,
        }
    }
}
