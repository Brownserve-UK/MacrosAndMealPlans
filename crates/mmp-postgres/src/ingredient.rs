use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{Ingredient, IngredientId, Revision};
use mmp_core::ports::{
    IngredientQuery, IngredientRepository, IngredientSort, Paginated, SortDirection, UpdateOutcome,
};
use sqlx::PgPool;

use crate::error::{map_db_error, repository_error};
use crate::rows::IngredientRow;

macro_rules! columns {
    () => {
        "id, name, default_unit, shopping_section, track_stock, origin, seed_key, source_provider, source_external_id, locally_modified, revision, created_at, updated_at, archived_at"
    };
}

macro_rules! filter {
    () => {
        " WHERE ($1 OR archived_at IS NULL) \
          AND ($2::text IS NULL OR origin = $2) \
          AND ($3::text IS NULL OR name ILIKE '%' || $3 || '%') \
          AND ($4::bool IS NULL \
               OR EXISTS (SELECT 1 FROM product \
                          WHERE product.mapped_ingredient_id = ingredient.id \
                            AND product.archived_at IS NULL) = ($4 = false))"
    };
}

const GET_BY_ID: &str = concat!("SELECT ", columns!(), " FROM ingredient WHERE id = $1");
const GET_BY_NAME: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM ingredient WHERE lower(name) = lower($1)"
);
const GET_BY_SEED_KEY: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM ingredient WHERE seed_key = $1"
);
macro_rules! product_count {
    () => {
        "(SELECT count(*) FROM product \
          WHERE product.mapped_ingredient_id = ingredient.id \
            AND product.archived_at IS NULL)"
    };
}

macro_rules! list {
    ($order:expr) => {
        concat!(
            "SELECT ",
            columns!(),
            " FROM ingredient",
            filter!(),
            " ORDER BY ",
            $order,
            " LIMIT $5 OFFSET $6"
        )
    };
}

const COUNT: &str = concat!("SELECT count(*) FROM ingredient", filter!());
const LIST_NAME_ASC: &str = list!(concat!(
    "CASE WHEN $3::text IS NULL THEN 0 ELSE similarity(name, $3) END DESC",
    ", lower(name) ASC"
));
const LIST_NAME_DESC: &str = list!(concat!(
    "CASE WHEN $3::text IS NULL THEN 0 ELSE similarity(name, $3) END DESC",
    ", lower(name) DESC"
));
const LIST_CREATED_ASC: &str = list!("created_at ASC, lower(name) ASC");
const LIST_CREATED_DESC: &str = list!("created_at DESC, lower(name) ASC");
const LIST_COUNT_ASC: &str = list!(concat!(product_count!(), " ASC, lower(name) ASC"));
const LIST_COUNT_DESC: &str = list!(concat!(product_count!(), " DESC, lower(name) ASC"));
const CURRENT_REVISION: &str = "SELECT revision FROM ingredient WHERE id = $1";

pub struct PgIngredientRepository {
    pool: PgPool,
}

impl PgIngredientRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IngredientRepository for PgIngredientRepository {
    async fn get(&self, id: IngredientId) -> Result<Option<Ingredient>> {
        let row: Option<IngredientRow> = sqlx::query_as(GET_BY_ID)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading an ingredient", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Ingredient>> {
        let row: Option<IngredientRow> = sqlx::query_as(GET_BY_NAME)
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("looking up an ingredient by name", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_seed_key(&self, seed_key: &str) -> Result<Option<Ingredient>> {
        let row: Option<IngredientRow> = sqlx::query_as(GET_BY_SEED_KEY)
            .bind(seed_key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("looking up an ingredient by seed key", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list(&self, query: &IngredientQuery) -> Result<Paginated<Ingredient>> {
        let origin = query.origin.map(|o| o.code());
        let search = query.search.as_deref();
        let list_sql = match (query.sort_by, query.sort) {
            (IngredientSort::Name, SortDirection::Ascending) => LIST_NAME_ASC,
            (IngredientSort::Name, SortDirection::Descending) => LIST_NAME_DESC,
            (IngredientSort::Created, SortDirection::Ascending) => LIST_CREATED_ASC,
            (IngredientSort::Created, SortDirection::Descending) => LIST_CREATED_DESC,
            (IngredientSort::ProductCount, SortDirection::Ascending) => LIST_COUNT_ASC,
            (IngredientSort::ProductCount, SortDirection::Descending) => LIST_COUNT_DESC,
        };

        let total: (i64,) = sqlx::query_as(COUNT)
            .bind(query.include_archived)
            .bind(origin)
            .bind(search)
            .bind(query.needs_products)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| repository_error("counting ingredients", e))?;

        let rows: Vec<IngredientRow> = sqlx::query_as(list_sql)
            .bind(query.include_archived)
            .bind(origin)
            .bind(search)
            .bind(query.needs_products)
            .bind(query.page.limit())
            .bind(query.page.offset())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing ingredients", e))?;

        let items = rows
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<Ingredient>>>()?;
        Ok(Paginated::new(items, total.0, query.page))
    }

    async fn insert(&self, ingredient: &Ingredient) -> Result<()> {
        sqlx::query(
            "INSERT INTO ingredient (
                 id, name, default_unit, shopping_section,
                 origin, seed_key, source_provider, source_external_id, locally_modified,
                 track_stock,
                 revision, created_at, updated_at, archived_at
             ) VALUES ($1, $2, $3, $13, $4, $5, $6, $7, $8, $14, $9, $10, $11, $12)",
        )
        .bind(ingredient.id.as_uuid())
        .bind(&ingredient.name)
        .bind(ingredient.default_unit.code())
        .bind(ingredient.provenance.origin.code())
        .bind(&ingredient.provenance.seed_key)
        .bind(&ingredient.provenance.source_provider)
        .bind(&ingredient.provenance.source_external_id)
        .bind(ingredient.provenance.locally_modified)
        .bind(ingredient.revision.get())
        .bind(ingredient.created_at)
        .bind(ingredient.updated_at)
        .bind(ingredient.archived_at)
        .bind(ingredient.shopping_section.map(|s| s.code()))
        .bind(ingredient.track_stock)
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "creating an ingredient"))?;
        Ok(())
    }

    async fn update(&self, ingredient: &Ingredient, expected: Revision) -> Result<UpdateOutcome> {
        let affected = sqlx::query(
            "UPDATE ingredient SET
                 name = $2, default_unit = $3, shopping_section = $13, track_stock = $14,
                 origin = $4, seed_key = $5, source_provider = $6, source_external_id = $7,
                 locally_modified = $8,
                 revision = $9, updated_at = $10, archived_at = $11
             WHERE id = $1 AND revision = $12",
        )
        .bind(ingredient.id.as_uuid())
        .bind(&ingredient.name)
        .bind(ingredient.default_unit.code())
        .bind(ingredient.provenance.origin.code())
        .bind(&ingredient.provenance.seed_key)
        .bind(&ingredient.provenance.source_provider)
        .bind(&ingredient.provenance.source_external_id)
        .bind(ingredient.provenance.locally_modified)
        .bind(ingredient.revision.get())
        .bind(ingredient.updated_at)
        .bind(ingredient.archived_at)
        .bind(expected.get())
        .bind(ingredient.shopping_section.map(|s| s.code()))
        .bind(ingredient.track_stock)
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "updating an ingredient"))?
        .rows_affected();

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }

        let current: Option<(i64,)> = sqlx::query_as(CURRENT_REVISION)
            .bind(ingredient.id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("re-reading an ingredient revision", e))?;

        Ok(match current {
            Some((actual,)) => UpdateOutcome::RevisionMismatch {
                actual: Revision::new(actual),
            },
            None => UpdateOutcome::NotFound,
        })
    }
}
