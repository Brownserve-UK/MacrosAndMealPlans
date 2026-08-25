use time::OffsetDateTime;

use super::{IngredientId, Provenance, Revision, Unit};
use crate::error::ValidationErrors;

pub const MAX_NAME_LEN: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Ingredient {
    pub id: IngredientId,
    pub name: String,
    pub default_unit: Unit,
    pub provenance: Provenance,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub archived_at: Option<OffsetDateTime>,
}

impl Ingredient {
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct IngredientSummary {
    pub ingredient: Ingredient,
    pub mapped_product_count: i64,
}

impl IngredientSummary {
    pub fn has_nutrition_source(&self) -> bool {
        self.mapped_product_count > 0
    }
}

#[derive(Debug, Clone)]
pub struct NewIngredient {
    pub id: Option<IngredientId>,
    pub name: String,
    pub default_unit: Unit,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Default)]
pub struct IngredientPatch {
    pub name: Option<String>,
    pub default_unit: Option<Unit>,
}

impl IngredientPatch {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.default_unit.is_none()
    }
}

pub fn validate_name(field: &str, name: &str, errors: &mut ValidationErrors) {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        errors.push(field, "Required");
    } else if trimmed.chars().count() > MAX_NAME_LEN {
        errors.push(field, "Too long");
    }
}

impl NewIngredient {
    pub fn validate(&self) -> crate::error::Result<()> {
        let mut errors = ValidationErrors::new();
        validate_name("name", &self.name, &mut errors);
        errors.into_result()
    }
}

impl IngredientPatch {
    pub fn validate(&self) -> crate::error::Result<()> {
        let mut errors = ValidationErrors::new();
        if let Some(name) = &self.name {
            validate_name("name", name, &mut errors);
        }
        errors.into_result()
    }
}

#[cfg(test)]
#[path = "ingredient_tests.rs"]
mod tests;
