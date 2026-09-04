use mmp_core::domain::{
    GoalProjection, HouseholdMemberId, NewWeightGoal, NewWeightRecord, Patch, Quantity,
    WeightDisplay, WeightGoal, WeightGoalId, WeightGoalPatch, WeightObjective, WeightRecord,
    WeightRecordId, WeightRecordPatch, WeightSource,
};
use mmp_core::services::{WeightPoint, WeightSummary};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use super::common::{QuantityDto, iso_date};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WeightSourceDto {
    Manual,
    HealthConnect,
}

impl From<WeightSource> for WeightSourceDto {
    fn from(value: WeightSource) -> Self {
        match value {
            WeightSource::Manual => Self::Manual,
            WeightSource::HealthConnect => Self::HealthConnect,
        }
    }
}

impl From<WeightSourceDto> for WeightSource {
    fn from(value: WeightSourceDto) -> Self {
        match value {
            WeightSourceDto::Manual => Self::Manual,
            WeightSourceDto::HealthConnect => Self::HealthConnect,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WeightObjectiveDto {
    Lose,
    Maintain,
    Gain,
}

impl From<WeightObjective> for WeightObjectiveDto {
    fn from(value: WeightObjective) -> Self {
        match value {
            WeightObjective::Lose => Self::Lose,
            WeightObjective::Maintain => Self::Maintain,
            WeightObjective::Gain => Self::Gain,
        }
    }
}

impl From<WeightObjectiveDto> for WeightObjective {
    fn from(value: WeightObjectiveDto) -> Self {
        match value {
            WeightObjectiveDto::Lose => Self::Lose,
            WeightObjectiveDto::Maintain => Self::Maintain,
            WeightObjectiveDto::Gain => Self::Gain,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WeightDisplayDto {
    Kilograms,
    StonesPounds,
    Pounds,
}

impl From<WeightDisplay> for WeightDisplayDto {
    fn from(value: WeightDisplay) -> Self {
        match value {
            WeightDisplay::Kilograms => Self::Kilograms,
            WeightDisplay::StonesPounds => Self::StonesPounds,
            WeightDisplay::Pounds => Self::Pounds,
        }
    }
}

impl From<WeightDisplayDto> for WeightDisplay {
    fn from(value: WeightDisplayDto) -> Self {
        match value {
            WeightDisplayDto::Kilograms => Self::Kilograms,
            WeightDisplayDto::StonesPounds => Self::StonesPounds,
            WeightDisplayDto::Pounds => Self::Pounds,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct WeightRecordDto {
    pub id: Uuid,
    pub member_id: Uuid,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 72.4)]
    pub weight_kg: Decimal,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-09-03")]
    pub recorded_on: Date,
    #[serde(default, with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub recorded_at: Option<OffsetDateTime>,
    pub source: WeightSourceDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_by: Option<Uuid>,
    pub revision: i64,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
}

impl From<WeightRecord> for WeightRecordDto {
    fn from(value: WeightRecord) -> Self {
        Self {
            id: value.id.as_uuid(),
            member_id: value.member_id.as_uuid(),
            weight_kg: value.weight_kg,
            recorded_on: value.recorded_on,
            recorded_at: value.recorded_at,
            source: value.source.into(),
            recorded_by: value.recorded_by.map(|id| id.as_uuid()),
            revision: value.revision.get(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
pub struct CreateWeightRecordRequest {
    pub weight: QuantityDto,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-09-03")]
    pub recorded_on: Date,
    #[serde(default, with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub recorded_at: Option<OffsetDateTime>,
}

impl CreateWeightRecordRequest {
    pub fn into_domain(
        self,
        member_id: HouseholdMemberId,
        recorded_by: Option<mmp_core::domain::UserId>,
    ) -> NewWeightRecord {
        NewWeightRecord {
            member_id,
            weight: self.weight.into(),
            recorded_on: self.recorded_on,
            recorded_at: self.recorded_at,
            source: WeightSource::Manual,
            recorded_by,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateWeightRecordRequest {
    #[serde(default)]
    pub weight: Option<QuantityDto>,
    #[serde(default, with = "iso_date::option")]
    #[schema(value_type = Option<String>, format = Date)]
    pub recorded_on: Option<Date>,
    #[serde(default)]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub recorded_at: Patch<OffsetDateTime>,
}

impl From<UpdateWeightRecordRequest> for WeightRecordPatch {
    fn from(value: UpdateWeightRecordRequest) -> Self {
        WeightRecordPatch {
            weight: value.weight.map(Quantity::from),
            recorded_on: value.recorded_on,
            recorded_at: value.recorded_at,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct WeightGoalDto {
    pub id: Uuid,
    pub member_id: Uuid,
    pub objective: WeightObjectiveDto,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 80.0)]
    pub starting_weight_kg: Decimal,
    #[serde(
        with = "rust_decimal::serde::float_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<f64>, example = 76.0)]
    pub target_weight_kg: Option<Decimal>,
    #[serde(
        with = "rust_decimal::serde::float_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<f64>, example = 0.5)]
    pub planned_rate_kg_per_week: Option<Decimal>,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-08-01")]
    pub started_on: Date,
    pub revision: i64,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
}

impl From<WeightGoal> for WeightGoalDto {
    fn from(value: WeightGoal) -> Self {
        Self {
            id: value.id.as_uuid(),
            member_id: value.member_id.as_uuid(),
            objective: value.objective.into(),
            starting_weight_kg: value.starting_weight_kg,
            target_weight_kg: value.target_weight_kg,
            planned_rate_kg_per_week: value.planned_rate_kg_per_week,
            started_on: value.started_on,
            revision: value.revision.get(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
pub struct CreateWeightGoalRequest {
    pub objective: WeightObjectiveDto,
    pub starting_weight: QuantityDto,
    #[serde(default)]
    pub target_weight: Option<QuantityDto>,
    #[serde(default)]
    pub planned_rate: Option<QuantityDto>,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-08-01")]
    pub started_on: Date,
}

impl CreateWeightGoalRequest {
    pub fn into_domain(self, member_id: HouseholdMemberId) -> NewWeightGoal {
        NewWeightGoal {
            member_id,
            objective: self.objective.into(),
            starting_weight: self.starting_weight.into(),
            target_weight: self.target_weight.map(Quantity::from),
            planned_rate: self.planned_rate.map(Quantity::from),
            started_on: self.started_on,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateWeightGoalRequest {
    #[serde(default)]
    pub objective: Option<WeightObjectiveDto>,
    #[serde(default)]
    pub starting_weight: Option<QuantityDto>,
    #[serde(default)]
    #[schema(value_type = Option<QuantityDto>)]
    pub target_weight: Patch<QuantityDto>,
    #[serde(default)]
    #[schema(value_type = Option<QuantityDto>)]
    pub planned_rate: Patch<QuantityDto>,
    #[serde(default, with = "iso_date::option")]
    #[schema(value_type = Option<String>, format = Date)]
    pub started_on: Option<Date>,
}

impl From<UpdateWeightGoalRequest> for WeightGoalPatch {
    fn from(value: UpdateWeightGoalRequest) -> Self {
        WeightGoalPatch {
            objective: value.objective.map(Into::into),
            starting_weight: value.starting_weight.map(Quantity::from),
            target_weight: value.target_weight.map(Quantity::from),
            planned_rate: value.planned_rate.map(Quantity::from),
            started_on: value.started_on,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GoalProjectionDto {
    Reached,
    Steady,
    Projected {
        #[serde(with = "iso_date")]
        #[schema(value_type = String, format = Date, example = "2026-12-01")]
        on: Date,
        #[serde(with = "rust_decimal::serde::float")]
        #[schema(value_type = f64, example = 2.0)]
        remaining_kg: Decimal,
    },
}

impl From<GoalProjection> for GoalProjectionDto {
    fn from(value: GoalProjection) -> Self {
        match value {
            GoalProjection::Reached => Self::Reached,
            GoalProjection::Steady => Self::Steady,
            GoalProjection::Projected { on, remaining_kg } => Self::Projected { on, remaining_kg },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct WeightPointDto {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-09-03")]
    pub on: Date,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 72.4)]
    pub weight_kg: Decimal,
}

impl From<WeightPoint> for WeightPointDto {
    fn from(value: WeightPoint) -> Self {
        Self {
            on: value.on,
            weight_kg: value.weight_kg,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WeightSummaryDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest: Option<WeightRecordDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goal: Option<WeightGoalDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projection: Option<GoalProjectionDto>,
    #[serde(
        with = "rust_decimal::serde::float_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<f64>, example = -2.0)]
    pub change_since_start_kg: Option<Decimal>,
    pub series: Vec<WeightPointDto>,
}

impl From<WeightSummary> for WeightSummaryDto {
    fn from(value: WeightSummary) -> Self {
        Self {
            latest: value.latest.map(Into::into),
            goal: value.goal.map(Into::into),
            projection: value.projection.map(Into::into),
            change_since_start_kg: value.change_since_start_kg,
            series: value.series.into_iter().map(Into::into).collect(),
        }
    }
}

pub fn weight_record_id(id: Uuid) -> WeightRecordId {
    WeightRecordId::from(id)
}

pub fn weight_goal_id(id: Uuid) -> WeightGoalId {
    WeightGoalId::from(id)
}
