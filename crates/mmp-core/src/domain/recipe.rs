use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;
use time::OffsetDateTime;

use super::{
    ConsumedAmount, ConsumedNutrition, NutritionFacts, NutritionQuality, Product, ProductId,
    RecipeComponentId, RecipeId, Revision, UserId, nutrition_for, sum_nutrition, validate_name,
};
use crate::error::ValidationErrors;

pub const MAX_SERVINGS: i32 = 10_000;

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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecipeComponent {
    pub id: RecipeComponentId,
    pub product_id: ProductId,
    pub amount: ConsumedAmount,
    pub position: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Recipe {
    pub id: RecipeId,
    pub name: String,
    pub servings: i32,
    pub components: Vec<RecipeComponent>,
    pub owner_id: UserId,
    pub visibility: RecipeVisibility,
    pub created_by: UserId,
    pub updated_by: UserId,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
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
pub struct NewRecipe {
    pub id: Option<RecipeId>,
    pub name: String,
    pub servings: i32,
    pub components: Vec<NewRecipeComponent>,
    pub actor_id: UserId,
}

#[derive(Debug, Clone, Default)]
pub struct RecipePatch {
    pub name: Option<String>,
    pub servings: Option<i32>,
    pub components: Option<Vec<NewRecipeComponent>>,
}

impl RecipePatch {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.servings.is_none() && self.components.is_none()
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

impl NewRecipe {
    pub fn validate(&self) -> crate::error::Result<()> {
        let mut errors = ValidationErrors::new();
        validate_name("name", &self.name, &mut errors);
        validate_servings("servings", self.servings, &mut errors);
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
