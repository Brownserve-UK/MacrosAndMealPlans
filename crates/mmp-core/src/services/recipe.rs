use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::domain::{
    ConsumedAmount, ConsumedNutrition, IngredientId, NewRecipe, NewRecipeComponent,
    NewRecipeInstruction, ProductId, Recipe, RecipeComponent, RecipeComponentId, RecipeId,
    RecipeInstruction, RecipeInstructionId, RecipePatch, RecipePhoto, RecipePhotoDerivatives,
    RecipeRequirement, RecipeSummary, RecipeVisibility, Revision, UserId, normalise_countries,
    normalise_optional_text, normalise_tags, normalise_unique, recipe_nutrition,
};
use crate::error::{CoreError, Result, ValidationErrors};
use crate::ports::{
    Clock, IngredientRepository, Paginated, ProductRepository, RecipeQuery, RecipeRepository,
    UpdateOutcome,
};

use super::fulfilment::RecipeFulfilments;

const RECIPE: &str = "recipe";

#[derive(Debug, Clone, Copy)]
pub enum ResolveRequirement {
    Ingredient { ingredient_id: IngredientId },
    Product { product_id: ProductId },
}

#[derive(Debug, Default)]
pub struct RecipeNames {
    pub products: HashMap<ProductId, String>,
    pub ingredients: HashMap<IngredientId, String>,
    pub candidate_counts: HashMap<IngredientId, i64>,
}

#[derive(Clone)]
pub struct RecipeService {
    recipes: Arc<dyn RecipeRepository>,
    products: Arc<dyn ProductRepository>,
    ingredients: Arc<dyn IngredientRepository>,
    clock: Arc<dyn Clock>,
}

impl RecipeService {
    pub fn new(
        recipes: Arc<dyn RecipeRepository>,
        products: Arc<dyn ProductRepository>,
        ingredients: Arc<dyn IngredientRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            recipes,
            products,
            ingredients,
            clock,
        }
    }

    pub async fn create_recipe(&self, input: NewRecipe) -> Result<Recipe> {
        input.validate()?;
        let name = input.name.trim().to_owned();
        let components = self.assemble_components(&input.components, &[]).await?;
        let instructions = assemble_instructions(&input.instructions, &[])?;

        let now = self.clock.now();
        let recipe = Recipe {
            id: input.id.unwrap_or_default(),
            name,
            description: normalise_optional_text(input.description),
            servings: input.servings,
            preparation_minutes: input.preparation_minutes,
            cooking_minutes: input.cooking_minutes,
            notes: normalise_optional_text(input.notes),
            components,
            instructions,
            meal_categories: normalise_unique(input.meal_categories),
            country_categories: normalise_countries(input.country_categories),
            tags: normalise_tags(input.tags),
            photo_version: None,
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

    pub async fn list_recipes(&self, query: &RecipeQuery) -> Result<Paginated<RecipeSummary>> {
        self.recipes.list(query).await
    }

    pub async fn names_for(&self, recipe: &Recipe) -> Result<RecipeNames> {
        let product_ids: Vec<ProductId> = recipe
            .components
            .iter()
            .filter_map(|component| component.requirement.product_id())
            .collect();
        let ingredient_ids: Vec<IngredientId> = recipe
            .components
            .iter()
            .filter_map(|component| component.requirement.ingredient_id())
            .collect();

        let products = self
            .products
            .get_many(&product_ids)
            .await?
            .into_iter()
            .map(|product| (product.id, product.name))
            .collect();
        let ingredients = self
            .ingredients
            .get_many(&ingredient_ids)
            .await?
            .into_iter()
            .map(|ingredient| (ingredient.id, ingredient.name))
            .collect();
        let candidate_counts = self.products.count_by_ingredient(&ingredient_ids).await?;

        Ok(RecipeNames {
            products,
            ingredients,
            candidate_counts,
        })
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
        current.description = normalise_optional_text(patch.description.apply(current.description));
        if let Some(servings) = patch.servings {
            current.servings = servings;
        }
        current.preparation_minutes = patch.preparation_minutes.apply(current.preparation_minutes);
        current.cooking_minutes = patch.cooking_minutes.apply(current.cooking_minutes);
        current.notes = normalise_optional_text(patch.notes.apply(current.notes));
        if let Some(components) = patch.components {
            current.components = self
                .assemble_components(&components, &current.components)
                .await?;
        }
        if let Some(instructions) = patch.instructions {
            current.instructions = assemble_instructions(&instructions, &current.instructions)?;
        }
        if let Some(categories) = patch.meal_categories {
            current.meal_categories = normalise_unique(categories);
        }
        if let Some(countries) = patch.country_categories {
            current.country_categories = normalise_countries(countries);
        }
        if let Some(tags) = patch.tags {
            current.tags = normalise_tags(tags);
        }

        current.revision = current.revision.next();
        current.updated_by = actor;
        current.updated_at = self.clock.now();
        self.commit(&current, expected).await?;
        Ok(current)
    }

    pub async fn resolve_component(
        &self,
        id: RecipeId,
        expected: Revision,
        component_id: RecipeComponentId,
        resolution: ResolveRequirement,
        actor: UserId,
    ) -> Result<Recipe> {
        let mut current = self.get_recipe(id, actor).await?;
        require_revision(id, expected, current.revision)?;

        let index = current
            .components
            .iter()
            .position(|component| component.id == component_id)
            .ok_or_else(|| CoreError::not_found("recipe component", component_id))?;

        if !current.components[index].requirement.is_unresolved() {
            let mut errors = ValidationErrors::new();
            errors.push("component", "Already resolved");
            return Err(errors.into());
        }

        let requirement = match resolution {
            ResolveRequirement::Ingredient { ingredient_id } => {
                if self.ingredients.get(ingredient_id).await?.is_none() {
                    let mut errors = ValidationErrors::new();
                    errors.push("ingredient_id", "Unknown ingredient");
                    return Err(errors.into());
                }
                RecipeRequirement::Ingredient { ingredient_id }
            }
            ResolveRequirement::Product { product_id } => {
                if self.products.get(product_id).await?.is_none() {
                    let mut errors = ValidationErrors::new();
                    errors.push("product_id", "Unknown product");
                    return Err(errors.into());
                }
                RecipeRequirement::Product { product_id }
            }
        };

        let old_text = match &current.components[index].requirement {
            RecipeRequirement::Unresolved { text } => text.clone(),
            _ => unreachable!("guarded above"),
        };
        current.components[index].requirement = requirement;
        current.components[index].source_text = Some(old_text);

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

    pub async fn get_photo(&self, id: RecipeId, actor: UserId) -> Result<RecipePhoto> {
        self.get_recipe(id, actor).await?;
        self.recipes
            .get_photo(id)
            .await?
            .ok_or_else(|| CoreError::not_found("recipe photo", id))
    }

    pub async fn replace_photo(
        &self,
        id: RecipeId,
        expected: Revision,
        derivatives: RecipePhotoDerivatives,
        actor: UserId,
    ) -> Result<Recipe> {
        let mut current = self.get_recipe(id, actor).await?;
        require_revision(id, expected, current.revision)?;
        let now = self.clock.now();
        let version = current.photo_version.unwrap_or(0) + 1;
        current.photo_version = Some(version);
        current.revision = current.revision.next();
        current.updated_by = actor;
        current.updated_at = now;
        let photo = RecipePhoto {
            recipe_id: id,
            version,
            derivatives,
            updated_at: now,
        };
        self.commit_photo(&current, expected, Some(&photo)).await?;
        Ok(current)
    }

    pub async fn delete_photo(
        &self,
        id: RecipeId,
        expected: Revision,
        actor: UserId,
    ) -> Result<Recipe> {
        let mut current = self.get_recipe(id, actor).await?;
        require_revision(id, expected, current.revision)?;
        if current.photo_version.is_none() {
            return Ok(current);
        }
        current.photo_version = None;
        current.revision = current.revision.next();
        current.updated_by = actor;
        current.updated_at = self.clock.now();
        self.commit_photo(&current, expected, None).await?;
        Ok(current)
    }

    pub async fn nutrition_preview(
        &self,
        servings: i32,
        components: &[NewRecipeComponent],
    ) -> Result<ConsumedNutrition> {
        let lines: Vec<(RecipeRequirement, ConsumedAmount)> = components
            .iter()
            .map(|component| (component.requirement.clone(), component.amount))
            .collect();
        self.derive_from_lines(&lines, servings).await
    }

    async fn derive_nutrition(
        &self,
        components: &[RecipeComponent],
        servings: i32,
    ) -> Result<ConsumedNutrition> {
        let lines: Vec<(RecipeRequirement, ConsumedAmount)> = components
            .iter()
            .map(|component| (component.requirement.clone(), component.amount))
            .collect();
        self.derive_from_lines(&lines, servings).await
    }

    async fn derive_from_lines(
        &self,
        lines: &[(RecipeRequirement, ConsumedAmount)],
        servings: i32,
    ) -> Result<ConsumedNutrition> {
        let requirements: Vec<&RecipeRequirement> =
            lines.iter().map(|(requirement, _)| requirement).collect();
        let fulfilments = RecipeFulfilments::load(&*self.products, &requirements).await?;
        Ok(recipe_nutrition(
            lines
                .iter()
                .map(|(requirement, amount)| (amount, fulfilments.get(requirement))),
            servings,
        ))
    }

    async fn assemble_components(
        &self,
        submitted: &[NewRecipeComponent],
        existing: &[RecipeComponent],
    ) -> Result<Vec<RecipeComponent>> {
        let allowed: HashSet<RecipeComponentId> = existing.iter().map(|c| c.id).collect();
        let previous: HashMap<RecipeComponentId, &RecipeComponent> =
            existing.iter().map(|c| (c.id, c)).collect();

        let known_products: HashSet<ProductId> = self
            .products
            .get_many(
                &submitted
                    .iter()
                    .filter_map(|c| c.requirement.product_id())
                    .collect::<Vec<_>>(),
            )
            .await?
            .into_iter()
            .map(|product| product.id)
            .collect();
        let known_ingredients: HashSet<IngredientId> = self
            .ingredients
            .get_many(
                &submitted
                    .iter()
                    .filter_map(|c| c.requirement.ingredient_id())
                    .collect::<Vec<_>>(),
            )
            .await?
            .into_iter()
            .map(|ingredient| ingredient.id)
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

            match &component.requirement {
                RecipeRequirement::Product { product_id } => {
                    if !known_products.contains(product_id) {
                        errors.push(
                            format!("components.{index}.requirement.product_id"),
                            "Unknown product",
                        );
                    }
                }
                RecipeRequirement::Ingredient { ingredient_id } => {
                    if !known_ingredients.contains(ingredient_id) {
                        errors.push(
                            format!("components.{index}.requirement.ingredient_id"),
                            "Unknown ingredient",
                        );
                    }
                }
                RecipeRequirement::Unresolved { .. } => {}
            }

            let source_text = component
                .id
                .and_then(|id| previous.get(&id))
                .and_then(|prev| prev.source_text.clone());

            components.push(RecipeComponent {
                id,
                requirement: normalise_requirement(&component.requirement),
                source_text,
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

    async fn commit_photo(
        &self,
        recipe: &Recipe,
        expected: Revision,
        photo: Option<&RecipePhoto>,
    ) -> Result<()> {
        match self.recipes.update_photo(recipe, expected, photo).await? {
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

fn normalise_requirement(requirement: &RecipeRequirement) -> RecipeRequirement {
    match requirement {
        RecipeRequirement::Unresolved { text } => RecipeRequirement::Unresolved {
            text: text.trim().to_owned(),
        },
        other => other.clone(),
    }
}

fn assemble_instructions(
    submitted: &[NewRecipeInstruction],
    existing: &[RecipeInstruction],
) -> Result<Vec<RecipeInstruction>> {
    let allowed: HashSet<RecipeInstructionId> = existing.iter().map(|step| step.id).collect();
    let mut errors = ValidationErrors::new();
    let mut seen = HashSet::new();
    let mut instructions = Vec::with_capacity(submitted.len());
    for (index, instruction) in submitted.iter().enumerate() {
        let id = match instruction.id {
            Some(id) => {
                if !existing.is_empty() && !allowed.contains(&id) {
                    errors.push(format!("instructions.{index}.id"), "Unknown step");
                }
                id
            }
            None => RecipeInstructionId::new(),
        };
        if !seen.insert(id) {
            errors.push(format!("instructions.{index}.id"), "Duplicate step");
        }
        instructions.push(RecipeInstruction {
            id,
            text: instruction.text.trim().to_owned(),
            position: index as i32,
        });
    }
    errors.into_result()?;
    Ok(instructions)
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
