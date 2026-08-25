use std::collections::BTreeMap;

use mmp_core::domain::{
    HouseholdMemberId, NUTRIENT_KEYS, NewNutritionTarget, NutritionGoals, NutritionGoalsPatch,
    NutritionTarget, NutritionTargetId, NutritionTargetPatch, Patch, TargetDirection,
    direction_for,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use super::common::iso_date;

macro_rules! goal_fields {
    ($($field:ident),* $(,)?) => {
        #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
        pub struct NutritionGoalsDto {
            $(
                #[serde(default, with = "rust_decimal::serde::float_option")]
                #[schema(value_type = Option<f64>)]
                pub $field: Option<Decimal>,
            )*
        }

        impl From<NutritionGoals> for NutritionGoalsDto {
            fn from(goals: NutritionGoals) -> Self {
                Self { $($field: goals.$field,)* }
            }
        }

        impl From<NutritionGoalsDto> for NutritionGoals {
            fn from(dto: NutritionGoalsDto) -> Self {
                Self { $($field: dto.$field,)* }
            }
        }
    };
}

goal_fields!(
    energy_kcal,
    protein_g,
    carbohydrate_g,
    sugar_g,
    fat_g,
    saturated_fat_g,
    fibre_g,
    salt_g,
    cholesterol_mg,
);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TargetDirectionDto {
    AtLeast,
    AtMost,
    Around,
}

impl From<TargetDirection> for TargetDirectionDto {
    fn from(value: TargetDirection) -> Self {
        match value {
            TargetDirection::AtLeast => TargetDirectionDto::AtLeast,
            TargetDirection::AtMost => TargetDirectionDto::AtMost,
            TargetDirection::Around => TargetDirectionDto::Around,
        }
    }
}

pub fn nutrient_directions() -> BTreeMap<String, TargetDirectionDto> {
    NUTRIENT_KEYS
        .into_iter()
        .map(|key| (key.to_owned(), direction_for(key).into()))
        .collect()
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NutritionTargetDto {
    pub id: Uuid,
    pub member_id: Uuid,
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-08-25")]
    pub effective_from: Date,
    #[serde(flatten)]
    pub goals: NutritionGoalsDto,
    pub revision: i64,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
}

impl From<NutritionTarget> for NutritionTargetDto {
    fn from(value: NutritionTarget) -> Self {
        Self {
            id: value.id.as_uuid(),
            member_id: value.member_id.as_uuid(),
            effective_from: value.effective_from,
            goals: value.goals.into(),
            revision: value.revision.get(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateNutritionTargetRequest {
    #[serde(with = "iso_date")]
    #[schema(value_type = String, format = Date, example = "2026-08-25")]
    pub effective_from: Date,
    #[serde(flatten)]
    pub goals: NutritionGoalsDto,
}

impl CreateNutritionTargetRequest {
    pub fn into_domain(self, member_id: HouseholdMemberId) -> NewNutritionTarget {
        NewNutritionTarget {
            member_id,
            effective_from: self.effective_from,
            goals: self.goals.into(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateNutritionTargetRequest {
    #[serde(default, with = "iso_date::option")]
    #[schema(value_type = Option<String>, format = Date)]
    pub effective_from: Option<Date>,
    #[serde(default)]
    #[schema(value_type = Option<f64>)]
    pub energy_kcal: Patch<f64>,
    #[serde(default)]
    #[schema(value_type = Option<f64>)]
    pub protein_g: Patch<f64>,
    #[serde(default)]
    #[schema(value_type = Option<f64>)]
    pub carbohydrate_g: Patch<f64>,
    #[serde(default)]
    #[schema(value_type = Option<f64>)]
    pub sugar_g: Patch<f64>,
    #[serde(default)]
    #[schema(value_type = Option<f64>)]
    pub fat_g: Patch<f64>,
    #[serde(default)]
    #[schema(value_type = Option<f64>)]
    pub saturated_fat_g: Patch<f64>,
    #[serde(default)]
    #[schema(value_type = Option<f64>)]
    pub fibre_g: Patch<f64>,
    #[serde(default)]
    #[schema(value_type = Option<f64>)]
    pub salt_g: Patch<f64>,
    #[serde(default)]
    #[schema(value_type = Option<f64>)]
    pub cholesterol_mg: Patch<f64>,
}

impl From<UpdateNutritionTargetRequest> for NutritionTargetPatch {
    fn from(value: UpdateNutritionTargetRequest) -> Self {
        NutritionTargetPatch {
            effective_from: value.effective_from,
            goals: NutritionGoalsPatch {
                energy_kcal: to_decimal(value.energy_kcal),
                protein_g: to_decimal(value.protein_g),
                carbohydrate_g: to_decimal(value.carbohydrate_g),
                sugar_g: to_decimal(value.sugar_g),
                fat_g: to_decimal(value.fat_g),
                saturated_fat_g: to_decimal(value.saturated_fat_g),
                fibre_g: to_decimal(value.fibre_g),
                salt_g: to_decimal(value.salt_g),
                cholesterol_mg: to_decimal(value.cholesterol_mg),
            },
        }
    }
}

fn to_decimal(patch: Patch<f64>) -> Patch<Decimal> {
    patch.map(|value| Decimal::from_f64(value).unwrap_or(Decimal::ZERO))
}

pub fn nutrition_target_id(id: Uuid) -> NutritionTargetId {
    NutritionTargetId::from(id)
}
