use mmp_core::domain::{
    ActualMealPlanComponent, ConfirmMealPlanEntry, MealPlanEntryPatch, MealPlanStatus, MealSlot,
    NewMealPlanComponent, NewMealPlanEntry, Patch,
};
use mmp_core::services::{
    MealPlanComponentView, MealPlanDay, MealPlanEntryView, MealPlanWeek, NutritionSummary,
};
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime, Time};
use utoipa::ToSchema;
use uuid::Uuid;

use super::common::{iso_date, iso_time};
use super::{AmountDto, ConsumptionRecordDto, NutritionDto, NutritionGoalsDto};

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NutritionSummaryDto {
    pub nutrition: NutritionDto,
    pub unknown_count: i64,
    pub partial_count: i64,
}

impl From<NutritionSummary> for NutritionSummaryDto {
    fn from(value: NutritionSummary) -> Self {
        Self {
            nutrition: value.nutrition.into(),
            unknown_count: value.unknown_count,
            partial_count: value.partial_count,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealPlanComponentDto {
    pub id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
    pub amount: AmountDto,
    pub position: i32,
    pub nutrition: NutritionDto,
    pub quality: mmp_core::domain::NutritionQuality,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumption_record: Option<ConsumptionRecordDto>,
}

impl From<MealPlanComponentView> for MealPlanComponentDto {
    fn from(value: MealPlanComponentView) -> Self {
        Self {
            id: value.component.id.as_uuid(),
            product_id: value.component.product_id.as_uuid(),
            product_name: value.product_name,
            amount: value.component.amount.into(),
            position: value.component.position,
            nutrition: value.nutrition.into(),
            quality: value.quality,
            consumption_record: value.consumption_record.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealPlanEntryDto {
    pub id: Uuid,
    pub member_id: Uuid,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub planned_on: Date,
    #[serde(with = "iso_time::option")]
    #[schema(value_type = Option<String>, example = "18:30")]
    pub planned_time: Option<Time>,
    pub slot: MealSlot,
    pub status: MealPlanStatus,
    pub components: Vec<MealPlanComponentDto>,
    pub planned: NutritionSummaryDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<NutritionSummaryDto>,
    pub needs_attention: bool,
    pub created_by: Uuid,
    pub updated_by: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_by: Option<Uuid>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub resolved_at: Option<OffsetDateTime>,
    pub revision: i64,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
}

impl From<MealPlanEntryView> for MealPlanEntryDto {
    fn from(value: MealPlanEntryView) -> Self {
        Self {
            id: value.entry.id.as_uuid(),
            member_id: value.entry.member_id.as_uuid(),
            planned_on: value.entry.planned_on,
            planned_time: value.entry.planned_time,
            slot: value.entry.slot,
            status: value.entry.status,
            components: value.components.into_iter().map(Into::into).collect(),
            planned: value.planned.into(),
            actual: value.actual.map(Into::into),
            needs_attention: value.needs_attention,
            created_by: value.entry.created_by.as_uuid(),
            updated_by: value.entry.updated_by.as_uuid(),
            resolved_by: value.entry.resolved_by.map(|id| id.as_uuid()),
            resolved_at: value.entry.resolved_at,
            revision: value.entry.revision.get(),
            created_at: value.entry.created_at,
            updated_at: value.entry.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealPlanDayDto {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub date: Date,
    pub entries: Vec<MealPlanEntryDto>,
    pub actual: NutritionSummaryDto,
    pub remaining_planned: NutritionSummaryDto,
    pub projected: NutritionSummaryDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<NutritionGoalsDto>,
}

impl From<MealPlanDay> for MealPlanDayDto {
    fn from(value: MealPlanDay) -> Self {
        Self {
            date: value.date,
            entries: value.entries.into_iter().map(Into::into).collect(),
            actual: value.actual.into(),
            remaining_planned: value.remaining_planned.into(),
            projected: value.projected.into(),
            target: value.target.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealPlanWeekDto {
    pub member_id: Uuid,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub week_start: Date,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub week_end: Date,
    pub days: Vec<MealPlanDayDto>,
    pub actual: NutritionSummaryDto,
    pub remaining_planned: NutritionSummaryDto,
    pub projected: NutritionSummaryDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<NutritionGoalsDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub insufficient_target_coverage: Vec<String>,
}

impl From<MealPlanWeek> for MealPlanWeekDto {
    fn from(value: MealPlanWeek) -> Self {
        Self {
            member_id: value.member_id.as_uuid(),
            week_start: value.week_start,
            week_end: value.week_end,
            days: value.days.into_iter().map(Into::into).collect(),
            actual: value.actual.into(),
            remaining_planned: value.remaining_planned.into(),
            projected: value.projected.into(),
            target: value.target.map(Into::into),
            insufficient_target_coverage: value.insufficient_target_coverage,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MealPlanComponentRequest {
    pub product_id: Uuid,
    pub amount: AmountDto,
}

impl From<MealPlanComponentRequest> for NewMealPlanComponent {
    fn from(value: MealPlanComponentRequest) -> Self {
        Self {
            product_id: value.product_id.into(),
            amount: value.amount.into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateMealPlanEntryRequest {
    #[serde(default)]
    pub id: Option<Uuid>,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub planned_on: Date,
    #[serde(default, with = "iso_time::option")]
    #[schema(value_type = Option<String>, example = "18:30")]
    pub planned_time: Option<Time>,
    pub slot: MealSlot,
    pub components: Vec<MealPlanComponentRequest>,
}

impl CreateMealPlanEntryRequest {
    pub fn into_domain(
        self,
        member_id: mmp_core::domain::HouseholdMemberId,
        actor_id: mmp_core::domain::UserId,
    ) -> NewMealPlanEntry {
        NewMealPlanEntry {
            id: self.id.map(Into::into),
            member_id,
            planned_on: self.planned_on,
            planned_time: self.planned_time,
            slot: self.slot,
            components: self.components.into_iter().map(Into::into).collect(),
            actor_id,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateMealPlanEntryRequest {
    #[serde(default, with = "iso_date::option")]
    #[schema(value_type = Option<String>, format = Date)]
    pub planned_on: Option<Date>,
    #[serde(default)]
    #[schema(value_type = Option<String>, example = "18:30")]
    pub planned_time: Patch<String>,
    #[serde(default)]
    pub slot: Option<MealSlot>,
    #[serde(default)]
    pub components: Option<Vec<MealPlanComponentRequest>>,
}

impl UpdateMealPlanEntryRequest {
    pub fn into_domain(self) -> Result<MealPlanEntryPatch, String> {
        let planned_time = match self.planned_time {
            Patch::Unchanged => None,
            Patch::Clear => Some(None),
            Patch::Set(value) => Some(Some(
                iso_time::parse(&value).map_err(|_| "Planned time must use HH:mm".to_owned())?,
            )),
        };
        Ok(MealPlanEntryPatch {
            planned_on: self.planned_on,
            planned_time,
            slot: self.slot,
            components: self
                .components
                .map(|components| components.into_iter().map(Into::into).collect()),
        })
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ActualMealPlanComponentRequest {
    pub component_id: Uuid,
    pub amount: AmountDto,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MarkMealPlanEatenRequest {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub consumed_on: Date,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub consumed_at: OffsetDateTime,
    pub components: Vec<ActualMealPlanComponentRequest>,
}

impl MarkMealPlanEatenRequest {
    pub fn into_domain(self, actor_id: mmp_core::domain::UserId) -> ConfirmMealPlanEntry {
        ConfirmMealPlanEntry {
            consumed_on: self.consumed_on,
            consumed_at: self.consumed_at,
            components: self
                .components
                .into_iter()
                .map(|component| ActualMealPlanComponent {
                    component_id: component.component_id.into(),
                    amount: component.amount.into(),
                })
                .collect(),
            actor_id,
        }
    }
}
