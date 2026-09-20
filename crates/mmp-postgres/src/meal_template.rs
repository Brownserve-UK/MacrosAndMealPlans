use std::collections::HashMap;

use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{
    IngredientId, MealItemRef, MealTemplate, MealTemplateComponent, MealTemplateComponentId,
    MealTemplateId, PreparedMealId, ProductId, RecipeId, Revision, UserId,
};
use mmp_core::ports::{MealTemplateQuery, MealTemplateRepository, Paginated, UpdateOutcome};
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_db_error, repository_error};
use crate::rows::{amount_bindings, bad_value, item_bindings, parse_amount};

#[derive(Debug, sqlx::FromRow)]
struct MealTemplateRow {
    id: Uuid,
    owner_id: Uuid,
    name: String,
    revision: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
    archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, sqlx::FromRow)]
struct ComponentRow {
    id: Uuid,
    template_id: Uuid,
    position: i32,
    item_kind: String,
    product_id: Option<Uuid>,
    recipe_id: Option<Uuid>,
    ingredient_id: Option<Uuid>,
    prepared_meal_id: Option<Uuid>,
    amount_kind: String,
    amount_value: Decimal,
    amount_unit: Option<String>,
}

impl ComponentRow {
    fn into_domain(self) -> Result<MealTemplateComponent> {
        let item = MealItemRef::from_parts(
            &self.item_kind,
            self.product_id.map(ProductId::from),
            self.recipe_id.map(RecipeId::from),
            self.ingredient_id.map(IngredientId::from),
            self.prepared_meal_id.map(PreparedMealId::from),
        )
        .map_err(|_| bad_value("item_kind", &self.item_kind))?;
        Ok(MealTemplateComponent {
            id: MealTemplateComponentId::from(self.id),
            item,
            amount: parse_amount(&self.amount_kind, self.amount_value, self.amount_unit)?,
            position: self.position,
        })
    }
}

fn assemble(row: MealTemplateRow, components: Vec<MealTemplateComponent>) -> MealTemplate {
    MealTemplate {
        id: MealTemplateId::from(row.id),
        owner_id: UserId::from(row.owner_id),
        name: row.name,
        components,
        revision: Revision::new(row.revision),
        created_at: row.created_at,
        updated_at: row.updated_at,
        archived_at: row.archived_at,
    }
}

const GET_BY_ID: &str = "SELECT id, owner_id, name, revision, created_at, updated_at, archived_at FROM meal_template WHERE id = $1";
const COUNT: &str = "SELECT count(*) FROM meal_template WHERE owner_id = $1 AND ($2 OR archived_at IS NULL) AND ($3::text IS NULL OR name ILIKE '%' || $3 || '%')";
const LIST_ASC: &str = "SELECT id, owner_id, name, revision, created_at, updated_at, archived_at FROM meal_template WHERE owner_id = $1 AND ($2 OR archived_at IS NULL) AND ($3::text IS NULL OR name ILIKE '%' || $3 || '%') ORDER BY CASE WHEN $3::text IS NULL THEN 0 ELSE similarity(name, $3) END DESC, lower(name) ASC LIMIT $4 OFFSET $5";
const LIST_DESC: &str = "SELECT id, owner_id, name, revision, created_at, updated_at, archived_at FROM meal_template WHERE owner_id = $1 AND ($2 OR archived_at IS NULL) AND ($3::text IS NULL OR name ILIKE '%' || $3 || '%') ORDER BY CASE WHEN $3::text IS NULL THEN 0 ELSE similarity(name, $3) END DESC, lower(name) DESC LIMIT $4 OFFSET $5";
const LIST_COMPONENTS: &str = "SELECT id, template_id, position, item_kind, product_id, recipe_id, ingredient_id, prepared_meal_id, amount_kind, amount_value, amount_unit FROM meal_template_component WHERE template_id = ANY($1) ORDER BY template_id, position";
const CURRENT_REVISION: &str = "SELECT revision FROM meal_template WHERE id = $1";

pub struct PgMealTemplateRepository {
    pool: PgPool,
}

impl PgMealTemplateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn components_for(
        &self,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<MealTemplateComponent>>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows: Vec<ComponentRow> = sqlx::query_as(LIST_COMPONENTS)
            .bind(ids)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| repository_error("loading saved meal foods", error))?;
        let mut grouped: HashMap<Uuid, Vec<MealTemplateComponent>> = HashMap::new();
        for row in rows {
            grouped
                .entry(row.template_id)
                .or_default()
                .push(row.into_domain()?);
        }
        Ok(grouped)
    }
}

#[async_trait]
impl MealTemplateRepository for PgMealTemplateRepository {
    async fn get(&self, id: MealTemplateId) -> Result<Option<MealTemplate>> {
        let row: Option<MealTemplateRow> = sqlx::query_as(GET_BY_ID)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| repository_error("loading a saved meal", error))?;
        let Some(row) = row else {
            return Ok(None);
        };
        let template_id = row.id;
        let mut components = self.components_for(&[template_id]).await?;
        Ok(Some(assemble(
            row,
            components.remove(&template_id).unwrap_or_default(),
        )))
    }

    async fn list(&self, query: &MealTemplateQuery) -> Result<Paginated<MealTemplate>> {
        let search = query.search.as_deref();
        let list_sql = match query.sort {
            mmp_core::ports::SortDirection::Ascending => LIST_ASC,
            mmp_core::ports::SortDirection::Descending => LIST_DESC,
        };

        let total: (i64,) = sqlx::query_as(COUNT)
            .bind(query.owner_id.as_uuid())
            .bind(query.include_archived)
            .bind(search)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| repository_error("counting saved meals", error))?;

        let rows: Vec<MealTemplateRow> = sqlx::query_as(list_sql)
            .bind(query.owner_id.as_uuid())
            .bind(query.include_archived)
            .bind(search)
            .bind(query.page.limit())
            .bind(query.page.offset())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| repository_error("listing saved meals", error))?;

        let ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();
        let mut components = self.components_for(&ids).await?;

        let items = rows
            .into_iter()
            .map(|row| {
                let template_id = row.id;
                let components = components.remove(&template_id).unwrap_or_default();
                assemble(row, components)
            })
            .collect();
        Ok(Paginated::new(items, total.0, query.page))
    }

    async fn insert(&self, template: &MealTemplate) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| repository_error("starting a saved meal transaction", error))?;
        insert_template(&mut tx, template).await?;
        insert_components(&mut tx, template).await?;
        tx.commit()
            .await
            .map_err(|error| repository_error("committing a saved meal", error))?;
        Ok(())
    }

    async fn update(&self, template: &MealTemplate, expected: Revision) -> Result<UpdateOutcome> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| repository_error("starting a saved meal update", error))?;
        let outcome = update_template(&mut tx, template, expected).await?;
        if outcome != UpdateOutcome::Updated {
            tx.rollback()
                .await
                .map_err(|error| repository_error("rolling back a saved meal update", error))?;
            return Ok(outcome);
        }
        sqlx::query("DELETE FROM meal_template_component WHERE template_id = $1")
            .bind(template.id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(|error| map_db_error(error, "replacing saved meal foods"))?;
        insert_components(&mut tx, template).await?;
        tx.commit()
            .await
            .map_err(|error| repository_error("committing a saved meal update", error))?;
        Ok(UpdateOutcome::Updated)
    }

    async fn delete(&self, id: MealTemplateId, expected: Revision) -> Result<UpdateOutcome> {
        let affected = sqlx::query("DELETE FROM meal_template WHERE id = $1 AND revision = $2")
            .bind(id.as_uuid())
            .bind(expected.get())
            .execute(&self.pool)
            .await
            .map_err(|error| map_db_error(error, "deleting a saved meal"))?
            .rows_affected();
        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }

        let current: Option<(i64,)> = sqlx::query_as(CURRENT_REVISION)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| repository_error("re-reading a saved meal revision", error))?;
        Ok(match current {
            Some((actual,)) => UpdateOutcome::RevisionMismatch {
                actual: Revision::new(actual),
            },
            None => UpdateOutcome::NotFound,
        })
    }
}

async fn insert_template(
    tx: &mut Transaction<'_, Postgres>,
    template: &MealTemplate,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO meal_template (id, owner_id, name, revision, created_at, updated_at, archived_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(template.id.as_uuid())
    .bind(template.owner_id.as_uuid())
    .bind(&template.name)
    .bind(template.revision.get())
    .bind(template.created_at)
    .bind(template.updated_at)
    .bind(template.archived_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| map_db_error(error, "creating a saved meal"))?;
    Ok(())
}

async fn update_template(
    tx: &mut Transaction<'_, Postgres>,
    template: &MealTemplate,
    expected: Revision,
) -> Result<UpdateOutcome> {
    let affected = sqlx::query(
        "UPDATE meal_template SET name = $2, revision = $3, updated_at = $4, archived_at = $5 WHERE id = $1 AND revision = $6",
    )
    .bind(template.id.as_uuid())
    .bind(&template.name)
    .bind(template.revision.get())
    .bind(template.updated_at)
    .bind(template.archived_at)
    .bind(expected.get())
    .execute(&mut **tx)
    .await
    .map_err(|error| map_db_error(error, "updating a saved meal"))?
    .rows_affected();
    if affected == 1 {
        return Ok(UpdateOutcome::Updated);
    }

    let current: Option<(i64,)> = sqlx::query_as(CURRENT_REVISION)
        .bind(template.id.as_uuid())
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| repository_error("re-reading a saved meal revision", error))?;
    Ok(match current {
        Some((actual,)) => UpdateOutcome::RevisionMismatch {
            actual: Revision::new(actual),
        },
        None => UpdateOutcome::NotFound,
    })
}

async fn insert_components(
    tx: &mut Transaction<'_, Postgres>,
    template: &MealTemplate,
) -> Result<()> {
    for component in &template.components {
        let (kind, value, unit) = amount_bindings(&component.amount);
        let (item_kind, product_id, recipe_id, ingredient_id, prepared_meal_id) =
            item_bindings(&component.item);
        sqlx::query("INSERT INTO meal_template_component (id, template_id, position, item_kind, product_id, recipe_id, ingredient_id, prepared_meal_id, amount_kind, amount_value, amount_unit) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)")
            .bind(component.id.as_uuid())
            .bind(template.id.as_uuid())
            .bind(component.position)
            .bind(item_kind)
            .bind(product_id)
            .bind(recipe_id)
            .bind(ingredient_id)
            .bind(prepared_meal_id)
            .bind(kind)
            .bind(value)
            .bind(unit)
            .execute(&mut **tx)
            .await
            .map_err(|error| map_db_error(error, "creating a saved meal food"))?;
    }
    Ok(())
}
