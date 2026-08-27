use mmp_core::domain::{HouseholdSettings, HouseholdSettingsPatch};
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

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HouseholdSettingsDto {
    #[serde(flatten)]
    pub meal_times: MealTimesDto,
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
}

impl From<UpdateMealTimesRequest> for HouseholdSettingsPatch {
    fn from(value: UpdateMealTimesRequest) -> Self {
        HouseholdSettingsPatch {
            breakfast_time: value.breakfast,
            lunch_time: value.lunch,
            dinner_time: value.dinner,
        }
    }
}
