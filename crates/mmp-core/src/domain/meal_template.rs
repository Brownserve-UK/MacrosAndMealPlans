use time::OffsetDateTime;

use super::{
    ConsumedAmount, MealItemRef, MealTemplateComponentId, MealTemplateId, Revision, UserId,
};
use crate::error::{Result, ValidationErrors};

pub const MAX_NAME_LEN: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MealTemplateComponent {
    pub id: MealTemplateComponentId,
    pub item: MealItemRef,
    pub amount: ConsumedAmount,
    pub position: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MealTemplate {
    pub id: MealTemplateId,
    pub owner_id: UserId,
    pub name: String,
    pub components: Vec<MealTemplateComponent>,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub archived_at: Option<OffsetDateTime>,
}

impl MealTemplate {
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct NewMealTemplateComponent {
    pub item: MealItemRef,
    pub amount: ConsumedAmount,
}

#[derive(Debug, Clone)]
pub struct NewMealTemplate {
    pub id: Option<MealTemplateId>,
    pub owner_id: UserId,
    pub name: String,
    pub components: Vec<NewMealTemplateComponent>,
}

#[derive(Debug, Clone, Default)]
pub struct MealTemplatePatch {
    pub name: Option<String>,
    pub components: Option<Vec<NewMealTemplateComponent>>,
}

impl MealTemplatePatch {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.components.is_none()
    }
}

fn validate_name(name: &str, errors: &mut ValidationErrors) {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        errors.push("name", "Required");
    } else if trimmed.chars().count() > MAX_NAME_LEN {
        errors.push("name", "Too long");
    }
}

pub fn validate_template_components(
    components: &[NewMealTemplateComponent],
    errors: &mut ValidationErrors,
) {
    if components.is_empty() {
        errors.push("components", "Add at least one food");
    }
    for (index, component) in components.iter().enumerate() {
        let field = format!("components.{index}.amount");
        if component.amount.value() <= rust_decimal::Decimal::ZERO {
            errors.push(field.clone(), "Must be more than zero");
        }
        match component.item {
            MealItemRef::Dish { .. } => {
                errors.push(
                    format!("components.{index}.item"),
                    "Cooked food already in the house cannot be saved into a meal",
                );
            }
            MealItemRef::Recipe { .. } => {
                if !matches!(component.amount, ConsumedAmount::Servings(_)) {
                    errors.push(field.clone(), "Recipes are measured in servings");
                }
            }
            MealItemRef::Ingredient { .. } | MealItemRef::PreparedMeal { .. } => {
                if !matches!(component.amount, ConsumedAmount::Measure(_)) {
                    errors.push(
                        field.clone(),
                        "A food without a brand is measured, not counted",
                    );
                }
            }
            MealItemRef::Product { .. } => {}
        }
    }
}

impl NewMealTemplate {
    pub fn validate(&self) -> Result<()> {
        let mut errors = ValidationErrors::new();
        validate_name(&self.name, &mut errors);
        validate_template_components(&self.components, &mut errors);
        errors.into_result()
    }
}

impl MealTemplatePatch {
    pub fn validate(&self) -> Result<()> {
        let mut errors = ValidationErrors::new();
        if let Some(name) = &self.name {
            validate_name(name, &mut errors);
        }
        if let Some(components) = &self.components {
            validate_template_components(components, &mut errors);
        }
        errors.into_result()
    }
}

pub fn make_components(input: Vec<NewMealTemplateComponent>) -> Vec<MealTemplateComponent> {
    input
        .into_iter()
        .enumerate()
        .map(|(position, component)| MealTemplateComponent {
            id: MealTemplateComponentId::new(),
            item: component.item,
            amount: component.amount,
            position: i32::try_from(position).unwrap_or(i32::MAX),
        })
        .collect()
}

#[cfg(test)]
#[path = "meal_template_tests.rs"]
mod tests;
