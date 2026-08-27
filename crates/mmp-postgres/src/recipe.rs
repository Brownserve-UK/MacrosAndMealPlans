use std::collections::HashMap;
use std::str::FromStr;

use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{
    Recipe, RecipeComponent, RecipeComponentId, RecipeId, RecipeVisibility, Revision, UserId,
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
    servings: i32,
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
struct ComponentRow {
    id: Uuid,
    recipe_id: Uuid,
    position: i32,
    product_id: Uuid,
    amount_kind: String,
    amount_value: Decimal,
    amount_unit: Option<String>,
}

impl ComponentRow {
    fn into_domain(self) -> Result<RecipeComponent> {
        Ok(RecipeComponent {
            id: RecipeComponentId::from(self.id),
            product_id: self.product_id.into(),
            amount: parse_amount(&self.amount_kind, self.amount_value, self.amount_unit)?,
            position: self.position,
        })
    }
}

fn assemble(row: RecipeRow, components: Vec<RecipeComponent>) -> Result<Recipe> {
    Ok(Recipe {
        id: RecipeId::from(row.id),
        name: row.name,
        servings: row.servings,
        components,
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
        "id, name, servings, owner_id, visibility, created_by, updated_by, revision, created_at, updated_at, archived_at"
    };
}

const GET_BY_ID: &str = concat!("SELECT ", columns!(), " FROM recipe WHERE id = $1");
const COUNT: &str = "SELECT count(*) FROM recipe WHERE owner_id = $1 AND ($2 OR archived_at IS NULL) AND ($3::text IS NULL OR name ILIKE '%' || $3 || '%')";
const LIST_ASC: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM recipe",
    " WHERE owner_id = $1 AND ($2 OR archived_at IS NULL) AND ($3::text IS NULL OR name ILIKE '%' || $3 || '%')",
    " ORDER BY CASE WHEN $3::text IS NULL THEN 0 ELSE similarity(name, $3) END DESC, lower(name) ASC LIMIT $4 OFFSET $5"
);
const LIST_DESC: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM recipe",
    " WHERE owner_id = $1 AND ($2 OR archived_at IS NULL) AND ($3::text IS NULL OR name ILIKE '%' || $3 || '%')",
    " ORDER BY CASE WHEN $3::text IS NULL THEN 0 ELSE similarity(name, $3) END DESC, lower(name) DESC LIMIT $4 OFFSET $5"
);
const LIST_COMPONENTS: &str = "SELECT id, recipe_id, position, product_id, amount_kind, amount_value, amount_unit FROM recipe_component WHERE recipe_id = ANY($1) ORDER BY recipe_id, position";
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
        Ok(Some(assemble(
            row,
            components.remove(&recipe_id).unwrap_or_default(),
        )?))
    }

    async fn list(&self, query: &RecipeQuery) -> Result<Paginated<Recipe>> {
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

        let rows: Vec<RecipeRow> = sqlx::query_as(list_sql)
            .bind(query.owner_id.as_uuid())
            .bind(query.include_archived)
            .bind(search)
            .bind(query.page.limit())
            .bind(query.page.offset())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| repository_error("listing recipes", error))?;

        let ids: Vec<_> = rows.iter().map(|row| row.id).collect();
        let mut components = self.components_for(&ids).await?;
        let items = rows
            .into_iter()
            .map(|row| {
                let id = row.id;
                assemble(row, components.remove(&id).unwrap_or_default())
            })
            .collect::<Result<Vec<Recipe>>>()?;
        Ok(Paginated::new(items, total.0, query.page))
    }

    async fn insert(&self, recipe: &Recipe) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| repository_error("starting a recipe transaction", error))?;
        insert_recipe(&mut tx, recipe).await?;
        insert_components(&mut tx, recipe).await?;
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
        tx.commit()
            .await
            .map_err(|error| repository_error("committing a recipe update", error))?;
        Ok(UpdateOutcome::Updated)
    }
}

async fn insert_recipe(tx: &mut Transaction<'_, Postgres>, recipe: &Recipe) -> Result<()> {
    sqlx::query(
        "INSERT INTO recipe (id, name, servings, owner_id, visibility, created_by, updated_by, revision, created_at, updated_at, archived_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(recipe.id.as_uuid())
    .bind(&recipe.name)
    .bind(recipe.servings)
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
        "UPDATE recipe SET name = $2, servings = $3, visibility = $4, updated_by = $5, revision = $6, updated_at = $7, archived_at = $8 WHERE id = $1 AND revision = $9",
    )
    .bind(recipe.id.as_uuid())
    .bind(&recipe.name)
    .bind(recipe.servings)
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
        sqlx::query("INSERT INTO recipe_component (id, recipe_id, position, product_id, amount_kind, amount_value, amount_unit) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(component.id.as_uuid())
            .bind(recipe.id.as_uuid())
            .bind(component.position)
            .bind(component.product_id.as_uuid())
            .bind(kind)
            .bind(value)
            .bind(unit)
            .execute(&mut **tx)
            .await
            .map_err(|error| map_db_error(error, "creating a recipe component"))?;
    }
    Ok(())
}
