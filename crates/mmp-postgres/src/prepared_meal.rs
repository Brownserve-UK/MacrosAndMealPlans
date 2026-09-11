use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{PreparedMeal, PreparedMealId, Revision};
use mmp_core::ports::{
    Paginated, PreparedMealQuery, PreparedMealRepository, PreparedMealSort, SortDirection,
    UpdateOutcome,
};
use sqlx::PgPool;

use crate::error::{map_db_error, repository_error};
use crate::rows::PreparedMealRow;

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
                          WHERE product.mapped_prepared_meal_id = prepared_meal.id \
                            AND product.archived_at IS NULL) = ($4 = false))"
    };
}

const GET_BY_ID: &str = concat!("SELECT ", columns!(), " FROM prepared_meal WHERE id = $1");
const GET_BY_NAME: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM prepared_meal WHERE lower(name) = lower($1)"
);
const GET_BY_SEED_KEY: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM prepared_meal WHERE seed_key = $1"
);
macro_rules! product_count {
    () => {
        "(SELECT count(*) FROM product \
          WHERE product.mapped_prepared_meal_id = prepared_meal.id \
            AND product.archived_at IS NULL)"
    };
}

macro_rules! list {
    ($order:expr) => {
        concat!(
            "SELECT ",
            columns!(),
            " FROM prepared_meal",
            filter!(),
            " ORDER BY ",
            $order,
            " LIMIT $5 OFFSET $6"
        )
    };
}

const COUNT: &str = concat!("SELECT count(*) FROM prepared_meal", filter!());
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
const CURRENT_REVISION: &str = "SELECT revision FROM prepared_meal WHERE id = $1";

pub struct PgPreparedMealRepository {
    pool: PgPool,
}

impl PgPreparedMealRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PreparedMealRepository for PgPreparedMealRepository {
    async fn get(&self, id: PreparedMealId) -> Result<Option<PreparedMeal>> {
        let row: Option<PreparedMealRow> = sqlx::query_as(GET_BY_ID)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a prepared meal", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<PreparedMeal>> {
        let row: Option<PreparedMealRow> = sqlx::query_as(GET_BY_NAME)
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("looking up a prepared meal by name", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_seed_key(&self, seed_key: &str) -> Result<Option<PreparedMeal>> {
        let row: Option<PreparedMealRow> = sqlx::query_as(GET_BY_SEED_KEY)
            .bind(seed_key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("looking up a prepared meal by seed key", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list(&self, query: &PreparedMealQuery) -> Result<Paginated<PreparedMeal>> {
        let origin = query.origin.map(|o| o.code());
        let search = query.search.as_deref();
        let list_sql = match (query.sort_by, query.sort) {
            (PreparedMealSort::Name, SortDirection::Ascending) => LIST_NAME_ASC,
            (PreparedMealSort::Name, SortDirection::Descending) => LIST_NAME_DESC,
            (PreparedMealSort::Created, SortDirection::Ascending) => LIST_CREATED_ASC,
            (PreparedMealSort::Created, SortDirection::Descending) => LIST_CREATED_DESC,
            (PreparedMealSort::ProductCount, SortDirection::Ascending) => LIST_COUNT_ASC,
            (PreparedMealSort::ProductCount, SortDirection::Descending) => LIST_COUNT_DESC,
        };

        let total: (i64,) = sqlx::query_as(COUNT)
            .bind(query.include_archived)
            .bind(origin)
            .bind(search)
            .bind(query.needs_products)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| repository_error("counting prepared meals", e))?;

        let rows: Vec<PreparedMealRow> = sqlx::query_as(list_sql)
            .bind(query.include_archived)
            .bind(origin)
            .bind(search)
            .bind(query.needs_products)
            .bind(query.page.limit())
            .bind(query.page.offset())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing prepared meals", e))?;

        let items = rows
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<PreparedMeal>>>()?;
        Ok(Paginated::new(items, total.0, query.page))
    }

    async fn insert(&self, prepared_meal: &PreparedMeal) -> Result<()> {
        sqlx::query(
            "INSERT INTO prepared_meal (
                 id, name, default_unit, shopping_section,
                 origin, seed_key, source_provider, source_external_id, locally_modified,
                 track_stock,
                 revision, created_at, updated_at, archived_at
             ) VALUES ($1, $2, $3, $13, $4, $5, $6, $7, $8, $14, $9, $10, $11, $12)",
        )
        .bind(prepared_meal.id.as_uuid())
        .bind(&prepared_meal.name)
        .bind(prepared_meal.default_unit.code())
        .bind(prepared_meal.provenance.origin.code())
        .bind(&prepared_meal.provenance.seed_key)
        .bind(&prepared_meal.provenance.source_provider)
        .bind(&prepared_meal.provenance.source_external_id)
        .bind(prepared_meal.provenance.locally_modified)
        .bind(prepared_meal.revision.get())
        .bind(prepared_meal.created_at)
        .bind(prepared_meal.updated_at)
        .bind(prepared_meal.archived_at)
        .bind(prepared_meal.shopping_section.map(|s| s.code()))
        .bind(prepared_meal.track_stock)
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "creating a prepared meal"))?;
        Ok(())
    }

    async fn update(
        &self,
        prepared_meal: &PreparedMeal,
        expected: Revision,
    ) -> Result<UpdateOutcome> {
        let affected = sqlx::query(
            "UPDATE prepared_meal SET
                 name = $2, default_unit = $3, shopping_section = $13, track_stock = $14,
                 origin = $4, seed_key = $5, source_provider = $6, source_external_id = $7,
                 locally_modified = $8,
                 revision = $9, updated_at = $10, archived_at = $11
             WHERE id = $1 AND revision = $12",
        )
        .bind(prepared_meal.id.as_uuid())
        .bind(&prepared_meal.name)
        .bind(prepared_meal.default_unit.code())
        .bind(prepared_meal.provenance.origin.code())
        .bind(&prepared_meal.provenance.seed_key)
        .bind(&prepared_meal.provenance.source_provider)
        .bind(&prepared_meal.provenance.source_external_id)
        .bind(prepared_meal.provenance.locally_modified)
        .bind(prepared_meal.revision.get())
        .bind(prepared_meal.updated_at)
        .bind(prepared_meal.archived_at)
        .bind(expected.get())
        .bind(prepared_meal.shopping_section.map(|s| s.code()))
        .bind(prepared_meal.track_stock)
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "updating a prepared meal"))?
        .rows_affected();

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }

        let current: Option<(i64,)> = sqlx::query_as(CURRENT_REVISION)
            .bind(prepared_meal.id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("re-reading a prepared meal revision", e))?;

        Ok(match current {
            Some((actual,)) => UpdateOutcome::RevisionMismatch {
                actual: Revision::new(actual),
            },
            None => UpdateOutcome::NotFound,
        })
    }
}
