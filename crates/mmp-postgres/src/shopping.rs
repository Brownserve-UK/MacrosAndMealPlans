use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{
    OpportunityException, Purchase, PurchaseId, Revision, ShoppingCadence, ShoppingOpportunityId,
};
use mmp_core::ports::{
    NewStockFromPurchase, Paginated, PurchaseQuery, PurchaseRepository, ShoppingCadenceRepository,
    ShoppingOpportunityRepository, SortDirection, UpdateOutcome,
};
use sqlx::PgPool;
use time::Date;

use crate::error::{map_db_error, repository_error};
use crate::rows::{OpportunityExceptionRow, PurchaseRow, ShoppingCadenceRow};
use crate::stock::insert_stock_item;

macro_rules! cadence_columns {
    () => {
        "interval_weeks, days_of_week, anchor_date, usual_time, revision, created_at, updated_at"
    };
}

const GET_CADENCE: &str = concat!(
    "SELECT ",
    cadence_columns!(),
    " FROM shopping_cadence WHERE singleton"
);

pub struct PgShoppingCadenceRepository {
    pool: PgPool,
}

impl PgShoppingCadenceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ShoppingCadenceRepository for PgShoppingCadenceRepository {
    async fn get(&self) -> Result<Option<ShoppingCadence>> {
        let row: Option<ShoppingCadenceRow> = sqlx::query_as(GET_CADENCE)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading the shopping cadence", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn set(&self, cadence: &ShoppingCadence) -> Result<()> {
        let days: Vec<i16> = cadence
            .days
            .iter()
            .map(|day| i16::from(mmp_core::domain::week_day_number(day)))
            .collect();
        sqlx::query(
            "INSERT INTO shopping_cadence (
                 singleton, interval_weeks, days_of_week, anchor_date, usual_time,
                 revision, created_at, updated_at
             ) VALUES (TRUE, $1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (singleton) DO UPDATE SET
                 interval_weeks = EXCLUDED.interval_weeks,
                 days_of_week = EXCLUDED.days_of_week,
                 anchor_date = EXCLUDED.anchor_date,
                 usual_time = EXCLUDED.usual_time,
                 revision = EXCLUDED.revision,
                 updated_at = EXCLUDED.updated_at",
        )
        .bind(i32::from(cadence.interval_weeks))
        .bind(&days)
        .bind(cadence.anchor)
        .bind(cadence.usual_time)
        .bind(cadence.revision.get())
        .bind(cadence.created_at)
        .bind(cadence.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "saving the shopping cadence"))?;
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        sqlx::query("DELETE FROM shopping_cadence WHERE singleton")
            .execute(&self.pool)
            .await
            .map_err(|e| map_db_error(e, "clearing the shopping cadence"))?;
        Ok(())
    }
}

macro_rules! opportunity_columns {
    () => {
        "id, generated_for, effective_date, usual_time, state, note, revision, created_at, \
         updated_at"
    };
}

const GET_OPPORTUNITY: &str = concat!(
    "SELECT ",
    opportunity_columns!(),
    " FROM shopping_opportunity WHERE id = $1"
);
const OPPORTUNITY_FOR_OCCURRENCE: &str = concat!(
    "SELECT ",
    opportunity_columns!(),
    " FROM shopping_opportunity WHERE generated_for = $1"
);
const OPPORTUNITIES_IN_RANGE: &str = concat!(
    "SELECT ",
    opportunity_columns!(),
    " FROM shopping_opportunity \
      WHERE (effective_date BETWEEN $1 AND $2) OR (generated_for BETWEEN $1 AND $2) \
      ORDER BY coalesce(effective_date, generated_for)"
);

pub struct PgShoppingOpportunityRepository {
    pool: PgPool,
}

impl PgShoppingOpportunityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ShoppingOpportunityRepository for PgShoppingOpportunityRepository {
    async fn get(&self, id: ShoppingOpportunityId) -> Result<Option<OpportunityException>> {
        let row: Option<OpportunityExceptionRow> = sqlx::query_as(GET_OPPORTUNITY)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a shopping opportunity", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list_in_range(&self, from: Date, to: Date) -> Result<Vec<OpportunityException>> {
        let rows: Vec<OpportunityExceptionRow> = sqlx::query_as(OPPORTUNITIES_IN_RANGE)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing shopping opportunities", e))?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn find_for_occurrence(
        &self,
        generated_for: Date,
    ) -> Result<Option<OpportunityException>> {
        let row: Option<OpportunityExceptionRow> = sqlx::query_as(OPPORTUNITY_FOR_OCCURRENCE)
            .bind(generated_for)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a shopping opportunity", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn upsert(&self, exception: &OpportunityException) -> Result<()> {
        sqlx::query(
            "INSERT INTO shopping_opportunity (
                 id, generated_for, effective_date, usual_time, state, note,
                 revision, created_at, updated_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (id) DO UPDATE SET
                 generated_for = EXCLUDED.generated_for,
                 effective_date = EXCLUDED.effective_date,
                 usual_time = EXCLUDED.usual_time,
                 state = EXCLUDED.state,
                 note = EXCLUDED.note,
                 revision = EXCLUDED.revision,
                 updated_at = EXCLUDED.updated_at",
        )
        .bind(exception.id.as_uuid())
        .bind(exception.generated_for)
        .bind(exception.effective_date)
        .bind(exception.usual_time)
        .bind(exception.state.code())
        .bind(exception.note.as_deref())
        .bind(exception.revision.get())
        .bind(exception.created_at)
        .bind(exception.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "saving a shopping opportunity"))?;
        Ok(())
    }

    async fn delete(&self, id: ShoppingOpportunityId) -> Result<UpdateOutcome> {
        let affected = sqlx::query("DELETE FROM shopping_opportunity WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(|e| map_db_error(e, "deleting a shopping opportunity"))?
            .rows_affected();
        Ok(if affected == 1 {
            UpdateOutcome::Updated
        } else {
            UpdateOutcome::NotFound
        })
    }
}

macro_rules! purchase_columns {
    () => {
        "id, ingredient_id, product_id, quantity_value, quantity_unit, opportunity_date, state, \
         stock_item_id, purchased_at, actor_user_id, note, revision, created_at, updated_at"
    };
}

macro_rules! purchase_filter {
    () => {
        " WHERE ($1::text IS NULL OR state = $1) \
          AND ($2::date IS NULL OR opportunity_date = $2)"
    };
}

const GET_PURCHASE: &str = concat!(
    "SELECT ",
    purchase_columns!(),
    " FROM purchase WHERE id = $1"
);
const LIST_PURCHASES_ASC: &str = concat!(
    "SELECT ",
    purchase_columns!(),
    " FROM purchase",
    purchase_filter!(),
    " ORDER BY purchased_at ASC, id ASC LIMIT $3 OFFSET $4"
);
const LIST_PURCHASES_DESC: &str = concat!(
    "SELECT ",
    purchase_columns!(),
    " FROM purchase",
    purchase_filter!(),
    " ORDER BY purchased_at DESC, id DESC LIMIT $3 OFFSET $4"
);
const COUNT_PURCHASES: &str = concat!("SELECT count(*) FROM purchase", purchase_filter!());
const LIST_OPEN_PURCHASES: &str = concat!(
    "SELECT ",
    purchase_columns!(),
    " FROM purchase WHERE state <> 'cancelled' ORDER BY purchased_at DESC"
);

pub struct PgPurchaseRepository {
    pool: PgPool,
}

impl PgPurchaseRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PurchaseRepository for PgPurchaseRepository {
    async fn get(&self, id: PurchaseId) -> Result<Option<Purchase>> {
        let row: Option<PurchaseRow> = sqlx::query_as(GET_PURCHASE)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a purchase", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list(&self, query: &PurchaseQuery) -> Result<Paginated<Purchase>> {
        let sql = match query.sort {
            SortDirection::Ascending => LIST_PURCHASES_ASC,
            SortDirection::Descending => LIST_PURCHASES_DESC,
        };
        let state = query.state.map(|state| state.code());

        let rows: Vec<PurchaseRow> = sqlx::query_as(sql)
            .bind(state)
            .bind(query.opportunity_date)
            .bind(query.page.limit())
            .bind(query.page.offset())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing purchases", e))?;

        let (total,): (i64,) = sqlx::query_as(COUNT_PURCHASES)
            .bind(state)
            .bind(query.opportunity_date)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| repository_error("counting purchases", e))?;

        let items: Result<Vec<Purchase>> = rows.into_iter().map(TryInto::try_into).collect();
        Ok(Paginated::new(items?, total, query.page))
    }

    async fn list_open(&self) -> Result<Vec<Purchase>> {
        let rows: Vec<PurchaseRow> = sqlx::query_as(LIST_OPEN_PURCHASES)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing open purchases", e))?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn insert(
        &self,
        purchase: &Purchase,
        stock: Option<&NewStockFromPurchase>,
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| repository_error("starting a purchase", e))?;

        if let Some(stock) = stock {
            insert_stock_item(&mut tx, &stock.item, &stock.event).await?;
        }

        sqlx::query(
            "INSERT INTO purchase (
                 id, ingredient_id, product_id, quantity_value, quantity_unit,
                 opportunity_date, state, stock_item_id, purchased_at, actor_user_id, note,
                 revision, created_at, updated_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(purchase.id.as_uuid())
        .bind(purchase.ingredient_id.map(|id| id.as_uuid()))
        .bind(purchase.product_id.map(|id| id.as_uuid()))
        .bind(purchase.quantity.map(|q| q.amount))
        .bind(purchase.quantity.map(|q| q.unit.code()))
        .bind(purchase.opportunity_date)
        .bind(purchase.state.code())
        .bind(purchase.stock_item_id.map(|id| id.as_uuid()))
        .bind(purchase.purchased_at)
        .bind(purchase.actor_user_id.as_uuid())
        .bind(purchase.note.as_deref())
        .bind(purchase.revision.get())
        .bind(purchase.created_at)
        .bind(purchase.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| map_db_error(e, "recording a purchase"))?;

        tx.commit()
            .await
            .map_err(|e| repository_error("committing a purchase", e))?;
        Ok(())
    }

    async fn update(
        &self,
        purchase: &Purchase,
        expected: Revision,
        stock: Option<&NewStockFromPurchase>,
    ) -> Result<UpdateOutcome> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| repository_error("starting a purchase update", e))?;

        if let Some(stock) = stock {
            insert_stock_item(&mut tx, &stock.item, &stock.event).await?;
        }

        let affected = sqlx::query(
            "UPDATE purchase SET
                 ingredient_id = $2, product_id = $3, quantity_value = $4, quantity_unit = $5,
                 opportunity_date = $6, state = $7, stock_item_id = $8, note = $9,
                 revision = $10, updated_at = $11
             WHERE id = $1 AND revision = $12",
        )
        .bind(purchase.id.as_uuid())
        .bind(purchase.ingredient_id.map(|id| id.as_uuid()))
        .bind(purchase.product_id.map(|id| id.as_uuid()))
        .bind(purchase.quantity.map(|q| q.amount))
        .bind(purchase.quantity.map(|q| q.unit.code()))
        .bind(purchase.opportunity_date)
        .bind(purchase.state.code())
        .bind(purchase.stock_item_id.map(|id| id.as_uuid()))
        .bind(purchase.note.as_deref())
        .bind(purchase.revision.get())
        .bind(purchase.updated_at)
        .bind(expected.get())
        .execute(&mut *tx)
        .await
        .map_err(|e| map_db_error(e, "updating a purchase"))?
        .rows_affected();

        if affected != 1 {
            let current: Option<(i64,)> =
                sqlx::query_as("SELECT revision FROM purchase WHERE id = $1")
                    .bind(purchase.id.as_uuid())
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| repository_error("checking a purchase revision", e))?;
            return Ok(match current {
                Some((actual,)) => UpdateOutcome::RevisionMismatch {
                    actual: Revision::new(actual),
                },
                None => UpdateOutcome::NotFound,
            });
        }

        tx.commit()
            .await
            .map_err(|e| repository_error("committing a purchase update", e))?;
        Ok(UpdateOutcome::Updated)
    }
}
