use mmp_core::domain::{
    ConsumedAmount, ConsumptionRecord, ConsumptionRecordId, ConsumptionRecordPatch, MealSlot,
    NewConsumptionRecord, NutritionQuality, Quantity, Unit,
};
use mmp_core::services::{DayTotals, DiaryDay, DiaryEntry};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::common::iso_date;
use super::nutrition::NutritionDto;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AmountDto {
    Measure {
        #[serde(with = "rust_decimal::serde::float")]
        #[schema(value_type = f64, example = 150.0)]
        value: Decimal,
        #[schema(example = "g")]
        unit: Unit,
    },
    Servings {
        #[serde(with = "rust_decimal::serde::float")]
        #[schema(value_type = f64, example = 1.0)]
        value: Decimal,
    },
    Packs {
        #[serde(with = "rust_decimal::serde::float")]
        #[schema(value_type = f64, example = 0.5)]
        value: Decimal,
    },
}

impl From<ConsumedAmount> for AmountDto {
    fn from(value: ConsumedAmount) -> Self {
        match value {
            ConsumedAmount::Measure(quantity) => AmountDto::Measure {
                value: quantity.amount,
                unit: quantity.unit,
            },
            ConsumedAmount::Servings(value) => AmountDto::Servings { value },
            ConsumedAmount::Packs(value) => AmountDto::Packs { value },
        }
    }
}

impl From<AmountDto> for ConsumedAmount {
    fn from(value: AmountDto) -> Self {
        match value {
            AmountDto::Measure { value, unit } => {
                ConsumedAmount::Measure(Quantity::new(value, unit))
            }
            AmountDto::Servings { value } => ConsumedAmount::Servings(value),
            AmountDto::Packs { value } => ConsumedAmount::Packs(value),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AmountKindDto {
    Measure,
    Servings,
    Packs,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ConsumptionRecordDto {
    pub id: Uuid,
    pub member_id: Uuid,
    pub product_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meal_plan_entry_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meal_plan_component_id: Option<Uuid>,
    pub slot: MealSlot,
    pub amount: AmountDto,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-08-22")]
    pub consumed_on: Date,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub consumed_at: OffsetDateTime,
    pub nutrition: NutritionDto,
    pub quality: NutritionQuality,
    pub revision: i64,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
}

impl From<ConsumptionRecord> for ConsumptionRecordDto {
    fn from(value: ConsumptionRecord) -> Self {
        Self {
            id: value.id.as_uuid(),
            member_id: value.member_id.as_uuid(),
            product_id: value.product_id.as_uuid(),
            recorded_by: value.recorded_by.map(|id| id.as_uuid()),
            meal_plan_entry_id: value.meal_plan_entry_id.map(|id| id.as_uuid()),
            meal_plan_component_id: value.meal_plan_component_id.map(|id| id.as_uuid()),
            slot: value.slot,
            amount: value.amount.into(),
            consumed_on: value.consumed_on,
            consumed_at: value.consumed_at,
            nutrition: value.nutrition.into(),
            quality: value.quality,
            revision: value.revision.get(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateConsumptionRequest {
    pub member_id: Uuid,
    pub product_id: Uuid,
    pub slot: MealSlot,
    pub amount: AmountDto,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-08-22")]
    pub consumed_on: Date,
    #[serde(default, with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub consumed_at: Option<OffsetDateTime>,
}

impl From<CreateConsumptionRequest> for NewConsumptionRecord {
    fn from(value: CreateConsumptionRequest) -> Self {
        Self {
            id: None,
            member_id: super::member_id(value.member_id),
            product_id: value.product_id.into(),
            recorded_by: None,
            meal_plan_entry_id: None,
            meal_plan_component_id: None,
            slot: value.slot,
            amount: value.amount.into(),
            consumed_on: value.consumed_on,
            consumed_at: value.consumed_at,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateConsumptionRequest {
    #[serde(default)]
    pub slot: Option<MealSlot>,
    #[serde(default)]
    pub amount: Option<AmountDto>,
    #[serde(default, with = "iso_date::option")]
    #[schema(value_type = Option<String>, format = Date)]
    pub consumed_on: Option<Date>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub consumed_at: Option<OffsetDateTime>,
}

impl From<UpdateConsumptionRequest> for ConsumptionRecordPatch {
    fn from(value: UpdateConsumptionRequest) -> Self {
        Self {
            slot: value.slot,
            amount: value.amount.map(Into::into),
            consumed_on: value.consumed_on,
            consumed_at: value.consumed_at,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ProductNutritionQuery {
    pub kind: Option<AmountKindDto>,
    #[param(value_type = Option<f64>)]
    pub value: Option<Decimal>,
    pub unit: Option<Unit>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProductNutritionDto {
    pub nutrition: NutritionDto,
    pub quality: NutritionQuality,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DayTotalsDto {
    pub nutrition: NutritionDto,
    pub entry_count: i64,
    pub unknown_count: i64,
    pub partial_count: i64,
}

impl From<DayTotals> for DayTotalsDto {
    fn from(value: DayTotals) -> Self {
        Self {
            nutrition: value.nutrition.into(),
            entry_count: value.entry_count,
            unknown_count: value.unknown_count,
            partial_count: value.partial_count,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DiaryEntryDto {
    #[serde(flatten)]
    pub record: ConsumptionRecordDto,
    #[schema(example = "Tesco Whole Milk 1L")]
    pub product_name: String,
}

impl From<DiaryEntry> for DiaryEntryDto {
    fn from(value: DiaryEntry) -> Self {
        Self {
            record: value.record.into(),
            product_name: value.product_name,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DiaryDayDto {
    pub member_id: Uuid,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-08-22")]
    pub date: Date,
    pub entries: Vec<DiaryEntryDto>,
    pub totals: DayTotalsDto,
}

impl From<DiaryDay> for DiaryDayDto {
    fn from(value: DiaryDay) -> Self {
        Self {
            member_id: value.member_id.as_uuid(),
            date: value.date,
            entries: value.entries.into_iter().map(Into::into).collect(),
            totals: value.totals.into(),
        }
    }
}

pub fn consumption_id(id: Uuid) -> ConsumptionRecordId {
    ConsumptionRecordId::from(id)
}
