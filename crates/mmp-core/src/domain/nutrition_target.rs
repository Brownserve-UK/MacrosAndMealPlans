use rust_decimal::Decimal;
use time::{Date, OffsetDateTime};

use super::str_enum::str_enum;
use super::{
    HouseholdMemberId, NutritionFacts, NutritionTargetId, Patch, Revision, WeightObjective,
};
use crate::error::{Result, ValidationErrors};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetDirection {
    AtLeast,
    AtMost,
    Around,
}

str_enum!(TargetSource, UnknownTargetSource, "target source");

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetSource {
    UserDefined,
    Calculated,
}

impl TargetSource {
    pub const ALL: [TargetSource; 2] = [TargetSource::UserDefined, TargetSource::Calculated];

    pub const fn code(&self) -> &'static str {
        match self {
            TargetSource::UserDefined => "user_defined",
            TargetSource::Calculated => "calculated",
        }
    }
}

pub fn direction_for(nutrient: &str, objective: WeightObjective) -> TargetDirection {
    match nutrient {
        "energy_kcal" => match objective {
            WeightObjective::Gain => TargetDirection::AtLeast,
            WeightObjective::Maintain => TargetDirection::Around,
            WeightObjective::Lose => TargetDirection::AtMost,
        },
        "protein_g" | "fibre_g" => TargetDirection::AtLeast,
        "carbohydrate_g" | "fat_g" => TargetDirection::Around,
        _ => TargetDirection::AtMost,
    }
}

pub fn default_direction_for(nutrient: &str) -> TargetDirection {
    direction_for(nutrient, WeightObjective::Lose)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NutritionGoals {
    pub energy_kcal: Option<Decimal>,
    pub protein_g: Option<Decimal>,
    pub carbohydrate_g: Option<Decimal>,
    pub sugar_g: Option<Decimal>,
    pub fat_g: Option<Decimal>,
    pub saturated_fat_g: Option<Decimal>,
    pub fibre_g: Option<Decimal>,
    pub salt_g: Option<Decimal>,
    pub cholesterol_mg: Option<Decimal>,
}

impl NutritionGoals {
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

    pub fn is_empty(&self) -> bool {
        self.named_values().all(|(_, value)| value.is_none())
    }

    pub fn get(&self, nutrient: &str) -> Option<Decimal> {
        self.named_values()
            .find(|(name, _)| *name == nutrient)
            .and_then(|(_, value)| value)
    }

    pub fn set(&mut self, nutrient: &str, value: Option<Decimal>) {
        match nutrient {
            "energy_kcal" => self.energy_kcal = value,
            "protein_g" => self.protein_g = value,
            "carbohydrate_g" => self.carbohydrate_g = value,
            "sugar_g" => self.sugar_g = value,
            "fat_g" => self.fat_g = value,
            "saturated_fat_g" => self.saturated_fat_g = value,
            "fibre_g" => self.fibre_g = value,
            "salt_g" => self.salt_g = value,
            "cholesterol_mg" => self.cholesterol_mg = value,
            _ => {}
        }
    }

    pub fn validate(&self, prefix: &str, errors: &mut ValidationErrors) {
        for (name, value) in self.named_values() {
            if let Some(value) = value
                && value.is_sign_negative()
            {
                errors.push(format!("{prefix}.{name}"), "Cannot be negative");
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct NutritionGoalsPatch {
    pub energy_kcal: Patch<Decimal>,
    pub protein_g: Patch<Decimal>,
    pub carbohydrate_g: Patch<Decimal>,
    pub sugar_g: Patch<Decimal>,
    pub fat_g: Patch<Decimal>,
    pub saturated_fat_g: Patch<Decimal>,
    pub fibre_g: Patch<Decimal>,
    pub salt_g: Patch<Decimal>,
    pub cholesterol_mg: Patch<Decimal>,
}

impl NutritionGoalsPatch {
    pub fn is_empty(&self) -> bool {
        self.energy_kcal.is_unchanged()
            && self.protein_g.is_unchanged()
            && self.carbohydrate_g.is_unchanged()
            && self.sugar_g.is_unchanged()
            && self.fat_g.is_unchanged()
            && self.saturated_fat_g.is_unchanged()
            && self.fibre_g.is_unchanged()
            && self.salt_g.is_unchanged()
            && self.cholesterol_mg.is_unchanged()
    }

    pub fn apply(self, goals: NutritionGoals) -> NutritionGoals {
        NutritionGoals {
            energy_kcal: self.energy_kcal.apply(goals.energy_kcal),
            protein_g: self.protein_g.apply(goals.protein_g),
            carbohydrate_g: self.carbohydrate_g.apply(goals.carbohydrate_g),
            sugar_g: self.sugar_g.apply(goals.sugar_g),
            fat_g: self.fat_g.apply(goals.fat_g),
            saturated_fat_g: self.saturated_fat_g.apply(goals.saturated_fat_g),
            fibre_g: self.fibre_g.apply(goals.fibre_g),
            salt_g: self.salt_g.apply(goals.salt_g),
            cholesterol_mg: self.cholesterol_mg.apply(goals.cholesterol_mg),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NutritionTarget {
    pub id: NutritionTargetId,
    pub member_id: HouseholdMemberId,
    pub effective_from: Date,
    pub source: TargetSource,
    pub goals: NutritionGoals,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct NewNutritionTarget {
    pub member_id: HouseholdMemberId,
    pub effective_from: Date,
    pub goals: NutritionGoals,
}

impl NewNutritionTarget {
    pub fn validate(&self) -> Result<()> {
        let mut errors = ValidationErrors::new();
        validate_goals(&self.goals, &mut errors);
        errors.into_result()
    }
}

#[derive(Debug, Clone, Default)]
pub struct NutritionTargetPatch {
    pub effective_from: Option<Date>,
    pub goals: NutritionGoalsPatch,
}

impl NutritionTargetPatch {
    pub fn is_empty(&self) -> bool {
        self.effective_from.is_none() && self.goals.is_empty()
    }
}

pub fn validate_goals(goals: &NutritionGoals, errors: &mut ValidationErrors) {
    goals.validate("goals", errors);
    if goals.is_empty() {
        errors.push("goals", "Set at least one target");
    }
}

pub fn resolve_on(targets: &[NutritionTarget], date: Date) -> Option<&NutritionTarget> {
    targets
        .iter()
        .filter(|target| target.effective_from <= date)
        .max_by_key(|target| target.effective_from)
}

pub const NUTRIENT_KEYS: [&str; 9] = NutritionFacts::NAMED_NUTRIENTS;

#[cfg(test)]
#[path = "nutrition_target_tests.rs"]
mod tests;
