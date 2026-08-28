use mmp_core::domain::{
    ActualMealPlanComponent, ConfirmMealPlanComponent, ConfirmMealPlanEntry, MealItemRef,
    MealPlanEntryPatch, MealPlanStatus, MealSlot, NewMealPlanComponent, NewMealPlanEntry, Patch,
};
use mmp_core::services::{
    MealItem, MealItemSource, MealPlanComponentView, MealPlanDay, MealPlanEntryView, MealPlanWeek,
    MealSlotView, NutritionSummary,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(tag = "item_kind", rename_all = "snake_case")]
pub enum MealItemRefDto {
    Product { product_id: Uuid },
    Recipe { recipe_id: Uuid },
}

impl From<MealItemRef> for MealItemRefDto {
    fn from(value: MealItemRef) -> Self {
        match value {
            MealItemRef::Product { product_id } => Self::Product {
                product_id: product_id.as_uuid(),
            },
            MealItemRef::Recipe { recipe_id } => Self::Recipe {
                recipe_id: recipe_id.as_uuid(),
            },
        }
    }
}

impl From<MealItemRefDto> for MealItemRef {
    fn from(value: MealItemRefDto) -> Self {
        match value {
            MealItemRefDto::Product { product_id } => MealItemRef::product(product_id.into()),
            MealItemRefDto::Recipe { recipe_id } => MealItemRef::recipe(recipe_id.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum ItemRefRequest {
    Product { product_id: Uuid },
    Recipe { recipe_id: Uuid },
}

impl From<ItemRefRequest> for MealItemRef {
    fn from(value: ItemRefRequest) -> Self {
        match value {
            ItemRefRequest::Product { product_id } => MealItemRef::product(product_id.into()),
            ItemRefRequest::Recipe { recipe_id } => MealItemRef::recipe(recipe_id.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealPlanComponentDto {
    pub id: Uuid,
    #[serde(flatten)]
    pub item: MealItemRefDto,
    pub item_name: String,
    pub amount: AmountDto,
    pub position: i32,
    pub nutrition: NutritionDto,
    pub quality: mmp_core::domain::NutritionQuality,
    pub status: MealPlanStatus,
    pub revision: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumption_record: Option<ConsumptionRecordDto>,
}

impl From<MealPlanComponentView> for MealPlanComponentDto {
    fn from(value: MealPlanComponentView) -> Self {
        Self {
            id: value.component.id.as_uuid(),
            item: value.component.item.into(),
            item_name: value.item_name,
            amount: value.component.amount.into(),
            position: value.component.position,
            nutrition: value.nutrition.into(),
            quality: value.quality,
            status: value.component.status,
            revision: value.component.revision.get(),
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
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MealItemSourceDto {
    Planned { entry_id: Uuid, component_id: Uuid },
    Logged { record_id: Uuid },
}

impl From<MealItemSource> for MealItemSourceDto {
    fn from(value: MealItemSource) -> Self {
        match value {
            MealItemSource::Planned {
                entry_id,
                component_id,
            } => Self::Planned {
                entry_id: entry_id.as_uuid(),
                component_id: component_id.as_uuid(),
            },
            MealItemSource::Logged { record_id } => Self::Logged {
                record_id: record_id.as_uuid(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealItemDto {
    #[serde(flatten)]
    pub source: MealItemSourceDto,
    #[serde(rename = "linked_record_id", skip_serializing_if = "Option::is_none")]
    pub record_id: Option<Uuid>,
    pub status: MealPlanStatus,
    #[serde(flatten)]
    pub item: MealItemRefDto,
    pub item_name: String,
    pub amount: AmountDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_amount: Option<AmountDto>,
    #[serde(skip_serializing_if = "Option::is_none", with = "iso_date::option")]
    #[schema(value_type = Option<String>, format = Date)]
    pub planned_on: Option<Date>,
    #[serde(skip_serializing_if = "Option::is_none", with = "iso_time::option")]
    #[schema(value_type = Option<String>, example = "18:30")]
    pub at: Option<Time>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub consumed_at: Option<OffsetDateTime>,
    pub nutrition: NutritionDto,
    pub quality: mmp_core::domain::NutritionQuality,
    pub needs_attention: bool,
    pub revision: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_revision: Option<i64>,
}

impl From<MealItem> for MealItemDto {
    fn from(value: MealItem) -> Self {
        Self {
            source: value.source.into(),
            record_id: value.record_id.map(|id| id.as_uuid()),
            status: value.status,
            item: value.item.into(),
            item_name: value.item_name,
            amount: value.amount.into(),
            planned_amount: value.planned_amount.map(Into::into),
            planned_on: value.planned_on,
            at: value.at,
            consumed_at: value.consumed_at,
            nutrition: value.nutrition.into(),
            quality: value.quality,
            needs_attention: value.needs_attention,
            revision: value.revision.get(),
            record_revision: value.record_revision.map(|revision| revision.get()),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealSlotViewDto {
    pub slot: MealSlot,
    pub items: Vec<MealItemDto>,
    pub nutrition: NutritionSummaryDto,
}

impl From<MealSlotView> for MealSlotViewDto {
    fn from(value: MealSlotView) -> Self {
        Self {
            slot: value.slot,
            items: value.items.into_iter().map(Into::into).collect(),
            nutrition: value.nutrition.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealPlanDayDto {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub date: Date,
    pub entries: Vec<MealPlanEntryDto>,
    pub slots: Vec<MealSlotViewDto>,
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
            slots: value.slots.into_iter().map(Into::into).collect(),
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
    #[serde(default)]
    pub id: Option<Uuid>,
    #[serde(flatten)]
    pub item: ItemRefRequest,
    pub amount: AmountDto,
}

impl From<MealPlanComponentRequest> for NewMealPlanComponent {
    fn from(value: MealPlanComponentRequest) -> Self {
        Self {
            id: value.id.map(Into::into),
            item: value.item.into(),
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
    #[serde(default, with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub consumed_at: Option<OffsetDateTime>,
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

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MarkMealPlanComponentEatenRequest {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub consumed_on: Date,
    #[serde(default, with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub consumed_at: Option<OffsetDateTime>,
    pub amount: AmountDto,
}

impl MarkMealPlanComponentEatenRequest {
    pub fn into_domain(self, actor_id: mmp_core::domain::UserId) -> ConfirmMealPlanComponent {
        ConfirmMealPlanComponent {
            consumed_on: self.consumed_on,
            consumed_at: self.consumed_at,
            amount: self.amount.into(),
            actor_id,
        }
    }
}
