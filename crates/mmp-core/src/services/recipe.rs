use std::collections::HashSet;
use std::sync::Arc;

use crate::domain::{
    ConsumedNutrition, NewRecipe, NewRecipeComponent, ProductId, Recipe, RecipeComponent,
    RecipeComponentId, RecipeId, RecipePatch, RecipeVisibility, Revision, UserId, recipe_nutrition,
};
use crate::error::{CoreError, Result, ValidationErrors};
use crate::ports::{
    Clock, Paginated, ProductRepository, RecipeQuery, RecipeRepository, UpdateOutcome,
};

const RECIPE: &str = "recipe";

#[derive(Clone)]
pub struct RecipeService {
    recipes: Arc<dyn RecipeRepository>,
    products: Arc<dyn ProductRepository>,
    clock: Arc<dyn Clock>,
}

impl RecipeService {
    pub fn new(
        recipes: Arc<dyn RecipeRepository>,
        products: Arc<dyn ProductRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            recipes,
            products,
            clock,
        }
    }

    pub async fn create_recipe(&self, input: NewRecipe) -> Result<Recipe> {
        input.validate()?;
        let name = input.name.trim().to_owned();
        let components = self.assemble_components(&input.components, &[]).await?;

        let now = self.clock.now();
        let recipe = Recipe {
            id: input.id.unwrap_or_default(),
            name,
            servings: input.servings,
            components,
            owner_id: input.actor_id,
            visibility: RecipeVisibility::Private,
            created_by: input.actor_id,
            updated_by: input.actor_id,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
            archived_at: None,
        };

        self.recipes.insert(&recipe).await?;
        Ok(recipe)
    }

    pub async fn get_recipe(&self, id: RecipeId, actor: UserId) -> Result<Recipe> {
        let recipe = self
            .recipes
            .get(id)
            .await?
            .filter(|recipe| recipe.owner_id == actor)
            .ok_or_else(|| CoreError::not_found(RECIPE, id))?;
        Ok(recipe)
    }

    pub async fn list_recipes(&self, query: &RecipeQuery) -> Result<Paginated<Recipe>> {
        self.recipes.list(query).await
    }

    pub async fn update_recipe(
        &self,
        id: RecipeId,
        expected: Revision,
        patch: RecipePatch,
        actor: UserId,
    ) -> Result<Recipe> {
        patch.validate()?;
        let mut current = self.get_recipe(id, actor).await?;
        require_revision(id, expected, current.revision)?;

        if patch.is_empty() {
            return Ok(current);
        }

        if let Some(name) = patch.name {
            current.name = name.trim().to_owned();
        }
        if let Some(servings) = patch.servings {
            current.servings = servings;
        }
        if let Some(components) = patch.components {
            current.components = self
                .assemble_components(&components, &current.components)
                .await?;
        }

        current.revision = current.revision.next();
        current.updated_by = actor;
        current.updated_at = self.clock.now();
        self.commit(&current, expected).await?;
        Ok(current)
    }

    pub async fn set_recipe_archived(
        &self,
        id: RecipeId,
        expected: Revision,
        archived: bool,
        actor: UserId,
    ) -> Result<Recipe> {
        let mut current = self.get_recipe(id, actor).await?;
        require_revision(id, expected, current.revision)?;

        if current.is_archived() == archived {
            return Ok(current);
        }

        let now = self.clock.now();
        current.archived_at = archived.then_some(now);
        current.revision = current.revision.next();
        current.updated_by = actor;
        current.updated_at = now;
        self.commit(&current, expected).await?;
        Ok(current)
    }

    pub async fn nutrition_for(&self, id: RecipeId, actor: UserId) -> Result<ConsumedNutrition> {
        let recipe = self.get_recipe(id, actor).await?;
        self.derive_nutrition(&recipe.components, recipe.servings)
            .await
    }

    pub async fn nutrition_preview(
        &self,
        servings: i32,
        components: &[NewRecipeComponent],
    ) -> Result<ConsumedNutrition> {
        let lines: Vec<(ProductId, crate::domain::ConsumedAmount)> = components
            .iter()
            .map(|component| (component.product_id, component.amount))
            .collect();
        self.derive_from_lines(&lines, servings).await
    }

    async fn derive_nutrition(
        &self,
        components: &[RecipeComponent],
        servings: i32,
    ) -> Result<ConsumedNutrition> {
        let lines: Vec<(ProductId, crate::domain::ConsumedAmount)> = components
            .iter()
            .map(|component| (component.product_id, component.amount))
            .collect();
        self.derive_from_lines(&lines, servings).await
    }

    async fn derive_from_lines(
        &self,
        lines: &[(ProductId, crate::domain::ConsumedAmount)],
        servings: i32,
    ) -> Result<ConsumedNutrition> {
        let ids: Vec<ProductId> = lines.iter().map(|(id, _)| *id).collect();
        let products = self.products.get_many(&ids).await?;
        Ok(recipe_nutrition(
            lines.iter().map(|(id, amount)| {
                let product = products.iter().find(|product| product.id == *id);
                (amount, product)
            }),
            servings,
        ))
    }

    async fn assemble_components(
        &self,
        submitted: &[NewRecipeComponent],
        existing: &[RecipeComponent],
    ) -> Result<Vec<RecipeComponent>> {
        let allowed: HashSet<RecipeComponentId> = existing.iter().map(|c| c.id).collect();
        let known_products: HashSet<ProductId> = self
            .products
            .get_many(&submitted.iter().map(|c| c.product_id).collect::<Vec<_>>())
            .await?
            .into_iter()
            .map(|product| product.id)
            .collect();

        let mut errors = ValidationErrors::new();
        let mut seen = HashSet::new();
        let mut components = Vec::with_capacity(submitted.len());

        for (index, component) in submitted.iter().enumerate() {
            let id = match component.id {
                Some(id) => {
                    if !existing.is_empty() && !allowed.contains(&id) {
                        errors.push(format!("components.{index}.id"), "Unknown component");
                    }
                    id
                }
                None => RecipeComponentId::new(),
            };
            if !seen.insert(id) {
                errors.push(format!("components.{index}.id"), "Duplicate component");
            }
            if !known_products.contains(&component.product_id) {
                errors.push(format!("components.{index}.product_id"), "Unknown product");
            }
            components.push(RecipeComponent {
                id,
                product_id: component.product_id,
                amount: component.amount,
                position: index as i32,
            });
        }

        errors.into_result()?;
        Ok(components)
    }

    async fn commit(&self, recipe: &Recipe, expected: Revision) -> Result<()> {
        match self.recipes.update(recipe, expected).await? {
            UpdateOutcome::Updated => Ok(()),
            UpdateOutcome::RevisionMismatch { actual } => Err(CoreError::RevisionMismatch {
                resource: RECIPE,
                id: recipe.id.to_string(),
                expected,
                actual,
            }),
            UpdateOutcome::NotFound => Err(CoreError::not_found(RECIPE, recipe.id)),
        }
    }
}

fn require_revision(id: RecipeId, expected: Revision, actual: Revision) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(CoreError::RevisionMismatch {
            resource: RECIPE,
            id: id.to_string(),
            expected,
            actual,
        })
    }
}

#[cfg(test)]
#[path = "recipe_tests.rs"]
mod tests;
