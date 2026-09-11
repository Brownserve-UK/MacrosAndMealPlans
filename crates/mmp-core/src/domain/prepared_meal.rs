use time::OffsetDateTime;

use super::{PreparedMealId, Provenance, Revision, ShoppingSection, Unit};
use crate::domain::ingredient::validate_name;
use crate::error::ValidationErrors;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PreparedMeal {
    pub id: PreparedMealId,
    pub name: String,
    pub default_unit: Unit,
    pub shopping_section: Option<ShoppingSection>,
    pub track_stock: Option<bool>,
    pub provenance: Provenance,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub archived_at: Option<OffsetDateTime>,
}

impl PreparedMeal {
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct PreparedMealSummary {
    pub prepared_meal: PreparedMeal,
    pub mapped_product_count: i64,
}

impl PreparedMealSummary {
    pub fn has_nutrition_source(&self) -> bool {
        self.mapped_product_count > 0
    }
}

#[derive(Debug, Clone)]
pub struct NewPreparedMeal {
    pub id: Option<PreparedMealId>,
    pub name: String,
    pub default_unit: Unit,
    pub shopping_section: Option<ShoppingSection>,
    pub track_stock: Option<bool>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Default)]
pub struct PreparedMealPatch {
    pub name: Option<String>,
    pub default_unit: Option<Unit>,
    pub shopping_section: super::Patch<ShoppingSection>,
    pub track_stock: super::Patch<bool>,
}

impl PreparedMealPatch {
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.default_unit.is_none()
            && self.shopping_section.is_unchanged()
            && self.track_stock.is_unchanged()
    }
}

impl NewPreparedMeal {
    pub fn validate(&self) -> crate::error::Result<()> {
        let mut errors = ValidationErrors::new();
        validate_name("name", &self.name, &mut errors);
        errors.into_result()
    }
}

impl PreparedMealPatch {
    pub fn validate(&self) -> crate::error::Result<()> {
        let mut errors = ValidationErrors::new();
        if let Some(name) = &self.name {
            validate_name("name", name, &mut errors);
        }
        errors.into_result()
    }
}

#[cfg(test)]
#[path = "prepared_meal_tests.rs"]
mod tests;
