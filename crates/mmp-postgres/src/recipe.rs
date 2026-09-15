use std::collections::HashMap;
use std::str::FromStr;

use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{
    IngredientId, MealCategory, Recipe, RecipeComponent, RecipeComponentId, RecipeId,
    RecipeInstruction, RecipeInstructionId, RecipePhoto, RecipePhotoDerivatives, RecipeRequirement,
    RecipeSummary, RecipeVisibility, Revision, UserId,
};
use mmp_core::ports::{Paginated, RecipeQuery, RecipeRepository, SortDirection, UpdateOutcome};
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_db_error, repository_error};
use crate::rows::{amount_bindings, bad_value, parse_amount};

#[derive(Debug, sqlx::FromRow)]
struct RecipeRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    servings: i32,
    preparation_minutes: Option<i32>,
    cooking_minutes: Option<i32>,
    notes: Option<String>,
    photo_version: Option<i64>,
    owner_id: Uuid,
    visibility: String,
    created_by: Uuid,
    updated_by: Uuid,
    revision: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
    archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, sqlx::FromRow)]
struct InstructionRow {
    id: Uuid,
    recipe_id: Uuid,
    position: i32,
    instruction: String,
}

#[derive(Debug, sqlx::FromRow)]
struct PositionedValueRow {
    recipe_id: Uuid,
    value: String,
}

#[derive(Debug, sqlx::FromRow)]
struct RecipeSummaryRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    servings: i32,
    preparation_minutes: Option<i32>,
    cooking_minutes: Option<i32>,
    component_count: i64,
    unresolved_count: i64,
    meal_categories: Vec<String>,
    country_categories: Vec<String>,
    tags: Vec<String>,
    photo_version: Option<i64>,
    revision: i64,
    updated_at: OffsetDateTime,
    archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, sqlx::FromRow)]
struct PhotoRow {
    recipe_id: Uuid,
    version: i64,
    hero_jpeg: Vec<u8>,
    card_jpeg: Vec<u8>,
    hero_width: i32,
    hero_height: i32,
    card_width: i32,
    card_height: i32,
    updated_at: OffsetDateTime,
}

#[derive(Debug, sqlx::FromRow)]
struct ComponentRow {
    id: Uuid,
    recipe_id: Uuid,
    position: i32,
    ingredient_id: Option<Uuid>,
    product_id: Option<Uuid>,
    unresolved_text: Option<String>,
    source_text: Option<String>,
    amount_kind: String,
    amount_value: Decimal,
    amount_unit: Option<String>,
}

impl ComponentRow {
    fn into_domain(self) -> Result<RecipeComponent> {
        let requirement = match (self.ingredient_id, self.product_id, self.unresolved_text) {
            (Some(ingredient_id), None, None) => RecipeRequirement::Ingredient {
                ingredient_id: ingredient_id.into(),
            },
            (None, Some(product_id), None) => RecipeRequirement::Product {
                product_id: product_id.into(),
            },
            (None, None, Some(text)) => RecipeRequirement::Unresolved { text },
            _ => return Err(bad_value("requirement", "recipe component")),
        };
        Ok(RecipeComponent {
            id: RecipeComponentId::from(self.id),
            requirement,
            source_text: self.source_text,
            amount: parse_amount(&self.amount_kind, self.amount_value, self.amount_unit)?,
            position: self.position,
        })
    }
}

fn assemble(
    row: RecipeRow,
    components: Vec<RecipeComponent>,
    instructions: Vec<RecipeInstruction>,
    meal_categories: Vec<String>,
    country_categories: Vec<String>,
    tags: Vec<String>,
) -> Result<Recipe> {
    Ok(Recipe {
        id: RecipeId::from(row.id),
        name: row.name,
        description: row.description,
        servings: row.servings,
        preparation_minutes: row.preparation_minutes,
        cooking_minutes: row.cooking_minutes,
        notes: row.notes,
        components,
        instructions,
        meal_categories: meal_categories
            .into_iter()
            .map(|value| MealCategory::from_str(&value).map_err(|_| bad_value("category", &value)))
            .collect::<Result<Vec<_>>>()?,
        country_categories,
        tags,
        photo_version: row.photo_version,
        owner_id: UserId::from(row.owner_id),
        visibility: RecipeVisibility::from_str(&row.visibility)
            .map_err(|_| bad_value("visibility", &row.visibility))?,
        created_by: UserId::from(row.created_by),
        updated_by: UserId::from(row.updated_by),
        revision: Revision::new(row.revision),
        created_at: row.created_at,
        updated_at: row.updated_at,
        archived_at: row.archived_at,
    })
}

macro_rules! columns {
    () => {
        "id, name, description, servings, preparation_minutes, cooking_minutes, notes, photo_version, owner_id, visibility, created_by, updated_by, revision, created_at, updated_at, archived_at"
    };
}

const GET_BY_ID: &str = concat!("SELECT ", columns!(), " FROM recipe WHERE id = $1");
const COUNT: &str = "SELECT count(*) FROM recipe WHERE owner_id = $1 AND ($2 OR archived_at IS NULL) AND ($3::text IS NULL OR name ILIKE '%' || $3 || '%')";
macro_rules! summary_columns {
    () => {
        "id, name, description, servings, preparation_minutes, cooking_minutes, (SELECT count(*) FROM recipe_component rc WHERE rc.recipe_id = recipe.id) AS component_count, (SELECT count(*) FROM recipe_component rc WHERE rc.recipe_id = recipe.id AND rc.unresolved_text IS NOT NULL) AS unresolved_count, COALESCE((SELECT array_agg(category ORDER BY position) FROM recipe_meal_category rmc WHERE rmc.recipe_id = recipe.id), ARRAY[]::text[]) AS meal_categories, COALESCE((SELECT array_agg(country_code ORDER BY position) FROM recipe_country_category rcc WHERE rcc.recipe_id = recipe.id), ARRAY[]::text[]) AS country_categories, COALESCE((SELECT array_agg(tag ORDER BY position) FROM recipe_tag rt WHERE rt.recipe_id = recipe.id), ARRAY[]::text[]) AS tags, photo_version, revision, updated_at, archived_at"
    };
}
const LIST_ASC: &str = concat!(
    "SELECT ",
    summary_columns!(),
    " FROM recipe",
    " WHERE owner_id = $1 AND ($2 OR archived_at IS NULL) AND ($3::text IS NULL OR name ILIKE '%' || $3 || '%')",
    " ORDER BY CASE WHEN $3::text IS NULL THEN 0 ELSE similarity(name, $3) END DESC, lower(name) ASC LIMIT $4 OFFSET $5"
);
const LIST_DESC: &str = concat!(
    "SELECT ",
    summary_columns!(),
    " FROM recipe",
    " WHERE owner_id = $1 AND ($2 OR archived_at IS NULL) AND ($3::text IS NULL OR name ILIKE '%' || $3 || '%')",
    " ORDER BY CASE WHEN $3::text IS NULL THEN 0 ELSE similarity(name, $3) END DESC, lower(name) DESC LIMIT $4 OFFSET $5"
);
const LIST_COMPONENTS: &str = "SELECT id, recipe_id, position, ingredient_id, product_id, unresolved_text, source_text, amount_kind, amount_value, amount_unit FROM recipe_component WHERE recipe_id = ANY($1) ORDER BY recipe_id, position";
const LIST_INSTRUCTIONS: &str = "SELECT id, recipe_id, position, instruction FROM recipe_instruction WHERE recipe_id = ANY($1) ORDER BY recipe_id, position";
const LIST_MEAL_CATEGORIES: &str = "SELECT recipe_id, category AS value FROM recipe_meal_category WHERE recipe_id = ANY($1) ORDER BY recipe_id, position";
const LIST_COUNTRY_CATEGORIES: &str = "SELECT recipe_id, country_code AS value FROM recipe_country_category WHERE recipe_id = ANY($1) ORDER BY recipe_id, position";
const LIST_TAGS: &str = "SELECT recipe_id, tag AS value FROM recipe_tag WHERE recipe_id = ANY($1) ORDER BY recipe_id, position";
const REFERENCED_INGREDIENT_IDS: &str = "SELECT DISTINCT rc.ingredient_id FROM recipe_component rc JOIN recipe r ON r.id = rc.recipe_id WHERE rc.ingredient_id IS NOT NULL AND r.archived_at IS NULL AND ($2 OR r.owner_id = $1 OR r.visibility = 'shared') ORDER BY rc.ingredient_id";
const CURRENT_REVISION: &str = "SELECT revision FROM recipe WHERE id = $1";

pub struct PgRecipeRepository {
    pool: PgPool,
}

impl PgRecipeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn components_for(&self, ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<RecipeComponent>>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows: Vec<ComponentRow> = sqlx::query_as(LIST_COMPONENTS)
            .bind(ids)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| repository_error("loading recipe components", error))?;
        let mut grouped: HashMap<Uuid, Vec<RecipeComponent>> = HashMap::new();
        for row in rows {
            grouped
                .entry(row.recipe_id)
                .or_default()
                .push(row.into_domain()?);
        }
        Ok(grouped)
    }

    async fn instructions_for(
        &self,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<RecipeInstruction>>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows: Vec<InstructionRow> = sqlx::query_as(LIST_INSTRUCTIONS)
            .bind(ids)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| repository_error("loading recipe instructions", error))?;
        let mut grouped: HashMap<Uuid, Vec<RecipeInstruction>> = HashMap::new();
        for row in rows {
            grouped
                .entry(row.recipe_id)
                .or_default()
                .push(RecipeInstruction {
                    id: RecipeInstructionId::from(row.id),
                    text: row.instruction,
                    position: row.position,
                });
        }
        Ok(grouped)
    }

    async fn values_for(
        &self,
        ids: &[Uuid],
        sql: &'static str,
        description: &'static str,
    ) -> Result<HashMap<Uuid, Vec<String>>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows: Vec<PositionedValueRow> = sqlx::query_as(sql)
            .bind(ids)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| repository_error(description, error))?;
        let mut grouped: HashMap<Uuid, Vec<String>> = HashMap::new();
        for row in rows {
            grouped.entry(row.recipe_id).or_default().push(row.value);
        }
        Ok(grouped)
    }
}

#[async_trait]
impl RecipeRepository for PgRecipeRepository {
    async fn get(&self, id: RecipeId) -> Result<Option<Recipe>> {
        let row: Option<RecipeRow> = sqlx::query_as(GET_BY_ID)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| repository_error("loading a recipe", error))?;
        let Some(row) = row else {
            return Ok(None);
        };
        let recipe_id = row.id;
        let mut components = self.components_for(&[recipe_id]).await?;
        let mut instructions = self.instructions_for(&[recipe_id]).await?;
        let mut meal_categories = self
            .values_for(
                &[recipe_id],
                LIST_MEAL_CATEGORIES,
                "loading recipe meal categories",
            )
            .await?;
        let mut country_categories = self
            .values_for(
                &[recipe_id],
                LIST_COUNTRY_CATEGORIES,
                "loading recipe country categories",
            )
            .await?;
        let mut tags = self
            .values_for(&[recipe_id], LIST_TAGS, "loading recipe tags")
            .await?;
        Ok(Some(assemble(
            row,
            components.remove(&recipe_id).unwrap_or_default(),
            instructions.remove(&recipe_id).unwrap_or_default(),
            meal_categories.remove(&recipe_id).unwrap_or_default(),
            country_categories.remove(&recipe_id).unwrap_or_default(),
            tags.remove(&recipe_id).unwrap_or_default(),
        )?))
    }

    async fn list(&self, query: &RecipeQuery) -> Result<Paginated<RecipeSummary>> {
        let search = query.search.as_deref();
        let list_sql = match query.sort {
            SortDirection::Ascending => LIST_ASC,
            SortDirection::Descending => LIST_DESC,
        };

        let total: (i64,) = sqlx::query_as(COUNT)
            .bind(query.owner_id.as_uuid())
            .bind(query.include_archived)
            .bind(search)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| repository_error("counting recipes", error))?;

        let rows: Vec<RecipeSummaryRow> = sqlx::query_as(list_sql)
            .bind(query.owner_id.as_uuid())
            .bind(query.include_archived)
            .bind(search)
            .bind(query.page.limit())
            .bind(query.page.offset())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| repository_error("listing recipes", error))?;

        let items = rows
            .into_iter()
            .map(|row| -> Result<RecipeSummary> {
                Ok(RecipeSummary {
                    id: RecipeId::from(row.id),
                    name: row.name,
                    description: row.description,
                    servings: row.servings,
                    preparation_minutes: row.preparation_minutes,
                    cooking_minutes: row.cooking_minutes,
                    component_count: row.component_count,
                    unresolved_count: row.unresolved_count,
                    meal_categories: row
                        .meal_categories
                        .into_iter()
                        .map(|value| {
                            MealCategory::from_str(&value)
                                .map_err(|_| bad_value("category", &value))
                        })
                        .collect::<Result<Vec<_>>>()?,
                    country_categories: row.country_categories,
                    tags: row.tags,
                    photo_version: row.photo_version,
                    revision: Revision::new(row.revision),
                    updated_at: row.updated_at,
                    archived_at: row.archived_at,
                })
            })
            .collect::<Result<Vec<RecipeSummary>>>()?;
        Ok(Paginated::new(items, total.0, query.page))
    }

    async fn referenced_ingredient_ids(
        &self,
        viewer_id: UserId,
        include_all_private: bool,
    ) -> Result<Vec<IngredientId>> {
        let ids: Vec<(Uuid,)> = sqlx::query_as(REFERENCED_INGREDIENT_IDS)
            .bind(viewer_id.as_uuid())
            .bind(include_all_private)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| repository_error("listing referenced ingredients", error))?;
        Ok(ids.into_iter().map(|(id,)| id.into()).collect())
    }

    async fn insert(&self, recipe: &Recipe) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| repository_error("starting a recipe transaction", error))?;
        insert_recipe(&mut tx, recipe).await?;
        insert_components(&mut tx, recipe).await?;
        insert_metadata(&mut tx, recipe).await?;
        tx.commit()
            .await
            .map_err(|error| repository_error("committing a recipe", error))?;
        Ok(())
    }

    async fn update(&self, recipe: &Recipe, expected: Revision) -> Result<UpdateOutcome> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| repository_error("starting a recipe update", error))?;
        let outcome = update_recipe(&mut tx, recipe, expected).await?;
        if outcome != UpdateOutcome::Updated {
            tx.rollback()
                .await
                .map_err(|error| repository_error("rolling back a recipe update", error))?;
            return Ok(outcome);
        }
        sqlx::query("DELETE FROM recipe_component WHERE recipe_id = $1")
            .bind(recipe.id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(|error| map_db_error(error, "replacing recipe components"))?;
        insert_components(&mut tx, recipe).await?;
        delete_metadata(&mut tx, recipe.id).await?;
        insert_metadata(&mut tx, recipe).await?;
        tx.commit()
            .await
            .map_err(|error| repository_error("committing a recipe update", error))?;
        Ok(UpdateOutcome::Updated)
    }

    async fn get_photo(&self, id: RecipeId) -> Result<Option<RecipePhoto>> {
        let row: Option<PhotoRow> = sqlx::query_as("SELECT recipe_id, version, hero_jpeg, card_jpeg, hero_width, hero_height, card_width, card_height, updated_at FROM recipe_photo WHERE recipe_id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| repository_error("loading a recipe photo", error))?;
        Ok(row.map(|row| RecipePhoto {
            recipe_id: RecipeId::from(row.recipe_id),
            version: row.version,
            derivatives: RecipePhotoDerivatives {
                hero_jpeg: row.hero_jpeg,
                card_jpeg: row.card_jpeg,
                hero_width: row.hero_width,
                hero_height: row.hero_height,
                card_width: row.card_width,
                card_height: row.card_height,
            },
            updated_at: row.updated_at,
        }))
    }

    async fn update_photo(
        &self,
        recipe: &Recipe,
        expected: Revision,
        photo: Option<&RecipePhoto>,
    ) -> Result<UpdateOutcome> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| repository_error("starting a recipe photo update", error))?;
        let outcome = update_recipe(&mut tx, recipe, expected).await?;
        if outcome != UpdateOutcome::Updated {
            tx.rollback()
                .await
                .map_err(|error| repository_error("rolling back a recipe photo update", error))?;
            return Ok(outcome);
        }
        match photo {
            Some(photo) => {
                sqlx::query("INSERT INTO recipe_photo (recipe_id, version, hero_jpeg, card_jpeg, hero_width, hero_height, card_width, card_height, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) ON CONFLICT (recipe_id) DO UPDATE SET version = EXCLUDED.version, hero_jpeg = EXCLUDED.hero_jpeg, card_jpeg = EXCLUDED.card_jpeg, hero_width = EXCLUDED.hero_width, hero_height = EXCLUDED.hero_height, card_width = EXCLUDED.card_width, card_height = EXCLUDED.card_height, updated_at = EXCLUDED.updated_at")
                    .bind(photo.recipe_id.as_uuid())
                    .bind(photo.version)
                    .bind(&photo.derivatives.hero_jpeg)
                    .bind(&photo.derivatives.card_jpeg)
                    .bind(photo.derivatives.hero_width)
                    .bind(photo.derivatives.hero_height)
                    .bind(photo.derivatives.card_width)
                    .bind(photo.derivatives.card_height)
                    .bind(photo.updated_at)
                    .execute(&mut *tx)
                    .await
                    .map_err(|error| map_db_error(error, "replacing a recipe photo"))?;
            }
            None => {
                sqlx::query("DELETE FROM recipe_photo WHERE recipe_id = $1")
                    .bind(recipe.id.as_uuid())
                    .execute(&mut *tx)
                    .await
                    .map_err(|error| map_db_error(error, "deleting a recipe photo"))?;
            }
        }
        tx.commit()
            .await
            .map_err(|error| repository_error("committing a recipe photo update", error))?;
        Ok(UpdateOutcome::Updated)
    }
}

async fn insert_recipe(tx: &mut Transaction<'_, Postgres>, recipe: &Recipe) -> Result<()> {
    sqlx::query(
        "INSERT INTO recipe (id, name, description, servings, preparation_minutes, cooking_minutes, notes, photo_version, owner_id, visibility, created_by, updated_by, revision, created_at, updated_at, archived_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
    )
    .bind(recipe.id.as_uuid())
    .bind(&recipe.name)
    .bind(&recipe.description)
    .bind(recipe.servings)
    .bind(recipe.preparation_minutes)
    .bind(recipe.cooking_minutes)
    .bind(&recipe.notes)
    .bind(recipe.photo_version)
    .bind(recipe.owner_id.as_uuid())
    .bind(recipe.visibility.code())
    .bind(recipe.created_by.as_uuid())
    .bind(recipe.updated_by.as_uuid())
    .bind(recipe.revision.get())
    .bind(recipe.created_at)
    .bind(recipe.updated_at)
    .bind(recipe.archived_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| map_db_error(error, "creating a recipe"))?;
    Ok(())
}

async fn update_recipe(
    tx: &mut Transaction<'_, Postgres>,
    recipe: &Recipe,
    expected: Revision,
) -> Result<UpdateOutcome> {
    let affected = sqlx::query(
        "UPDATE recipe SET name = $2, description = $3, servings = $4, preparation_minutes = $5, cooking_minutes = $6, notes = $7, photo_version = $8, visibility = $9, updated_by = $10, revision = $11, updated_at = $12, archived_at = $13 WHERE id = $1 AND revision = $14",
    )
    .bind(recipe.id.as_uuid())
    .bind(&recipe.name)
    .bind(&recipe.description)
    .bind(recipe.servings)
    .bind(recipe.preparation_minutes)
    .bind(recipe.cooking_minutes)
    .bind(&recipe.notes)
    .bind(recipe.photo_version)
    .bind(recipe.visibility.code())
    .bind(recipe.updated_by.as_uuid())
    .bind(recipe.revision.get())
    .bind(recipe.updated_at)
    .bind(recipe.archived_at)
    .bind(expected.get())
    .execute(&mut **tx)
    .await
    .map_err(|error| map_db_error(error, "updating a recipe"))?
    .rows_affected();
    if affected == 1 {
        return Ok(UpdateOutcome::Updated);
    }

    let current: Option<(i64,)> = sqlx::query_as(CURRENT_REVISION)
        .bind(recipe.id.as_uuid())
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| repository_error("re-reading a recipe revision", error))?;
    Ok(match current {
        Some((actual,)) => UpdateOutcome::RevisionMismatch {
            actual: Revision::new(actual),
        },
        None => UpdateOutcome::NotFound,
    })
}

async fn insert_components(tx: &mut Transaction<'_, Postgres>, recipe: &Recipe) -> Result<()> {
    for component in &recipe.components {
        let (kind, value, unit) = amount_bindings(&component.amount);
        let ingredient_id = component.requirement.ingredient_id().map(|id| id.as_uuid());
        let product_id = component.requirement.product_id().map(|id| id.as_uuid());
        let unresolved_text = match &component.requirement {
            RecipeRequirement::Unresolved { text } => Some(text.as_str()),
            _ => None,
        };
        sqlx::query("INSERT INTO recipe_component (id, recipe_id, position, ingredient_id, product_id, unresolved_text, source_text, amount_kind, amount_value, amount_unit) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)")
            .bind(component.id.as_uuid())
            .bind(recipe.id.as_uuid())
            .bind(component.position)
            .bind(ingredient_id)
            .bind(product_id)
            .bind(unresolved_text)
            .bind(component.source_text.as_deref())
            .bind(kind)
            .bind(value)
            .bind(unit)
            .execute(&mut **tx)
            .await
            .map_err(|error| map_db_error(error, "creating a recipe component"))?;
    }
    Ok(())
}

async fn delete_metadata(tx: &mut Transaction<'_, Postgres>, id: RecipeId) -> Result<()> {
    delete_metadata_rows(
        tx,
        id,
        "DELETE FROM recipe_instruction WHERE recipe_id = $1",
        "replacing recipe instructions",
    )
    .await?;
    delete_metadata_rows(
        tx,
        id,
        "DELETE FROM recipe_meal_category WHERE recipe_id = $1",
        "replacing recipe meal categories",
    )
    .await?;
    delete_metadata_rows(
        tx,
        id,
        "DELETE FROM recipe_country_category WHERE recipe_id = $1",
        "replacing recipe country categories",
    )
    .await?;
    delete_metadata_rows(
        tx,
        id,
        "DELETE FROM recipe_tag WHERE recipe_id = $1",
        "replacing recipe tags",
    )
    .await?;
    Ok(())
}

async fn delete_metadata_rows(
    tx: &mut Transaction<'_, Postgres>,
    id: RecipeId,
    sql: &'static str,
    description: &'static str,
) -> Result<()> {
    sqlx::query(sql)
        .bind(id.as_uuid())
        .execute(&mut **tx)
        .await
        .map_err(|error| map_db_error(error, description))?;
    Ok(())
}

async fn insert_metadata(tx: &mut Transaction<'_, Postgres>, recipe: &Recipe) -> Result<()> {
    for instruction in &recipe.instructions {
        sqlx::query("INSERT INTO recipe_instruction (id, recipe_id, position, instruction) VALUES ($1, $2, $3, $4)")
            .bind(instruction.id.as_uuid())
            .bind(recipe.id.as_uuid())
            .bind(instruction.position)
            .bind(&instruction.text)
            .execute(&mut **tx)
            .await
            .map_err(|error| map_db_error(error, "creating a recipe instruction"))?;
    }
    for (position, category) in recipe.meal_categories.iter().enumerate() {
        sqlx::query(
            "INSERT INTO recipe_meal_category (recipe_id, position, category) VALUES ($1, $2, $3)",
        )
        .bind(recipe.id.as_uuid())
        .bind(position as i32)
        .bind(category.code())
        .execute(&mut **tx)
        .await
        .map_err(|error| map_db_error(error, "creating a recipe meal category"))?;
    }
    for (position, country) in recipe.country_categories.iter().enumerate() {
        sqlx::query("INSERT INTO recipe_country_category (recipe_id, position, country_code) VALUES ($1, $2, $3)")
            .bind(recipe.id.as_uuid())
            .bind(position as i32)
            .bind(country)
            .execute(&mut **tx)
            .await
            .map_err(|error| map_db_error(error, "creating a recipe country category"))?;
    }
    for (position, tag) in recipe.tags.iter().enumerate() {
        sqlx::query("INSERT INTO recipe_tag (recipe_id, position, tag) VALUES ($1, $2, $3)")
            .bind(recipe.id.as_uuid())
            .bind(position as i32)
            .bind(tag)
            .execute(&mut **tx)
            .await
            .map_err(|error| map_db_error(error, "creating a recipe tag"))?;
    }
    Ok(())
}
