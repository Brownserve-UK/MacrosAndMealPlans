use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;
use time::OffsetDateTime;

use super::{
    ConsumedAmount, ConsumedNutrition, NutritionFacts, NutritionQuality, Patch, Product, ProductId,
    RecipeComponentId, RecipeId, RecipeInstructionId, Revision, UserId, nutrition_for,
    sum_nutrition, validate_name,
};
use crate::error::ValidationErrors;

pub const MAX_SERVINGS: i32 = 10_000;
pub const MAX_RECIPE_MINUTES: i32 = 10_080;
pub const MAX_INSTRUCTIONS: usize = 100;
pub const MAX_TAGS: usize = 50;

const ISO_COUNTRY_CODES: &str = "AD AE AF AG AI AL AM AO AQ AR AS AT AU AW AX AZ BA BB BD BE BF BG BH BI BJ BL BM BN BO BQ BR BS BT BV BW BY BZ CA CC CD CF CG CH CI CK CL CM CN CO CR CU CV CW CX CY CZ DE DJ DK DM DO DZ EC EE EG EH ER ES ET FI FJ FK FM FO FR GA GB GD GE GF GG GH GI GL GM GN GP GQ GR GS GT GU GW GY HK HM HN HR HT HU ID IE IL IM IN IO IQ IR IS IT JE JM JO JP KE KG KH KI KM KN KP KR KW KY KZ LA LB LC LI LK LR LS LT LU LV LY MA MC MD ME MF MG MH MK ML MM MN MO MP MQ MR MS MT MU MV MW MX MY MZ NA NC NE NF NG NI NL NO NP NR NU NZ OM PA PE PF PG PH PK PL PM PN PR PS PT PW PY QA RE RO RS RU RW SA SB SC SD SE SG SH SI SJ SK SL SM SN SO SR SS ST SV SX SY SZ TC TD TF TG TH TJ TK TL TM TN TO TR TT TV TW TZ UA UG UM US UY UZ VA VC VE VG VI VN VU WF WS YE YT ZA ZM ZW";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum RecipeVisibility {
    Private,
    Shared,
}

impl RecipeVisibility {
    pub const ALL: [RecipeVisibility; 2] = [RecipeVisibility::Private, RecipeVisibility::Shared];

    pub const fn code(self) -> &'static str {
        match self {
            RecipeVisibility::Private => "private",
            RecipeVisibility::Shared => "shared",
        }
    }
}

impl fmt::Display for RecipeVisibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known recipe visibility")]
pub struct UnknownRecipeVisibility(pub String);

impl FromStr for RecipeVisibility {
    type Err = UnknownRecipeVisibility;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|visibility| visibility.code() == value)
            .ok_or_else(|| UnknownRecipeVisibility(value.to_owned()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum MealCategory {
    Breakfast,
    Lunch,
    Dinner,
    Snack,
}

impl MealCategory {
    pub const ALL: [MealCategory; 4] = [
        MealCategory::Breakfast,
        MealCategory::Lunch,
        MealCategory::Dinner,
        MealCategory::Snack,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            MealCategory::Breakfast => "breakfast",
            MealCategory::Lunch => "lunch",
            MealCategory::Dinner => "dinner",
            MealCategory::Snack => "snack",
        }
    }
}

impl fmt::Display for MealCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known meal category")]
pub struct UnknownMealCategory(pub String);

impl FromStr for MealCategory {
    type Err = UnknownMealCategory;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|category| category.code() == value)
            .ok_or_else(|| UnknownMealCategory(value.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecipeComponent {
    pub id: RecipeComponentId,
    pub product_id: ProductId,
    pub amount: ConsumedAmount,
    pub position: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecipeInstruction {
    pub id: RecipeInstructionId,
    pub text: String,
    pub position: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipePhotoDerivatives {
    pub hero_jpeg: Vec<u8>,
    pub card_jpeg: Vec<u8>,
    pub hero_width: i32,
    pub hero_height: i32,
    pub card_width: i32,
    pub card_height: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipePhoto {
    pub recipe_id: RecipeId,
    pub version: i64,
    pub derivatives: RecipePhotoDerivatives,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Recipe {
    pub id: RecipeId,
    pub name: String,
    pub description: Option<String>,
    pub servings: i32,
    pub preparation_minutes: Option<i32>,
    pub cooking_minutes: Option<i32>,
    pub notes: Option<String>,
    pub components: Vec<RecipeComponent>,
    pub instructions: Vec<RecipeInstruction>,
    pub meal_categories: Vec<MealCategory>,
    pub country_categories: Vec<String>,
    pub tags: Vec<String>,
    pub photo_version: Option<i64>,
    pub owner_id: UserId,
    pub visibility: RecipeVisibility,
    pub created_by: UserId,
    pub updated_by: UserId,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecipeSummary {
    pub id: RecipeId,
    pub name: String,
    pub description: Option<String>,
    pub servings: i32,
    pub preparation_minutes: Option<i32>,
    pub cooking_minutes: Option<i32>,
    pub component_count: i64,
    pub meal_categories: Vec<MealCategory>,
    pub country_categories: Vec<String>,
    pub tags: Vec<String>,
    pub photo_version: Option<i64>,
    pub revision: Revision,
    pub updated_at: OffsetDateTime,
    pub archived_at: Option<OffsetDateTime>,
}

impl Recipe {
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct NewRecipeComponent {
    pub id: Option<RecipeComponentId>,
    pub product_id: ProductId,
    pub amount: ConsumedAmount,
}

#[derive(Debug, Clone)]
pub struct NewRecipeInstruction {
    pub id: Option<RecipeInstructionId>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct NewRecipe {
    pub id: Option<RecipeId>,
    pub name: String,
    pub description: Option<String>,
    pub servings: i32,
    pub preparation_minutes: Option<i32>,
    pub cooking_minutes: Option<i32>,
    pub notes: Option<String>,
    pub components: Vec<NewRecipeComponent>,
    pub instructions: Vec<NewRecipeInstruction>,
    pub meal_categories: Vec<MealCategory>,
    pub country_categories: Vec<String>,
    pub tags: Vec<String>,
    pub actor_id: UserId,
}

#[derive(Debug, Clone, Default)]
pub struct RecipePatch {
    pub name: Option<String>,
    pub description: Patch<String>,
    pub servings: Option<i32>,
    pub preparation_minutes: Patch<i32>,
    pub cooking_minutes: Patch<i32>,
    pub notes: Patch<String>,
    pub components: Option<Vec<NewRecipeComponent>>,
    pub instructions: Option<Vec<NewRecipeInstruction>>,
    pub meal_categories: Option<Vec<MealCategory>>,
    pub country_categories: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

impl RecipePatch {
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.description.is_unchanged()
            && self.servings.is_none()
            && self.preparation_minutes.is_unchanged()
            && self.cooking_minutes.is_unchanged()
            && self.notes.is_unchanged()
            && self.components.is_none()
            && self.instructions.is_none()
            && self.meal_categories.is_none()
            && self.country_categories.is_none()
            && self.tags.is_none()
    }
}

pub fn validate_servings(field: &str, servings: i32, errors: &mut ValidationErrors) {
    if servings <= 0 {
        errors.push(field, "Must be more than zero");
    } else if servings > MAX_SERVINGS {
        errors.push(field, "Too many servings");
    }
}

pub fn validate_components(components: &[NewRecipeComponent]) -> crate::error::Result<()> {
    let mut errors = ValidationErrors::new();
    if components.is_empty() {
        errors.push("components", "Add at least one product");
    }
    for (index, component) in components.iter().enumerate() {
        if component.amount.value() <= Decimal::ZERO {
            errors.push(
                format!("components.{index}.amount"),
                "Must be more than zero",
            );
        }
    }
    errors.into_result()
}

fn validate_optional_text(
    field: &str,
    value: Option<&str>,
    max_chars: usize,
    errors: &mut ValidationErrors,
) {
    if value.is_some_and(|value| value.chars().count() > max_chars) {
        errors.push(field, "Too long");
    }
}

fn validate_minutes(field: &str, value: Option<i32>, errors: &mut ValidationErrors) {
    if let Some(value) = value {
        if value <= 0 {
            errors.push(field, "Must be more than zero");
        } else if value > MAX_RECIPE_MINUTES {
            errors.push(field, "Too long");
        }
    }
}

fn validate_instructions(instructions: &[NewRecipeInstruction], errors: &mut ValidationErrors) {
    if instructions.len() > MAX_INSTRUCTIONS {
        errors.push("instructions", "Too many steps");
    }
    for (index, instruction) in instructions.iter().enumerate() {
        if instruction.text.trim().is_empty() {
            errors.push(format!("instructions.{index}.text"), "Required");
        } else if instruction.text.chars().count() > 4_000 {
            errors.push(format!("instructions.{index}.text"), "Too long");
        }
    }
}

struct RecipeMetadata<'a> {
    description: Option<&'a str>,
    preparation_minutes: Option<i32>,
    cooking_minutes: Option<i32>,
    notes: Option<&'a str>,
    instructions: Option<&'a [NewRecipeInstruction]>,
    country_categories: Option<&'a [String]>,
    tags: Option<&'a [String]>,
}

fn validate_metadata(metadata: RecipeMetadata<'_>, errors: &mut ValidationErrors) {
    validate_optional_text("description", metadata.description, 2_000, errors);
    validate_optional_text("notes", metadata.notes, 20_000, errors);
    validate_minutes("preparation_minutes", metadata.preparation_minutes, errors);
    validate_minutes("cooking_minutes", metadata.cooking_minutes, errors);
    if let Some(instructions) = metadata.instructions {
        validate_instructions(instructions, errors);
    }
    if let Some(countries) = metadata.country_categories {
        for (index, country) in countries.iter().enumerate() {
            if !is_country_code(country) {
                errors.push(format!("country_categories.{index}"), "Unknown country");
            }
        }
    }
    if let Some(tags) = metadata.tags {
        if tags.len() > MAX_TAGS {
            errors.push("tags", "Too many tags");
        }
        for (index, tag) in tags.iter().enumerate() {
            if tag.trim().is_empty() {
                errors.push(format!("tags.{index}"), "Required");
            } else if tag.chars().count() > 50 {
                errors.push(format!("tags.{index}"), "Too long");
            }
        }
    }
}

fn is_country_code(value: &str) -> bool {
    value.len() == 2
        && ISO_COUNTRY_CODES
            .split_ascii_whitespace()
            .any(|code| code == value)
}

pub fn normalise_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_owned();
        (!value.is_empty()).then_some(value)
    })
}

pub fn normalise_unique<T: Eq + std::hash::Hash + Copy>(values: Vec<T>) -> Vec<T> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(*value))
        .collect()
}

pub fn normalise_countries(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .map(|value| value.trim().to_ascii_uppercase())
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

pub fn normalise_tags(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .map(|value| value.trim().to_owned())
        .filter(|value| seen.insert(value.to_lowercase()))
        .collect()
}

impl NewRecipe {
    pub fn validate(&self) -> crate::error::Result<()> {
        let mut errors = ValidationErrors::new();
        validate_name("name", &self.name, &mut errors);
        validate_servings("servings", self.servings, &mut errors);
        validate_metadata(
            RecipeMetadata {
                description: self.description.as_deref(),
                preparation_minutes: self.preparation_minutes,
                cooking_minutes: self.cooking_minutes,
                notes: self.notes.as_deref(),
                instructions: Some(&self.instructions),
                country_categories: Some(&self.country_categories),
                tags: Some(&self.tags),
            },
            &mut errors,
        );
        errors.into_result()?;
        validate_components(&self.components)
    }
}

impl RecipePatch {
    pub fn validate(&self) -> crate::error::Result<()> {
        let mut errors = ValidationErrors::new();
        if let Some(name) = &self.name {
            validate_name("name", name, &mut errors);
        }
        if let Some(servings) = self.servings {
            validate_servings("servings", servings, &mut errors);
        }
        validate_metadata(
            RecipeMetadata {
                description: match self.description.as_ref() {
                    Patch::Set(value) => Some(value),
                    _ => None,
                },
                preparation_minutes: match self.preparation_minutes {
                    Patch::Set(value) => Some(value),
                    _ => None,
                },
                cooking_minutes: match self.cooking_minutes {
                    Patch::Set(value) => Some(value),
                    _ => None,
                },
                notes: match self.notes.as_ref() {
                    Patch::Set(value) => Some(value),
                    _ => None,
                },
                instructions: self.instructions.as_deref(),
                country_categories: self.country_categories.as_deref(),
                tags: self.tags.as_deref(),
            },
            &mut errors,
        );
        errors.into_result()?;
        if let Some(components) = &self.components {
            validate_components(components)?;
        }
        Ok(())
    }
}

/// Derives a recipe's per-serving nutrition from the Products fulfilling its components.
///
/// Each line is paired with the Product that fulfils it, or `None` when that Product is
/// unresolved. Unresolved lines contribute nothing and drag the overall quality down: we
/// never guess nutrition for a line we cannot resolve (RCP-016D).
pub fn recipe_nutrition<'a>(
    lines: impl IntoIterator<Item = (&'a ConsumedAmount, Option<&'a Product>)>,
    servings: i32,
) -> ConsumedNutrition {
    let per_line: Vec<ConsumedNutrition> = lines
        .into_iter()
        .map(|(amount, product)| match product {
            Some(product) => nutrition_for(product, amount),
            None => ConsumedNutrition {
                facts: NutritionFacts::default(),
                quality: NutritionQuality::Unknown,
            },
        })
        .collect();

    let total = sum_nutrition(per_line.iter().map(|line| &line.facts));
    let divisor = Decimal::from(servings.max(1));
    let facts = total.scale(Decimal::ONE / divisor);
    let quality = rollup_quality(per_line.iter().map(|line| line.quality));

    ConsumedNutrition { facts, quality }
}

fn rollup_quality(qualities: impl IntoIterator<Item = NutritionQuality>) -> NutritionQuality {
    let mut any = false;
    let mut all_known = true;
    let mut all_unknown = true;
    for quality in qualities {
        any = true;
        match quality {
            NutritionQuality::Known => all_unknown = false,
            NutritionQuality::Unknown => all_known = false,
            NutritionQuality::Partial => {
                all_known = false;
                all_unknown = false;
            }
        }
    }

    if !any || all_unknown {
        NutritionQuality::Unknown
    } else if all_known {
        NutritionQuality::Known
    } else {
        NutritionQuality::Partial
    }
}

#[cfg(test)]
#[path = "recipe_tests.rs"]
mod tests;
