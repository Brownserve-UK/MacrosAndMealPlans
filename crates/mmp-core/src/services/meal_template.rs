use std::sync::Arc;

use super::revision::{commit_outcome, require_revision};
use crate::domain::{
    MealItemRef, MealPlanEntryId, MealTemplate, MealTemplateId, MealTemplatePatch, NewMealTemplate,
    NewMealTemplateComponent, Revision, UserId, make_template_components,
};
use crate::error::{CoreError, Result};
use crate::ports::{
    Clock, MealPlanRepository, MealTemplateQuery, MealTemplateRepository, Paginated,
};

const MEAL_TEMPLATE: &str = "meal_template";

#[derive(Clone)]
pub struct MealTemplateService {
    templates: Arc<dyn MealTemplateRepository>,
    plans: Arc<dyn MealPlanRepository>,
    clock: Arc<dyn Clock>,
}

impl MealTemplateService {
    pub fn new(
        templates: Arc<dyn MealTemplateRepository>,
        plans: Arc<dyn MealPlanRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            templates,
            plans,
            clock,
        }
    }

    pub async fn create(&self, input: NewMealTemplate) -> Result<MealTemplate> {
        input.validate()?;
        let now = self.clock.now();
        let template = MealTemplate {
            id: input.id.unwrap_or_default(),
            owner_id: input.owner_id,
            name: input.name.trim().to_owned(),
            components: make_template_components(input.components),
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
            archived_at: None,
        };
        self.templates.insert(&template).await?;
        Ok(template)
    }

    pub async fn get(&self, id: MealTemplateId, owner_id: UserId) -> Result<MealTemplate> {
        self.templates
            .get(id)
            .await?
            .filter(|template| template.owner_id == owner_id)
            .ok_or_else(|| CoreError::not_found(MEAL_TEMPLATE, id))
    }

    pub async fn list(&self, query: &MealTemplateQuery) -> Result<Paginated<MealTemplate>> {
        self.templates.list(query).await
    }

    pub async fn update(
        &self,
        id: MealTemplateId,
        owner_id: UserId,
        expected: Revision,
        patch: MealTemplatePatch,
    ) -> Result<MealTemplate> {
        patch.validate()?;
        let mut current = self.get(id, owner_id).await?;
        require_revision(MEAL_TEMPLATE, id, expected, current.revision)?;

        if patch.is_empty() {
            return Ok(current);
        }

        if let Some(name) = patch.name {
            current.name = name.trim().to_owned();
        }
        if let Some(components) = patch.components {
            current.components = make_template_components(components);
        }
        current.revision = current.revision.next();
        current.updated_at = self.clock.now();

        commit_outcome(
            MEAL_TEMPLATE,
            id,
            expected,
            self.templates.update(&current, expected).await?,
        )?;
        Ok(current)
    }

    pub async fn delete(
        &self,
        id: MealTemplateId,
        owner_id: UserId,
        expected: Revision,
    ) -> Result<()> {
        self.get(id, owner_id).await?;
        commit_outcome(
            MEAL_TEMPLATE,
            id,
            expected,
            self.templates.delete(id, expected).await?,
        )
    }

    pub async fn from_entry(
        &self,
        entry_id: MealPlanEntryId,
        owner_id: UserId,
        name: String,
    ) -> Result<MealTemplate> {
        let entry = self
            .plans
            .get(entry_id)
            .await?
            .ok_or_else(|| CoreError::not_found("meal_plan_entry", entry_id))?;

        let components: Vec<NewMealTemplateComponent> = entry
            .components
            .iter()
            .filter(|component| !matches!(component.item, MealItemRef::Dish { .. }))
            .map(|component| NewMealTemplateComponent {
                item: component.item,
                amount: component.amount,
            })
            .collect();

        self.create(NewMealTemplate {
            id: None,
            owner_id,
            name,
            components,
        })
        .await
    }
}

#[cfg(test)]
#[path = "meal_template_tests.rs"]
mod tests;
