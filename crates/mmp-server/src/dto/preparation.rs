use mmp_core::domain::{
    MealPlanComponentId, MealPlanEntryId, NutritionQuality, PortionPlacement, PreparationSource,
    PreparedBatch,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::common::iso_date;
use super::{NutritionDto, StorageLocationDto, UsabilityDeadlineDto};

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PreparationRangeQuery {
    #[serde(with = "iso_date")]
    #[param(value_type = String, format = Date, example = "2026-09-01")]
    pub from: Date,
    #[serde(with = "iso_date")]
    #[param(value_type = String, format = Date, example = "2026-09-07")]
    pub to: Date,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PreparedBatchDto {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipe_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meal_plan_entry_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meal_plan_component_id: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub prepared_at: OffsetDateTime,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64)]
    pub servings_produced: Decimal,
    pub item_name: String,
    pub nutrition: NutritionDto,
    pub nutrition_quality: NutritionQuality,
    pub revision: i64,
}

impl From<PreparedBatch> for PreparedBatchDto {
    fn from(value: PreparedBatch) -> Self {
        Self {
            id: value.id.as_uuid(),
            recipe_id: value.recipe_id.map(|id| id.as_uuid()),
            meal_plan_entry_id: value.source.entry_id().map(|id| id.as_uuid()),
            meal_plan_component_id: value.source.component_id().map(|id| id.as_uuid()),
            prepared_at: value.prepared_at,
            servings_produced: value.servings_produced,
            item_name: value.item_name,
            nutrition: value.nutrition.facts.into(),
            nutrition_quality: value.nutrition.quality,
            revision: value.revision.get(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PortionPlacementRequest {
    pub storage_location: StorageLocationDto,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64)]
    pub servings: Decimal,
    #[serde(default)]
    pub usability_deadline: Option<UsabilityDeadlineDto>,
    #[serde(default)]
    pub note: Option<String>,
}

impl From<PortionPlacementRequest> for PortionPlacement {
    fn from(value: PortionPlacementRequest) -> Self {
        Self {
            storage_location: value.storage_location.into(),
            servings: value.servings,
            usability_deadline: value.usability_deadline.map(Into::into),
            note: value.note,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RecordPreparationRequest {
    pub recipe_id: Uuid,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64)]
    pub servings_produced: Decimal,
    pub placements: Vec<PortionPlacementRequest>,
    #[serde(default)]
    pub meal_plan_entry_id: Option<Uuid>,
    #[serde(default)]
    pub meal_plan_component_id: Option<Uuid>,
}

impl RecordPreparationRequest {
    pub fn source(&self) -> PreparationSource {
        match (self.meal_plan_entry_id, self.meal_plan_component_id) {
            (Some(entry_id), Some(component_id)) => PreparationSource::MealPlanComponent {
                entry_id: MealPlanEntryId::from(entry_id),
                component_id: MealPlanComponentId::from(component_id),
            },
            _ => PreparationSource::Standalone,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PlacePortionsRequest {
    pub placements: Vec<PortionPlacementRequest>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MoveCookedFoodRequest {
    pub from: StorageLocationDto,
    pub to: StorageLocationDto,
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64)]
    pub servings: Decimal,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PreparationResponse {
    #[serde(flatten)]
    pub batch: PreparedBatchDto,
    #[serde(default)]
    pub stock_outcomes: Vec<super::StockOutcomeDto>,
}
