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
mod tests {
    use super::*;
    use crate::domain::Unit;

    fn per_100g() -> Quantity {
        Quantity::new(Decimal::new(100, 0), Unit::Gram)
    }

    #[test]
    fn empty_facts_are_unknown() {
        assert!(NutritionFacts::default().is_unknown());
    }

    #[test]
    fn a_zero_value_is_known_not_unknown() {
        let facts = NutritionFacts {
            basis: Some(per_100g()),
            fat_g: Some(Decimal::ZERO),
            ..Default::default()
        };
        assert!(!facts.is_unknown());
    }

    #[test]
    fn negative_nutrients_are_rejected() {
        let facts = NutritionFacts {
            basis: Some(per_100g()),
            protein_g: Some(Decimal::new(-1, 0)),
            ..Default::default()
        };
        let mut errors = ValidationErrors::new();
        facts.validate("nutrition", &mut errors);
        assert!(!errors.is_empty());
    }

    #[test]
    fn a_basis_is_required_once_any_value_is_supplied() {
        let facts = NutritionFacts {
            energy_kcal: Some(Decimal::new(250, 0)),
            ..Default::default()
        };
        let mut errors = ValidationErrors::new();
        facts.validate("nutrition", &mut errors);
        assert!(errors.iter().any(|e| e.field == "nutrition.basis"));
    }

    #[test]
    fn a_basis_of_zero_is_rejected() {
        let facts = NutritionFacts {
            basis: Some(Quantity::new(Decimal::ZERO, Unit::Gram)),
            energy_kcal: Some(Decimal::new(250, 0)),
            ..Default::default()
        };
        let mut errors = ValidationErrors::new();
        facts.validate("nutrition", &mut errors);
        assert!(errors.iter().any(|e| e.field == "nutrition.basis.amount"));
    }

    #[test]
    fn a_serving_sized_basis_is_expressible() {
        let facts = NutritionFacts {
            basis: Some(Quantity::new(Decimal::new(30, 0), Unit::Gram)),
            energy_kcal: Some(Decimal::new(120, 0)),
            ..Default::default()
        };
        let mut errors = ValidationErrors::new();
        facts.validate("nutrition", &mut errors);
        assert!(
            errors.is_empty(),
            "a per-serving label must be representable"
        );
    }

    #[test]
    fn wholly_unknown_nutrition_needs_no_basis() {
        let mut errors = ValidationErrors::new();
        NutritionFacts::default().validate("nutrition", &mut errors);
        assert!(errors.is_empty());
    }

    #[test]
    fn named_nutrient_list_matches_the_accessors() {
        let facts = NutritionFacts::default();
        let names: Vec<&str> = facts.named_values().map(|(name, _)| name).collect();
        assert_eq!(names, NutritionFacts::NAMED_NUTRIENTS.to_vec());
    }
}
