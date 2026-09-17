use std::collections::HashSet;

use super::{MealPlanComponent, NewMealPlanComponent};
use crate::domain::Revision;
use crate::error::{Result, ValidationErrors};

pub fn make_components(input: Vec<NewMealPlanComponent>) -> Vec<MealPlanComponent> {
    input
        .into_iter()
        .enumerate()
        .map(|(position, component)| MealPlanComponent {
            id: component.id.unwrap_or_default(),
            item: component.item,
            amount: component.amount,
            position: i32::try_from(position).unwrap_or(i32::MAX),
            snapshot: None,
            revision: Revision::INITIAL,
            display_order: uuid::Uuid::now_v7(),
        })
        .collect()
}

pub fn merge_components(
    existing: &[MealPlanComponent],
    input: Vec<NewMealPlanComponent>,
) -> Result<Vec<MealPlanComponent>> {
    let mut used = HashSet::new();
    let mut errors = ValidationErrors::new();
    let components = input
        .into_iter()
        .enumerate()
        .map(|(position, component)| {
            let Some(id) = component.id else {
                return MealPlanComponent {
                    id: Default::default(),
                    item: component.item,
                    amount: component.amount,
                    position: i32::try_from(position).unwrap_or(i32::MAX),
                    snapshot: None,
                    revision: Revision::INITIAL,
                    display_order: uuid::Uuid::now_v7(),
                };
            };
            if !used.insert(id) {
                errors.push(
                    format!("components.{position}.id"),
                    "Use each component once",
                );
            }
            let Some(previous) = existing.iter().find(|candidate| candidate.id == id) else {
                errors.push(
                    format!("components.{position}.id"),
                    "That component does not belong to this meal",
                );
                return MealPlanComponent {
                    id,
                    item: component.item,
                    amount: component.amount,
                    position: i32::try_from(position).unwrap_or(i32::MAX),
                    snapshot: None,
                    revision: Revision::INITIAL,
                    display_order: uuid::Uuid::now_v7(),
                };
            };
            let position = i32::try_from(position).unwrap_or(i32::MAX);
            let changed = previous.item != component.item
                || previous.amount != component.amount
                || previous.position != position;
            MealPlanComponent {
                id,
                item: component.item,
                amount: component.amount,
                position,
                snapshot: previous.snapshot.clone(),
                revision: if changed {
                    previous.revision.next()
                } else {
                    previous.revision
                },
                display_order: previous.display_order,
            }
        })
        .collect();
    errors.into_result()?;
    Ok(components)
}

pub fn validate_group_shape(
    label: Option<&str>,
    ad_hoc: Option<super::AdHocKind>,
    components: &[NewMealPlanComponent],
) -> crate::error::Result<()> {
    let mut errors = ValidationErrors::new();
    let named = label.is_some_and(|label| !label.trim().is_empty());
    if let Some(label) = label
        && label.chars().count() > MAX_LABEL_LEN
    {
        errors.push("label", "Keep the name under 120 characters");
    }
    if ad_hoc.is_some() && !components.is_empty() {
        errors.push(
            "components",
            "Eating out, takeaway and fend for yourself have no food",
        );
    }
    if ad_hoc.is_none() && components.is_empty() && !named {
        errors.push(
            "components",
            "Add at least one item or give the meal a name",
        );
    }
    errors.into_result()
}

pub const MAX_LABEL_LEN: usize = 120;

pub fn validate_components(components: &[NewMealPlanComponent]) -> crate::error::Result<()> {
    let mut errors = ValidationErrors::new();
    for (index, component) in components.iter().enumerate() {
        if component.amount.value() <= rust_decimal::Decimal::ZERO {
            errors.push(
                format!("components.{index}.amount"),
                "Must be more than zero",
            );
        }
        crate::domain::consumption::validate_recipe_amount(
            &format!("components.{index}.amount"),
            component.item,
            &component.amount,
            &mut errors,
        );
    }
    errors.into_result()
}
