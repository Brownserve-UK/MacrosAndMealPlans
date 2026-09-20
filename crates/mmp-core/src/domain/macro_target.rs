use std::str::FromStr;

use rust_decimal::Decimal;

use super::str_enum::str_enum;
use crate::error::{Result, ValidationErrors};

str_enum!(
    NutritionEmphasis,
    UnknownNutritionEmphasis,
    "nutrition emphasis"
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NutritionEmphasis {
    General,
    Muscle,
    Endurance,
}

impl NutritionEmphasis {
    pub const ALL: [Self; 3] = [Self::General, Self::Muscle, Self::Endurance];

    pub const fn code(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Muscle => "muscle",
            Self::Endurance => "endurance",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MacroTargets {
    pub protein_g: Decimal,
    pub carbohydrate_g: Decimal,
    pub fat_g: Decimal,
}

impl MacroTargets {
    pub fn validate(self) -> Result<()> {
        let mut errors = ValidationErrors::new();
        for (field, value) in [
            ("protein_g", self.protein_g),
            ("carbohydrate_g", self.carbohydrate_g),
            ("fat_g", self.fat_g),
        ] {
            if value <= Decimal::ZERO {
                errors.push(field, "Must be greater than zero");
            }
        }
        errors.into_result()
    }
}

pub fn suggest_macro_targets(
    weight_kg: Decimal,
    target_energy_kcal: Decimal,
    emphasis: NutritionEmphasis,
) -> Result<MacroTargets> {
    let mut errors = ValidationErrors::new();
    if weight_kg <= Decimal::ZERO {
        errors.push("weight", "That does not look like a weight");
    }
    if target_energy_kcal <= Decimal::ZERO {
        errors.push("energy_kcal", "Must be greater than zero");
    }
    errors.into_result()?;

    let (protein_per_kg, fat_share) = match emphasis {
        NutritionEmphasis::General => (decimal("1.6"), decimal("0.28")),
        NutritionEmphasis::Muscle => (decimal("2.2"), decimal("0.25")),
        NutritionEmphasis::Endurance => (decimal("1.6"), decimal("0.22")),
    };
    let protein_g = round_to_five(weight_kg * protein_per_kg);
    let fat_g = round_to_five(target_energy_kcal * fat_share / Decimal::from(9));
    let carbohydrate_kcal =
        target_energy_kcal - protein_g * Decimal::from(4) - fat_g * Decimal::from(9);
    let carbohydrate_g =
        round_to_five((carbohydrate_kcal / Decimal::from(4)).max(Decimal::from(5)));

    let targets = MacroTargets {
        protein_g,
        carbohydrate_g,
        fat_g,
    };
    targets.validate()?;
    Ok(targets)
}

fn round_to_five(value: Decimal) -> Decimal {
    (value / Decimal::from(5)).round() * Decimal::from(5)
}

fn decimal(value: &str) -> Decimal {
    Decimal::from_str(value).expect("a valid decimal constant")
}

#[cfg(test)]
#[path = "macro_target_tests.rs"]
mod tests;
