use mmp_core::domain::{
    CalorieCalculation, MacroTargets, NutritionEmphasis, Pace, Quantity, direction_for,
};
use mmp_core::services::{
    GuidedNutritionPlan, NutritionPlan, NutritionPlanAnswers, NutritionPlanRecommendation,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use super::common::iso_date;
use super::{
    HabitualActivityDto, MemberBodyProfileDto, NutritionTargetDto, QuantityDto, SexDto,
    TargetDirectionDto, WeightGoalDto, WeightObjectiveDto, WeightRecordDto,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaceDto {
    Steady,
    Standard,
    Faster,
    Fastest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NutritionEmphasisDto {
    General,
    Muscle,
    Endurance,
}

impl From<NutritionEmphasis> for NutritionEmphasisDto {
    fn from(value: NutritionEmphasis) -> Self {
        match value {
            NutritionEmphasis::General => Self::General,
            NutritionEmphasis::Muscle => Self::Muscle,
            NutritionEmphasis::Endurance => Self::Endurance,
        }
    }
}

impl From<NutritionEmphasisDto> for NutritionEmphasis {
    fn from(value: NutritionEmphasisDto) -> Self {
        match value {
            NutritionEmphasisDto::General => Self::General,
            NutritionEmphasisDto::Muscle => Self::Muscle,
            NutritionEmphasisDto::Endurance => Self::Endurance,
        }
    }
}

impl From<Pace> for PaceDto {
    fn from(value: Pace) -> Self {
        match value {
            Pace::Steady => Self::Steady,
            Pace::Standard => Self::Standard,
            Pace::Faster => Self::Faster,
            Pace::Fastest => Self::Fastest,
        }
    }
}

impl From<PaceDto> for Pace {
    fn from(value: PaceDto) -> Self {
        match value {
            PaceDto::Steady => Self::Steady,
            PaceDto::Standard => Self::Standard,
            PaceDto::Faster => Self::Faster,
            PaceDto::Fastest => Self::Fastest,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalorieCalculationDto {
    pub id: Uuid,
    pub member_id: Uuid,
    pub nutrition_target_id: Uuid,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-09-12")]
    pub calculated_on: Date,
    pub formula: String,
    pub activity_source: String,
    pub habitual_activity: HabitualActivityDto,
    pub age_years: i32,
    pub sex: SexDto,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 180.0)]
    pub height_cm: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 80.0)]
    pub weight_kg: Decimal,
    pub objective: WeightObjectiveDto,
    pub emphasis: NutritionEmphasisDto,
    #[serde(
        with = "rust_decimal::serde::float_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<f64>, example = 0.5)]
    pub requested_rate_kg_per_week: Option<Decimal>,
    #[serde(
        with = "rust_decimal::serde::float_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<f64>, example = 0.5)]
    pub applied_rate_kg_per_week: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 2480.0)]
    pub maintenance_kcal: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = -550.0)]
    pub adjustment_kcal: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 1930.0)]
    pub recommended_kcal: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 1500.0)]
    pub floor_kcal: Decimal,
    pub eased: bool,
    pub revision: i64,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
}

impl From<CalorieCalculation> for CalorieCalculationDto {
    fn from(value: CalorieCalculation) -> Self {
        Self {
            id: value.id.as_uuid(),
            member_id: value.member_id.as_uuid(),
            nutrition_target_id: value.nutrition_target_id.as_uuid(),
            calculated_on: value.calculated_on,
            formula: value.formula,
            activity_source: value.activity_source,
            habitual_activity: value.habitual_activity.into(),
            age_years: value.age_years,
            sex: value.sex.into(),
            height_cm: value.height_cm,
            weight_kg: value.weight_kg,
            objective: value.objective.into(),
            emphasis: value.emphasis.into(),
            requested_rate_kg_per_week: value.requested_rate_kg_per_week,
            applied_rate_kg_per_week: value.applied_rate_kg_per_week,
            maintenance_kcal: value.maintenance_kcal,
            adjustment_kcal: value.adjustment_kcal,
            recommended_kcal: value.recommended_kcal,
            floor_kcal: value.floor_kcal,
            eased: value.eased,
            revision: value.revision.get(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NutritionPlanAnswersRequest {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "1986-01-01")]
    pub date_of_birth: Date,
    pub sex: SexDto,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 180.0)]
    pub height_cm: Decimal,
    pub current_weight: QuantityDto,
    pub habitual_activity: HabitualActivityDto,
    pub objective: WeightObjectiveDto,
    pub emphasis: NutritionEmphasisDto,
    #[serde(default)]
    pub target_weight: Option<QuantityDto>,
    #[serde(default)]
    pub pace: Option<PaceDto>,
}

impl NutritionPlanAnswersRequest {
    pub fn into_domain(
        self,
        member_id: mmp_core::domain::HouseholdMemberId,
        recorded_by: mmp_core::domain::UserId,
    ) -> NutritionPlanAnswers {
        NutritionPlanAnswers {
            member_id,
            date_of_birth: self.date_of_birth,
            sex: self.sex.into(),
            height_cm: self.height_cm,
            current_weight: Quantity::from(self.current_weight),
            habitual_activity: self.habitual_activity.into(),
            objective: self.objective.into(),
            emphasis: self.emphasis.into(),
            target_weight: self.target_weight.map(Quantity::from),
            pace: self.pace.map(Into::into),
            recorded_by: Some(recorded_by),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
pub struct ManualCalorieTargetRequest {
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 1800.0)]
    pub energy_kcal: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 140.0)]
    pub protein_g: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 220.0)]
    pub carbohydrate_g: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 65.0)]
    pub fat_g: Decimal,
}

impl ManualCalorieTargetRequest {
    pub fn macros(self) -> MacroTargets {
        MacroTargets {
            protein_g: self.protein_g,
            carbohydrate_g: self.carbohydrate_g,
            fat_g: self.fat_g,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct MacroTargetsDto {
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 140.0)]
    pub protein_g: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 220.0)]
    pub carbohydrate_g: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 65.0)]
    pub fat_g: Decimal,
}

impl From<MacroTargets> for MacroTargetsDto {
    fn from(value: MacroTargets) -> Self {
        Self {
            protein_g: value.protein_g,
            carbohydrate_g: value.carbohydrate_g,
            fat_g: value.fat_g,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NutritionPlanRecommendationDto {
    pub calculation: CalorieCalculationDto,
    pub macros: MacroTargetsDto,
    #[serde(with = "iso_date::option", skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>, format = Date)]
    pub estimated_goal_date: Option<Date>,
}

impl From<NutritionPlanRecommendation> for NutritionPlanRecommendationDto {
    fn from(value: NutritionPlanRecommendation) -> Self {
        Self {
            calculation: value.calculation.into(),
            macros: value.macros.into(),
            estimated_goal_date: value.estimated_goal_date,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NutritionPlanDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<NutritionTargetDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calculation: Option<CalorieCalculationDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calorie_direction: Option<TargetDirectionDto>,
}

impl From<NutritionPlan> for NutritionPlanDto {
    fn from(value: NutritionPlan) -> Self {
        let calorie_direction = value
            .target
            .as_ref()
            .filter(|target| target.goals.energy_kcal.is_some())
            .zip(value.objective)
            .map(|(_, objective)| direction_for("energy_kcal", objective).into());
        Self {
            target: value.target.map(Into::into),
            calculation: value.calculation.map(Into::into),
            calorie_direction,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GuidedNutritionPlanDto {
    pub profile: MemberBodyProfileDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight_record: Option<WeightRecordDto>,
    pub goal: WeightGoalDto,
    pub target: NutritionTargetDto,
    pub calculation: CalorieCalculationDto,
}

impl From<GuidedNutritionPlan> for GuidedNutritionPlanDto {
    fn from(value: GuidedNutritionPlan) -> Self {
        Self {
            profile: value.profile.into(),
            weight_record: value.weight_record.map(Into::into),
            goal: value.goal.into(),
            target: value.target.into(),
            calculation: value.calculation.into(),
        }
    }
}
