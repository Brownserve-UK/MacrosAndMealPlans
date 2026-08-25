use std::collections::BTreeMap;

use rust_decimal::Decimal;

use super::Quantity;
use crate::error::ValidationErrors;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NutritionFacts {
    pub basis: Option<Quantity>,
    pub energy_kcal: Option<Decimal>,
    pub protein_g: Option<Decimal>,
    pub carbohydrate_g: Option<Decimal>,
    pub sugar_g: Option<Decimal>,
    pub fat_g: Option<Decimal>,
    pub saturated_fat_g: Option<Decimal>,
    pub fibre_g: Option<Decimal>,
    pub salt_g: Option<Decimal>,
    pub cholesterol_mg: Option<Decimal>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Decimal>,
}

impl NutritionFacts {
    pub const NAMED_NUTRIENTS: [&'static str; 9] = [
        "energy_kcal",
        "protein_g",
        "carbohydrate_g",
        "sugar_g",
        "fat_g",
        "saturated_fat_g",
        "fibre_g",
        "salt_g",
        "cholesterol_mg",
    ];

    pub fn is_unknown(&self) -> bool {
        self.named_values().all(|(_, value)| value.is_none()) && self.extra.is_empty()
    }

    pub fn named_values(&self) -> impl Iterator<Item = (&'static str, Option<Decimal>)> {
        [
            ("energy_kcal", self.energy_kcal),
            ("protein_g", self.protein_g),
            ("carbohydrate_g", self.carbohydrate_g),
            ("sugar_g", self.sugar_g),
            ("fat_g", self.fat_g),
            ("saturated_fat_g", self.saturated_fat_g),
            ("fibre_g", self.fibre_g),
            ("salt_g", self.salt_g),
            ("cholesterol_mg", self.cholesterol_mg),
        ]
        .into_iter()
    }

    pub fn validate(&self, prefix: &str, errors: &mut ValidationErrors) {
        for (name, value) in self.named_values() {
            if let Some(value) = value
                && value.is_sign_negative()
            {
                errors.push(format!("{prefix}.{name}"), "Cannot be negative");
            }
        }

        for (name, value) in &self.extra {
            if value.is_sign_negative() {
                errors.push(format!("{prefix}.extra.{name}"), "Cannot be negative");
            }
        }

        match self.basis {
            None if !self.is_unknown() => errors.push(format!("{prefix}.basis"), "Required"),
            Some(basis) if basis.amount <= Decimal::ZERO => {
                errors.push(format!("{prefix}.basis.amount"), "Must be more than zero")
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "nutrition_tests.rs"]
mod tests;
