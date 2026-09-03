use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::domain::{
    ConsumedAmount, ConsumedNutrition, IngredientId, NewRecipe, NewRecipeComponent,
    NewRecipeInstruction, NutritionQuality, ProductId, Recipe, RecipeComponent, RecipeComponentId,
    RecipeId, RecipeInstruction, RecipeInstructionId, RecipePatch, RecipePhoto,
    RecipePhotoDerivatives, RecipeRequirement, RecipeSummary, RecipeVisibility, Revision, UserId,
    normalise_countries, normalise_optional_text, normalise_tags, normalise_unique,
    recipe_nutrition_detailed,
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

#[derive(Debug, Clone)]
pub struct RecipeNutrition {
    pub consumed: ConsumedNutrition,
    pub gaps: Vec<RecipeNutritionGap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeNutritionGap {
    pub component_id: Option<RecipeComponentId>,
    pub name: String,
    pub reason: NutritionGapReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NutritionGapReason {
    Unmatched,
    NoData,
    Incomplete,
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

    pub async fn ingredients_needing_products(
        &self,
        viewer_id: UserId,
        include_all_private: bool,
    ) -> Result<Vec<crate::domain::Ingredient>> {
        let ids = self
            .recipes
            .referenced_ingredient_ids(viewer_id, include_all_private)
            .await?;
        let counts = self.products.count_by_ingredient(&ids).await?;
        let mut ingredients: Vec<_> = self
            .ingredients
            .get_many(&ids)
            .await?
            .into_iter()
            .filter(|ingredient| !ingredient.is_archived())
            .filter(|ingredient| counts.get(&ingredient.id).copied().unwrap_or(0) == 0)
            .collect();
        ingredients.sort_by_cached_key(|ingredient| ingredient.name.to_lowercase());
        Ok(ingredients)
    }

    pub async fn names_for(&self, recipe: &Recipe) -> Result<RecipeNames> {
        let requirements: Vec<&RecipeRequirement> = recipe
            .components
            .iter()
            .map(|component| &component.requirement)
            .collect();
        self.names_for_requirements(&requirements).await
    }

    async fn names_for_requirements(
        &self,
        requirements: &[&RecipeRequirement],
    ) -> Result<RecipeNames> {
        let product_ids: Vec<ProductId> = requirements
            .iter()
            .filter_map(|requirement| requirement.product_id())
            .collect();
        let ingredient_ids: Vec<IngredientId> = requirements
            .iter()
            .filter_map(|requirement| requirement.ingredient_id())
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

    pub async fn nutrition_for(&self, id: RecipeId, actor: UserId) -> Result<RecipeNutrition> {
        let recipe = self.get_recipe(id, actor).await?;
        let lines: Vec<(Option<RecipeComponentId>, RecipeRequirement, ConsumedAmount)> = recipe
            .components
            .iter()
            .map(|component| {
                (
                    Some(component.id),
                    component.requirement.clone(),
                    component.amount,
                )
            })
            .collect();
        self.derive_from_lines(&lines, recipe.servings).await
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
    ) -> Result<RecipeNutrition> {
        let lines: Vec<(Option<RecipeComponentId>, RecipeRequirement, ConsumedAmount)> = components
            .iter()
            .map(|component| {
                (
                    component.id,
                    component.requirement.clone(),
                    component.amount,
                )
            })
            .collect();
        self.derive_from_lines(&lines, servings).await
    }

    async fn derive_from_lines(
        &self,
        lines: &[(Option<RecipeComponentId>, RecipeRequirement, ConsumedAmount)],
        servings: i32,
    ) -> Result<RecipeNutrition> {
        let requirements: Vec<&RecipeRequirement> = lines
            .iter()
            .map(|(_, requirement, _)| requirement)
            .collect();
        let names = self.names_for_requirements(&requirements).await?;
        let fulfilments = RecipeFulfilments::load(&*self.products, &requirements).await?;
        let derived = recipe_nutrition_detailed(
            lines
                .iter()
                .map(|(_, requirement, amount)| (amount, fulfilments.get(requirement))),
            servings,
        );
        let gaps = lines
            .iter()
            .zip(derived.line_qualities)
            .filter_map(|((component_id, requirement, _), quality)| {
                gap_for(*component_id, requirement, quality, &names)
            })
            .collect();
        Ok(RecipeNutrition {
            consumed: derived.consumed,
            gaps,
        })
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

fn gap_for(
    component_id: Option<RecipeComponentId>,
    requirement: &RecipeRequirement,
    quality: NutritionQuality,
    names: &RecipeNames,
) -> Option<RecipeNutritionGap> {
    let (name, reason) = match requirement {
        RecipeRequirement::Unresolved { text } => (text.clone(), NutritionGapReason::Unmatched),
        RecipeRequirement::Ingredient { ingredient_id } => (
            names
                .ingredients
                .get(ingredient_id)
                .cloned()
                .unwrap_or_else(|| "Unknown ingredient".to_owned()),
            gap_reason(quality)?,
        ),
        RecipeRequirement::Product { product_id } => (
            names
                .products
                .get(product_id)
                .cloned()
                .unwrap_or_else(|| "Unknown product".to_owned()),
            gap_reason(quality)?,
        ),
    };
    Some(RecipeNutritionGap {
        component_id,
        name,
        reason,
    })
}

fn gap_reason(quality: NutritionQuality) -> Option<NutritionGapReason> {
    match quality {
        NutritionQuality::Unknown => Some(NutritionGapReason::NoData),
        NutritionQuality::Partial => Some(NutritionGapReason::Incomplete),
        NutritionQuality::Known | NutritionQuality::Estimated => None,
    }
}

#[cfg(test)]
#[path = "recipe_tests.rs"]
mod tests;
