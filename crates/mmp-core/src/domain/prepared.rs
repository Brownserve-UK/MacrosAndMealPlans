use rust_decimal::Decimal;
use time::OffsetDateTime;

use super::str_enum::str_enum;
use super::{
    ConsumedNutrition, MealPlanComponentId, MealPlanEntryId, PreparedBatchId, RecipeId, Revision,
    StockItemId, UserId,
};
use crate::error::{Result, ValidationErrors};

str_enum!(
    LeftoverDisposition,
    UnknownLeftoverDisposition,
    "leftover disposition"
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeftoverDisposition {
    Retain,
    Discard,
}

impl LeftoverDisposition {
    pub const ALL: [LeftoverDisposition; 2] =
        [LeftoverDisposition::Retain, LeftoverDisposition::Discard];

    pub const fn code(&self) -> &'static str {
        match self {
            LeftoverDisposition::Retain => "retain",
            LeftoverDisposition::Discard => "discard",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PreparationSource {
    Standalone,
    MealPlanComponent {
        entry_id: MealPlanEntryId,
        component_id: MealPlanComponentId,
    },
}

impl PreparationSource {
    pub const fn entry_id(&self) -> Option<MealPlanEntryId> {
        match self {
            PreparationSource::MealPlanComponent { entry_id, .. } => Some(*entry_id),
            PreparationSource::Standalone => None,
        }
    }

    pub const fn component_id(&self) -> Option<MealPlanComponentId> {
        match self {
            PreparationSource::MealPlanComponent { component_id, .. } => Some(*component_id),
            PreparationSource::Standalone => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreparedBatch {
    pub id: PreparedBatchId,
    pub recipe_id: Option<RecipeId>,
    pub source: PreparationSource,
    pub prepared_at: OffsetDateTime,
    pub servings_produced: Decimal,
    pub item_name: String,
    pub nutrition: ConsumedNutrition,
    pub created_by: UserId,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct NewPreparedBatch {
    pub recipe_id: Option<RecipeId>,
    pub source: PreparationSource,
    pub prepared_at: OffsetDateTime,
    pub servings_produced: Decimal,
    pub item_name: String,
    pub nutrition: ConsumedNutrition,
}

impl NewPreparedBatch {
    pub fn validate(&self) -> Result<()> {
        let mut errors = ValidationErrors::new();
        if self.servings_produced <= Decimal::ZERO {
            errors.push("servings_produced", "Must be more than zero");
        }
        if self.item_name.trim().is_empty() {
            errors.push("item_name", "Cannot be blank");
        }
        errors.into_result()
    }
}

#[derive(Debug, Clone)]
pub struct PreparedPortion {
    pub batch: PreparedBatch,
    pub stock_item_id: StockItemId,
}
