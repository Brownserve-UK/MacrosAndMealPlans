use mmp_core::domain::{
    ActualMealPlanComponent, ComponentPreparation, ConfirmMealPlanComponent, ConfirmMealPlanEntry,
    MealGuestGroup, MealItemRef, MealOptOut, MealParticipantAllocation, MealPlanEntryPatch,
    MealPlanScope, MealPlanStatus, MealSlot, NewMealGuestAllocation, NewMealGuestGroup,
    NewMealParticipant, NewMealParticipantAllocation, NewMealPlanComponent, NewMealPlanEntry,
    ParticipantStatus, Patch, Portioning, ReviewMealOutcomes, ReviewedGuestOutcome,
    ReviewedMealOutcome, ReviewedMemberOutcome, SetMealParticipants, SlotAttendance,
};
use mmp_core::services::{
    MealItem, MealItemSource, MealParticipantView, MealPlanComponentView, MealPlanDay,
    MealPlanEntryView, MealPlanWeek, MealSlotView, NutritionSummary, StockAffected,
};
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime, Time};
use utoipa::ToSchema;
use uuid::Uuid;

use super::common::{iso_date, iso_time};
use super::{AmountDto, ConsumptionRecordDto, NutritionDto, NutritionGoalsDto, StockOutcomeDto};

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
pub struct AmountSummaryDto {
    pub kind: String,
    #[schema(value_type = String)]
    pub value: rust_decimal::Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

impl From<mmp_core::domain::ConsumedAmount> for AmountSummaryDto {
    fn from(value: mmp_core::domain::ConsumedAmount) -> Self {
        let unit = match value {
            mmp_core::domain::ConsumedAmount::Measure(quantity) => Some(quantity.unit.to_string()),
            _ => None,
        };
        Self {
            kind: value.kind_code().to_owned(),
            value: value.value(),
            unit,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ComponentPreparationDto {
    pub prepared: AmountSummaryDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allocated: Option<AmountSummaryDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unallocated: Option<AmountSummaryDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leftover: Option<AmountSummaryDto>,
    pub shortage: bool,
}

impl From<ComponentPreparation> for ComponentPreparationDto {
    fn from(value: ComponentPreparation) -> Self {
        Self {
            prepared: value.prepared.into(),
            allocated: value.allocated.map(Into::into),
            unallocated: value.unallocated.map(Into::into),
            leftover: value.leftover.map(Into::into),
            shortage: value.shortage,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealParticipantAllocationDto {
    pub component_id: Uuid,
    pub allocated: AmountSummaryDto,
    pub status: ParticipantStatus,
}

impl From<MealParticipantAllocation> for MealParticipantAllocationDto {
    fn from(value: MealParticipantAllocation) -> Self {
        Self {
            component_id: value.component_id.as_uuid(),
            allocated: value.allocated.into(),
            status: value.status,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealParticipantDto {
    pub member_id: Uuid,
    pub display_name: String,
    pub status: MealPlanStatus,
    pub allocations: Vec<MealParticipantAllocationDto>,
    pub nutrition: NutritionSummaryDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealOptOutDto {
    pub member_id: Uuid,
    pub created_by: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
}

impl From<MealOptOut> for MealOptOutDto {
    fn from(value: MealOptOut) -> Self {
        Self {
            member_id: value.member_id.as_uuid(),
            created_by: value.created_by.as_uuid(),
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SlotAttendanceDto {
    pub member_id: Uuid,
    pub display_name: String,
    pub attendance: SlotAttendance,
    #[serde(with = "iso_time::option")]
    #[schema(value_type = Option<String>, example = "18:30")]
    pub claimed_time: Option<Time>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealGuestGroupDto {
    pub id: Uuid,
    pub count: i32,
    pub status: MealPlanStatus,
    pub allocations: Vec<MealParticipantAllocationDto>,
}

impl From<MealGuestGroup> for MealGuestGroupDto {
    fn from(value: MealGuestGroup) -> Self {
        let status = mmp_core::domain::derive_guest_status(&value);
        Self {
            id: value.id.as_uuid(),
            count: value.count,
            status,
            allocations: value
                .allocations
                .into_iter()
                .map(|allocation| MealParticipantAllocationDto {
                    component_id: allocation.component_id.as_uuid(),
                    allocated: allocation.allocated.into(),
                    status: allocation.status,
                })
                .collect(),
        }
    }
}

impl From<MealParticipantView> for MealParticipantDto {
    fn from(value: MealParticipantView) -> Self {
        Self {
            member_id: value.member_id.as_uuid(),
            display_name: value.display_name,
            status: value.status,
            allocations: value.allocations.into_iter().map(Into::into).collect(),
            nutrition: value.nutrition.into(),
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
    pub subject_status: MealPlanStatus,
    pub preparation: ComponentPreparationDto,
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
            status: value.status,
            subject_status: value.subject_status,
            preparation: value.preparation.into(),
            revision: value.component.revision.get(),
            consumption_record: value.consumption_record.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealPlanEntryDto {
    pub id: Uuid,
    pub scope: MealPlanScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_member_id: Option<Uuid>,
    pub participants: Vec<MealParticipantDto>,
    pub guest_groups: Vec<MealGuestGroupDto>,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub planned_on: Date,
    #[serde(with = "iso_time::option")]
    #[schema(value_type = Option<String>, example = "18:30")]
    pub planned_time: Option<Time>,
    pub slot: MealSlot,
    pub portioning: Portioning,
    pub status: MealPlanStatus,
    pub components: Vec<MealPlanComponentDto>,
    pub planned: NutritionSummaryDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<NutritionSummaryDto>,
    pub needs_attention: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stock_outcomes: Vec<StockOutcomeDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opted_out: Vec<MealOptOutDto>,
    pub created_by: Uuid,
    pub updated_by: Uuid,
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
            scope: value.entry.scope,
            member_id: value.entry.member_id.map(|id| id.as_uuid()),
            subject_member_id: value.subject_member_id.map(|id| id.as_uuid()),
            participants: value.participants.into_iter().map(Into::into).collect(),
            guest_groups: value
                .entry
                .guest_groups
                .iter()
                .cloned()
                .map(Into::into)
                .collect(),
            planned_on: value.entry.planned_on,
            planned_time: value.entry.planned_time,
            slot: value.entry.slot,
            portioning: value.entry.portioning,
            status: value.entry.status(),
            components: value.components.into_iter().map(Into::into).collect(),
            planned: value.planned.into(),
            actual: value.actual.map(Into::into),
            needs_attention: value.needs_attention,
            stock_outcomes: Vec::new(),
            opted_out: value
                .entry
                .opted_out
                .iter()
                .cloned()
                .map(Into::into)
                .collect(),
            created_by: value.entry.created_by.as_uuid(),
            updated_by: value.entry.updated_by.as_uuid(),
            revision: value.entry.revision.get(),
            created_at: value.entry.created_at,
            updated_at: value.entry.updated_at,
        }
    }
}

impl From<StockAffected<MealPlanEntryView>> for MealPlanEntryDto {
    fn from(value: StockAffected<MealPlanEntryView>) -> Self {
        let stock_outcomes = value.stock.iter().cloned().map(Into::into).collect();
        let mut dto: MealPlanEntryDto = value.into_value().into();
        dto.stock_outcomes = stock_outcomes;
        dto
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

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlannerCapabilitiesDto {
    pub can_edit: bool,
    pub can_delete: bool,
    pub can_record_guests: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlannerPersonDto {
    pub member_id: Uuid,
    pub display_name: String,
    pub status: MealPlanStatus,
    pub allocations: Vec<MealParticipantAllocationDto>,
    pub can_record: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlannerFoodDto {
    pub id: Uuid,
    #[serde(flatten)]
    pub item: MealItemRefDto,
    pub item_name: String,
    pub amount: AmountDto,
    pub shortage: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlannerMealDto {
    pub id: Uuid,
    pub scope: MealPlanScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_name: Option<String>,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub planned_on: Date,
    #[serde(with = "iso_time::option")]
    #[schema(value_type = Option<String>, example = "18:30")]
    pub planned_time: Option<Time>,
    pub slot: MealSlot,
    pub portioning: Portioning,
    pub status: MealPlanStatus,
    pub foods: Vec<PlannerFoodDto>,
    pub people: Vec<PlannerPersonDto>,
    pub guest_groups: Vec<MealGuestGroupDto>,
    pub opted_out: Vec<MealOptOutDto>,
    pub can_opt_out: bool,
    pub can_join: bool,
    pub capabilities: PlannerCapabilitiesDto,
    pub revision: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlannerWeekDto {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub week_start: Date,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub week_end: Date,
    pub meals: Vec<PlannerMealDto>,
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
    #[serde(default)]
    pub household: bool,
    #[serde(default)]
    pub member_id: Option<Uuid>,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub planned_on: Date,
    #[serde(default, with = "iso_time::option")]
    #[schema(value_type = Option<String>, example = "18:30")]
    pub planned_time: Option<Time>,
    pub slot: MealSlot,
    #[serde(default)]
    pub portioning: Option<Portioning>,
    pub components: Vec<MealPlanComponentRequest>,
    #[serde(default)]
    pub participants: Option<Vec<MealParticipantRequest>>,
    #[serde(default)]
    pub guest_count: i32,
    #[serde(default)]
    pub guest_allocations: Vec<MealParticipantAllocationRequest>,
}

impl CreateMealPlanEntryRequest {
    pub fn into_domain(
        self,
        member_id: mmp_core::domain::HouseholdMemberId,
        actor_id: mmp_core::domain::UserId,
    ) -> NewMealPlanEntry {
        let (scope, member_id) = if self.household {
            (MealPlanScope::Household, None)
        } else {
            (
                MealPlanScope::Member,
                Some(self.member_id.map(Into::into).unwrap_or(member_id)),
            )
        };
        NewMealPlanEntry {
            id: self.id.map(Into::into),
            scope,
            member_id,
            planned_on: self.planned_on,
            planned_time: self.planned_time,
            slot: self.slot,
            portioning: self.portioning.unwrap_or(Portioning::Equal),
            components: self.components.into_iter().map(Into::into).collect(),
            participants: self.participants.map(participants_into_domain),
            guest_groups: guest_groups_into_domain(self.guest_count, self.guest_allocations),
            actor_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetMealPlanParticipantsRequest {
    pub participants: Vec<MealParticipantRequest>,
    #[serde(default)]
    pub guest_count: i32,
    #[serde(default)]
    pub guest_allocations: Vec<MealParticipantAllocationRequest>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MealParticipantRequest {
    pub member_id: Uuid,
    #[serde(default)]
    pub allocations: Vec<MealParticipantAllocationRequest>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MealParticipantAllocationRequest {
    pub component_id: Uuid,
    pub amount: AmountDto,
}

impl SetMealPlanParticipantsRequest {
    pub fn into_domain(self, actor_id: mmp_core::domain::UserId) -> SetMealParticipants {
        SetMealParticipants {
            actor_id,
            participants: participants_into_domain(self.participants),
            guest_groups: guest_groups_into_domain(self.guest_count, self.guest_allocations),
        }
    }
}

fn participants_into_domain(participants: Vec<MealParticipantRequest>) -> Vec<NewMealParticipant> {
    participants
        .into_iter()
        .map(|participant| NewMealParticipant {
            id: None,
            member_id: participant.member_id.into(),
            allocations: participant
                .allocations
                .into_iter()
                .map(|allocation| NewMealParticipantAllocation {
                    component_id: allocation.component_id.into(),
                    allocated: allocation.amount.into(),
                })
                .collect(),
        })
        .collect()
}

fn guest_groups_into_domain(
    guest_count: i32,
    allocations: Vec<MealParticipantAllocationRequest>,
) -> Vec<NewMealGuestGroup> {
    (guest_count > 0)
        .then(|| NewMealGuestGroup {
            id: None,
            count: guest_count,
            allocations: allocations
                .into_iter()
                .map(|allocation| NewMealGuestAllocation {
                    component_id: allocation.component_id.into(),
                    allocated: allocation.amount.into(),
                })
                .collect(),
        })
        .into_iter()
        .collect()
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
    pub portioning: Option<Portioning>,
    #[serde(default)]
    pub components: Option<Vec<MealPlanComponentRequest>>,
    #[serde(default)]
    pub participants: Option<Vec<MealParticipantRequest>>,
    #[serde(default)]
    pub guest_count: Option<i32>,
    #[serde(default)]
    pub guest_allocations: Option<Vec<MealParticipantAllocationRequest>>,
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
            portioning: self.portioning,
            components: self
                .components
                .map(|components| components.into_iter().map(Into::into).collect()),
            participants: self.participants.map(participants_into_domain),
            guest_groups: self.guest_count.map(|count| {
                guest_groups_into_domain(count, self.guest_allocations.unwrap_or_default())
            }),
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
    #[serde(default)]
    pub member_id: Option<Uuid>,
    pub components: Vec<ActualMealPlanComponentRequest>,
}

impl MarkMealPlanEatenRequest {
    pub fn into_domain(self, actor_id: mmp_core::domain::UserId) -> ConfirmMealPlanEntry {
        ConfirmMealPlanEntry {
            consumed_on: self.consumed_on,
            consumed_at: self.consumed_at,
            subject_member_id: self.member_id.map(Into::into),
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
#[serde(tag = "result", rename_all = "snake_case")]
pub enum ReviewedMealOutcomeRequest {
    AsPlanned,
    NotEaten,
    Changed {
        components: Vec<ActualMealPlanComponentRequest>,
    },
}

impl ReviewedMealOutcomeRequest {
    fn into_domain(self) -> ReviewedMealOutcome {
        match self {
            Self::AsPlanned => ReviewedMealOutcome::AsPlanned,
            Self::NotEaten => ReviewedMealOutcome::NotEaten,
            Self::Changed { components } => ReviewedMealOutcome::Changed(
                components
                    .into_iter()
                    .map(|component| ActualMealPlanComponent {
                        component_id: component.component_id.into(),
                        amount: component.amount.into(),
                    })
                    .collect(),
            ),
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReviewedMemberOutcomeRequest {
    pub member_id: Uuid,
    #[serde(flatten)]
    pub outcome: ReviewedMealOutcomeRequest,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReviewedGuestOutcomeRequest {
    pub source_group_id: Uuid,
    pub count: i32,
    #[serde(flatten)]
    pub outcome: ReviewedMealOutcomeRequest,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReviewMealOutcomesRequest {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub consumed_on: Date,
    #[serde(default, with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub consumed_at: Option<OffsetDateTime>,
    #[serde(default)]
    pub members: Vec<ReviewedMemberOutcomeRequest>,
    #[serde(default)]
    pub guests: Vec<ReviewedGuestOutcomeRequest>,
}

impl ReviewMealOutcomesRequest {
    pub fn into_domain(self, actor_id: mmp_core::domain::UserId) -> ReviewMealOutcomes {
        ReviewMealOutcomes {
            consumed_on: self.consumed_on,
            consumed_at: self.consumed_at,
            members: self
                .members
                .into_iter()
                .map(|reviewed| ReviewedMemberOutcome {
                    member_id: reviewed.member_id.into(),
                    outcome: reviewed.outcome.into_domain(),
                })
                .collect(),
            guests: self
                .guests
                .into_iter()
                .map(|reviewed| ReviewedGuestOutcome {
                    source_group_id: reviewed.source_group_id.into(),
                    count: reviewed.count,
                    outcome: reviewed.outcome.into_domain(),
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
    #[serde(default)]
    pub member_id: Option<Uuid>,
    pub amount: AmountDto,
}

impl MarkMealPlanComponentEatenRequest {
    pub fn into_domain(self, actor_id: mmp_core::domain::UserId) -> ConfirmMealPlanComponent {
        ConfirmMealPlanComponent {
            consumed_on: self.consumed_on,
            consumed_at: self.consumed_at,
            subject_member_id: self.member_id.map(Into::into),
            amount: self.amount.into(),
            actor_id,
        }
    }
}
