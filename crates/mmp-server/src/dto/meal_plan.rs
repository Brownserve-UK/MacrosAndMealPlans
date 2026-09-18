use mmp_core::domain::{
    ActualMealPlanComponent, AdHocKind, ChangedMealOutcome, ComponentPreparation,
    ConfirmMealPlanComponent, ConfirmMealPlanEntry, MealAttendance, MealGroupPatch, MealGuestGroup,
    MealItemRef, MealOccasionPatch, MealParticipantAllocation, MealPlanEntryId, MealPlanStatus,
    MealSlot, MealTimes, NewMealGroup, NewMealGuestGroup, NewMealOccasion, NewMealParticipant,
    NewMealPlanComponent, ParticipantStatus, Patch, ReplacementItem, ReviewMealOutcomes,
    ReviewedGuestOutcome, ReviewedMealOutcome, ReviewedMemberOutcome, WeightObjective,
    direction_for,
};
use mmp_core::services::{
    MealDiner, MealGroupView, MealItem, MealItemSource, MealOccasionView, MealParticipantView,
    MealPlanComponentView, MealPlanDay, MealPlanEntryView, MealPlanWeek, MealSlotView,
    NutritionSummary, PlannerDay, PlannerMember, PlannerWeek, StockAffected,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime, Time};
use utoipa::ToSchema;
use uuid::Uuid;

use super::common::{iso_date, iso_time};
use super::{
    AmountDto, ConsumptionRecordDto, NutritionDto, NutritionGoalsDto, StockOutcomeDto,
    TargetDirectionDto,
};

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
    Dish { dish_recipe_id: Uuid },
    Ingredient { ingredient_id: Uuid },
    PreparedMeal { prepared_meal_id: Uuid },
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
            MealItemRef::Dish { recipe_id } => Self::Dish {
                dish_recipe_id: recipe_id.as_uuid(),
            },
            MealItemRef::Ingredient { ingredient_id } => Self::Ingredient {
                ingredient_id: ingredient_id.as_uuid(),
            },
            MealItemRef::PreparedMeal { prepared_meal_id } => Self::PreparedMeal {
                prepared_meal_id: prepared_meal_id.as_uuid(),
            },
        }
    }
}

impl From<MealItemRefDto> for MealItemRef {
    fn from(value: MealItemRefDto) -> Self {
        match value {
            MealItemRefDto::Product { product_id } => MealItemRef::product(product_id.into()),
            MealItemRefDto::Recipe { recipe_id } => MealItemRef::recipe(recipe_id.into()),
            MealItemRefDto::Dish { dish_recipe_id } => MealItemRef::dish(dish_recipe_id.into()),
            MealItemRefDto::Ingredient { ingredient_id } => {
                MealItemRef::ingredient(ingredient_id.into())
            }
            MealItemRefDto::PreparedMeal { prepared_meal_id } => {
                MealItemRef::prepared_meal(prepared_meal_id.into())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum ItemRefRequest {
    Product { product_id: Uuid },
    Recipe { recipe_id: Uuid },
    Dish { dish_recipe_id: Uuid },
    Ingredient { ingredient_id: Uuid },
    PreparedMeal { prepared_meal_id: Uuid },
}

impl From<ItemRefRequest> for MealItemRef {
    fn from(value: ItemRefRequest) -> Self {
        match value {
            ItemRefRequest::Product { product_id } => MealItemRef::product(product_id.into()),
            ItemRefRequest::Recipe { recipe_id } => MealItemRef::recipe(recipe_id.into()),
            ItemRefRequest::Dish { dish_recipe_id } => MealItemRef::dish(dish_recipe_id.into()),
            ItemRefRequest::Ingredient { ingredient_id } => {
                MealItemRef::ingredient(ingredient_id.into())
            }
            ItemRefRequest::PreparedMeal { prepared_meal_id } => {
                MealItemRef::prepared_meal(prepared_meal_id.into())
            }
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
pub struct MealGuestGroupDto {
    pub id: Uuid,
    pub count: i32,
    pub status: MealPlanStatus,
    pub allocations: Vec<MealParticipantAllocationDto>,
}

impl MealGuestGroupDto {
    pub fn build(value: MealGuestGroup, assumption: mmp_core::domain::Assumption) -> Self {
        let status = mmp_core::domain::derive_guest_status(&value, assumption);
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
    pub needs_cooking: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooked: Option<CookedDto>,
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
            needs_cooking: value.component.item.is_recipe() && value.cooked.is_none(),
            cooked: value.cooked.as_ref().map(|batch| CookedDto {
                prepared_batch_id: batch.id.as_uuid(),
                prepared_at: batch.prepared_at,
                servings_produced: batch.servings_produced,
                revision: batch.revision.get(),
            }),
            consumption_record: value.consumption_record.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NeedsReviewDto {
    pub personal_meals: Vec<MealPlanEntryDto>,
    pub household_meals: Vec<MealPlanEntryDto>,
    pub food_mappings: Vec<FoodMappingReviewDto>,
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FoodMappingKindDto {
    Ingredient,
    PreparedMeal,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FoodMappingReviewDto {
    pub id: Uuid,
    pub name: String,
    pub kind: FoodMappingKindDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealPlanEntryDto {
    pub id: Uuid,
    pub occasion_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ad_hoc: Option<AdHocKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_member_id: Option<Uuid>,
    pub everyone: bool,
    pub participants: Vec<MealParticipantDto>,
    pub guest_groups: Vec<MealGuestGroupDto>,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub planned_on: Date,
    #[serde(with = "iso_time::option")]
    #[schema(value_type = Option<String>, example = "18:30")]
    pub planned_time: Option<Time>,
    pub slot: MealSlot,
    pub status: MealPlanStatus,
    pub components: Vec<MealPlanComponentDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooking_servings: Option<i32>,
    pub planned: NutritionSummaryDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<NutritionSummaryDto>,
    pub needs_attention: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stock_outcomes: Vec<StockOutcomeDto>,
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
            occasion_id: value.entry.occasion_id.as_uuid(),
            label: value.entry.label.clone(),
            ad_hoc: value.entry.ad_hoc,
            subject_member_id: value.subject_member_id.map(|id| id.as_uuid()),
            everyone: value.entry.everyone,
            participants: value.participants.into_iter().map(Into::into).collect(),
            guest_groups: value
                .entry
                .guest_groups
                .iter()
                .cloned()
                .map(|group| MealGuestGroupDto::build(group, value.assumption))
                .collect(),
            planned_on: value.entry.planned_on,
            planned_time: value.entry.planned_time,
            slot: value.entry.slot,
            status: value.status,
            components: value.components.into_iter().map(Into::into).collect(),
            cooking_servings: value.entry.cooking_servings,
            planned: value.planned.into(),
            actual: value.actual.map(Into::into),
            needs_attention: value.needs_attention,
            stock_outcomes: Vec::new(),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calorie_direction: Option<TargetDirectionDto>,
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
            calorie_direction: None,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calorie_direction: Option<TargetDirectionDto>,
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
            calorie_direction: None,
            insufficient_target_coverage: value.insufficient_target_coverage,
        }
    }
}

impl MealPlanWeekDto {
    pub fn with_calorie_direction(mut self, objective: Option<WeightObjective>) -> Self {
        self.calorie_direction = calorie_direction(&self.target, objective);
        for day in &mut self.days {
            day.calorie_direction = calorie_direction(&day.target, objective);
        }
        self
    }
}

fn calorie_direction(
    target: &Option<NutritionGoalsDto>,
    objective: Option<WeightObjective>,
) -> Option<TargetDirectionDto> {
    target
        .as_ref()
        .filter(|target| target.energy_kcal.is_some())
        .zip(objective)
        .map(|(_, objective)| direction_for("energy_kcal", objective).into())
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CookedDto {
    pub prepared_batch_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub prepared_at: OffsetDateTime,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64)]
    pub servings_produced: Decimal,
    pub revision: i64,
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
        #[serde(default)]
        replacements: Vec<ReplacementItemRequest>,
    },
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReplacementItemRequest {
    #[serde(flatten)]
    pub item: MealItemRefDto,
    pub amount: AmountDto,
}

impl ReviewedMealOutcomeRequest {
    fn into_domain(self) -> ReviewedMealOutcome {
        match self {
            Self::AsPlanned => ReviewedMealOutcome::AsPlanned,
            Self::NotEaten => ReviewedMealOutcome::NotEaten,
            Self::Changed {
                components,
                replacements,
            } => ReviewedMealOutcome::Changed(ChangedMealOutcome {
                components: components
                    .into_iter()
                    .map(|component| ActualMealPlanComponent {
                        component_id: component.component_id.into(),
                        amount: component.amount.into(),
                    })
                    .collect(),
                replacements: replacements
                    .into_iter()
                    .map(|replacement| ReplacementItem {
                        item: replacement.item.into(),
                        amount: replacement.amount.into(),
                    })
                    .collect(),
            }),
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

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateMealPlanEntryRequest {
    #[serde(default)]
    pub components: Option<Vec<MealPlanComponentRequest>>,
}

impl UpdateMealPlanEntryRequest {
    pub fn into_domain(self) -> MealGroupPatch {
        MealGroupPatch {
            components: self
                .components
                .map(|components| components.into_iter().map(Into::into).collect()),
            ..Default::default()
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewGroupComponentRequest {
    #[serde(flatten)]
    pub item: ItemRefRequest,
    pub amount: AmountDto,
}

impl From<NewGroupComponentRequest> for NewMealPlanComponent {
    fn from(value: NewGroupComponentRequest) -> Self {
        Self {
            id: None,
            item: value.item.into(),
            amount: value.amount.into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewGroupParticipantRequest {
    pub member_id: Uuid,
    #[serde(default)]
    pub note: Option<String>,
}

impl From<NewGroupParticipantRequest> for NewMealParticipant {
    fn from(value: NewGroupParticipantRequest) -> Self {
        Self {
            id: None,
            member_id: value.member_id.into(),
            note: value.note,
            allocations: Vec::new(),
        }
    }
}

fn guest_groups_of(guest_count: i32) -> Vec<NewMealGuestGroup> {
    (guest_count > 0)
        .then(|| NewMealGuestGroup {
            id: None,
            count: guest_count,
            allocations: Vec::new(),
        })
        .into_iter()
        .collect()
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewGroupRequest {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub ad_hoc: Option<AdHocKind>,
    #[serde(default)]
    pub components: Vec<NewGroupComponentRequest>,
    #[serde(default = "default_true")]
    pub everyone: bool,
    #[serde(default)]
    pub participants: Vec<NewGroupParticipantRequest>,
    #[serde(default)]
    pub guest_count: i32,
    #[serde(default)]
    pub cooking_servings: Option<i32>,
}

impl NewGroupRequest {
    pub fn into_domain(self) -> NewMealGroup {
        NewMealGroup {
            id: Some(MealPlanEntryId::new()),
            label: self.label,
            ad_hoc: self.ad_hoc,
            components: self.components.into_iter().map(Into::into).collect(),
            everyone: self.everyone,
            participants: self.participants.into_iter().map(Into::into).collect(),
            guest_groups: guest_groups_of(self.guest_count),
            cooking_servings: self.cooking_servings,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateOccasionRequest {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub planned_on: Date,
    pub slot: MealSlot,
    pub group: NewGroupRequest,
}

impl CreateOccasionRequest {
    pub fn into_domain(self, actor_id: mmp_core::domain::UserId) -> NewMealOccasion {
        NewMealOccasion {
            id: None,
            planned_on: self.planned_on,
            slot: self.slot,
            planned_time: None,
            note: None,
            group: self.group.into_domain(),
            actor_id,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct OccasionPatchRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>, example = "18:30")]
    pub planned_time: Patch<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub note: Patch<String>,
}

impl OccasionPatchRequest {
    pub fn into_domain(self) -> Result<MealOccasionPatch, String> {
        let planned_time = match self.planned_time {
            Patch::Unchanged => None,
            Patch::Clear => Some(None),
            Patch::Set(value) => Some(Some(
                iso_time::parse(&value).map_err(|_| "Planned time must use HH:mm".to_owned())?,
            )),
        };
        let note = match self.note {
            Patch::Unchanged => None,
            Patch::Clear => Some(None),
            Patch::Set(value) => Some(Some(value)),
        };
        Ok(MealOccasionPatch { planned_time, note })
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MoveOrCopyOccasionRequest {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub planned_on: Date,
    pub slot: MealSlot,
}

fn patch_option<T>(patch: Patch<T>) -> Option<Option<T>> {
    match patch {
        Patch::Unchanged => None,
        Patch::Clear => Some(None),
        Patch::Set(value) => Some(Some(value)),
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct GroupPatchRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub label: Patch<String>,
    #[serde(default)]
    #[schema(value_type = Option<AdHocKind>)]
    pub ad_hoc: Patch<AdHocKind>,
    #[serde(default)]
    pub components: Option<Vec<NewGroupComponentRequest>>,
    #[serde(default)]
    pub everyone: Option<bool>,
    #[serde(default)]
    pub participants: Option<Vec<NewGroupParticipantRequest>>,
    #[serde(default)]
    pub guest_count: Option<i32>,
    #[serde(default)]
    #[schema(value_type = Option<i32>)]
    pub cooking_servings: Patch<i32>,
}

impl GroupPatchRequest {
    pub fn into_domain(self) -> MealGroupPatch {
        MealGroupPatch {
            label: patch_option(self.label),
            ad_hoc: patch_option(self.ad_hoc),
            components: self
                .components
                .map(|components| components.into_iter().map(Into::into).collect()),
            everyone: self.everyone,
            participants: self
                .participants
                .map(|participants| participants.into_iter().map(Into::into).collect()),
            guest_groups: self.guest_count.map(guest_groups_of),
            cooking_servings: patch_option(self.cooking_servings),
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AttendanceRequest {
    Eating {
        group_id: Uuid,
        #[serde(default)]
        note: Option<String>,
    },
    Elsewhere,
    Unaccounted,
}

impl AttendanceRequest {
    pub fn into_domain(self) -> MealAttendance {
        match self {
            Self::Eating { group_id, note } => MealAttendance::Eating {
                group_id: group_id.into(),
                note,
            },
            Self::Elsewhere => MealAttendance::Elsewhere,
            Self::Unaccounted => MealAttendance::Unaccounted,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetAttendanceRequest {
    pub attendance: AttendanceRequest,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GroupParticipantDto {
    pub member_id: Uuid,
    pub name: String,
    pub note: Option<String>,
}

impl From<MealDiner> for GroupParticipantDto {
    fn from(value: MealDiner) -> Self {
        Self {
            member_id: value.member_id.as_uuid(),
            name: value.display_name,
            note: value.note,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GroupViewDto {
    pub id: Uuid,
    pub name: String,
    pub label: Option<String>,
    pub ad_hoc: Option<AdHocKind>,
    pub components: Vec<MealPlanComponentDto>,
    pub everyone: bool,
    pub participants: Vec<GroupParticipantDto>,
    pub guest_count: i32,
    pub serves: i32,
    pub cooking_servings: Option<i32>,
    pub effective_cooking_servings: i32,
    pub cook_minutes: Option<i32>,
    pub to_buy: i64,
    pub leftover_servings_available: Option<f64>,
    pub revision: i64,
}

impl GroupViewDto {
    pub fn build(value: MealGroupView, to_buy: i64) -> Self {
        Self {
            id: value.entry.entry.id.as_uuid(),
            name: value.name,
            label: value.entry.entry.label.clone(),
            ad_hoc: value.entry.entry.ad_hoc,
            components: value.entry.components.into_iter().map(Into::into).collect(),
            everyone: value.entry.entry.everyone,
            participants: value.diners.into_iter().map(Into::into).collect(),
            guest_count: value.guest_count,
            serves: value.serves,
            cooking_servings: value.cooking_servings,
            effective_cooking_servings: value.effective_cooking_servings,
            cook_minutes: value.cook_minutes,
            to_buy,
            leftover_servings_available: value
                .leftover_servings_available
                .and_then(|amount| amount.to_f64()),
            revision: value.entry.entry.revision.get(),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OccasionViewDto {
    pub id: Uuid,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub planned_on: Date,
    pub slot: MealSlot,
    #[serde(with = "iso_time::option")]
    #[schema(value_type = Option<String>, example = "18:30")]
    pub planned_time: Option<Time>,
    #[serde(with = "iso_time::option")]
    #[schema(value_type = Option<String>, example = "18:30")]
    pub effective_time: Option<Time>,
    pub note: Option<String>,
    pub groups: Vec<GroupViewDto>,
    pub absent_member_ids: Vec<Uuid>,
    pub unaccounted_member_ids: Vec<Uuid>,
    pub revision: i64,
}

impl OccasionViewDto {
    pub fn build(value: MealOccasionView, to_buy_of: impl Fn(Uuid) -> i64) -> Self {
        Self {
            id: value.occasion.id.as_uuid(),
            planned_on: value.occasion.planned_on,
            slot: value.occasion.slot,
            planned_time: value.occasion.planned_time,
            effective_time: value.effective_time,
            note: value.occasion.note.clone(),
            groups: value
                .groups
                .into_iter()
                .map(|group| {
                    let id = group.entry.entry.id.as_uuid();
                    GroupViewDto::build(group, to_buy_of(id))
                })
                .collect(),
            absent_member_ids: value
                .absent_member_ids
                .into_iter()
                .map(|id| id.as_uuid())
                .collect(),
            unaccounted_member_ids: value
                .unaccounted_member_ids
                .into_iter()
                .map(|id| id.as_uuid())
                .collect(),
            revision: value.occasion.revision.get(),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlannerDayDto {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub date: Date,
    pub occasions: Vec<Option<OccasionViewDto>>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UsualTimesDto {
    #[serde(with = "iso_time")]
    #[schema(value_type = String, example = "08:00")]
    pub breakfast: Time,
    #[serde(with = "iso_time")]
    #[schema(value_type = String, example = "12:30")]
    pub lunch: Time,
    #[serde(with = "iso_time")]
    #[schema(value_type = String, example = "18:00")]
    pub dinner: Time,
    #[serde(with = "iso_time::option")]
    #[schema(value_type = Option<String>)]
    pub snacks: Option<Time>,
}

impl From<MealTimes> for UsualTimesDto {
    fn from(value: MealTimes) -> Self {
        Self {
            breakfast: value.breakfast,
            lunch: value.lunch,
            dinner: value.dinner,
            snacks: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlannerMemberDto {
    pub id: Uuid,
    pub name: String,
    pub initials: String,
}

impl From<PlannerMember> for PlannerMemberDto {
    fn from(value: PlannerMember) -> Self {
        Self {
            id: value.id.as_uuid(),
            initials: initials_of(&value.display_name),
            name: value.display_name,
        }
    }
}

fn initials_of(name: &str) -> String {
    let mut words = name.split_whitespace();
    match (words.next(), words.next()) {
        (Some(first), Some(second)) => format!(
            "{}{}",
            first
                .chars()
                .next()
                .unwrap_or_default()
                .to_ascii_uppercase(),
            second
                .chars()
                .next()
                .unwrap_or_default()
                .to_ascii_uppercase(),
        ),
        (Some(first), None) => first
            .chars()
            .take(2)
            .collect::<String>()
            .to_ascii_uppercase(),
        _ => String::new(),
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlannerWeekDto {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub week_start: Date,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date)]
    pub week_end: Date,
    pub usual_times: UsualTimesDto,
    pub members: Vec<PlannerMemberDto>,
    pub days: Vec<PlannerDayDto>,
}

impl PlannerWeekDto {
    pub fn build(value: PlannerWeek, to_buy_of: impl Fn(Uuid) -> i64) -> Self {
        Self {
            week_start: value.week_start,
            week_end: value.week_end,
            usual_times: value.usual_times.into(),
            members: value.members.into_iter().map(Into::into).collect(),
            days: value
                .days
                .into_iter()
                .map(|day: PlannerDay| PlannerDayDto {
                    date: day.date,
                    occasions: day
                        .occasions
                        .into_iter()
                        .map(|occasion| {
                            occasion.map(|view| OccasionViewDto::build(view, &to_buy_of))
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}
