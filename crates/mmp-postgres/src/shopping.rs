use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{
    DemandSubject, IngredientId, OpportunityException, PreparedMealId, ProductId, Purchase,
    PurchaseId, Revision, ShoppingCadence, ShoppingListItem, ShoppingListItemId,
    ShoppingOpportunityId, ShoppingTrip, ShoppingTripRow,
};
use mmp_core::ports::{
    FinishShopRepository, FinishedPurchase, FinishedShoppingTrip, NewStockFromPurchase, Paginated,
    PurchaseQuery, PurchaseRepository, ShoppingCadenceRepository, ShoppingListItemRepository,
    ShoppingOpportunityRepository, ShoppingSuggestionDismissalRepository, ShoppingTripRepository,
    SortDirection, UpdateOutcome,
};
use sqlx::PgPool;
use time::Date;

use crate::error::{map_db_error, repository_error};
use crate::rows::{
    OpportunityExceptionRow, PurchaseRow, ShoppingCadenceRow, ShoppingListItemRow,
    ShoppingTripHeadRow, ShoppingTripRowRow,
};
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

    async fn set(&self, cadence: &ShoppingCadence, expected: Revision) -> Result<UpdateOutcome> {
        let days: Vec<i16> = cadence
            .days
            .iter()
            .map(|day| i16::from(mmp_core::domain::week_day_number(day)))
            .collect();
        let affected = if expected == Revision::UNRECORDED {
            sqlx::query(
                "INSERT INTO shopping_cadence (
                     singleton, interval_weeks, days_of_week, anchor_date, usual_time,
                     revision, created_at, updated_at
                 ) VALUES (TRUE, $1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (singleton) DO NOTHING",
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
            .map_err(|e| map_db_error(e, "saving the shopping cadence"))?
            .rows_affected()
        } else {
            sqlx::query(
                "UPDATE shopping_cadence SET
                     interval_weeks = $1, days_of_week = $2, anchor_date = $3, usual_time = $4,
                     revision = $5, updated_at = $6
                 WHERE singleton AND revision = $7",
            )
            .bind(i32::from(cadence.interval_weeks))
            .bind(&days)
            .bind(cadence.anchor)
            .bind(cadence.usual_time)
            .bind(cadence.revision.get())
            .bind(cadence.updated_at)
            .bind(expected.get())
            .execute(&self.pool)
            .await
            .map_err(|e| map_db_error(e, "saving the shopping cadence"))?
            .rows_affected()
        };

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }
        match self.get().await? {
            Some(current) => Ok(UpdateOutcome::RevisionMismatch {
                actual: current.revision,
            }),
            None => Ok(UpdateOutcome::NotFound),
        }
    }

    async fn clear(&self, expected: Revision) -> Result<UpdateOutcome> {
        let affected =
            sqlx::query("DELETE FROM shopping_cadence WHERE singleton AND revision = $1")
                .bind(expected.get())
                .execute(&self.pool)
                .await
                .map_err(|e| map_db_error(e, "clearing the shopping cadence"))?
                .rows_affected();
        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }
        match self.get().await? {
            Some(current) => Ok(UpdateOutcome::RevisionMismatch {
                actual: current.revision,
            }),
            None => Ok(UpdateOutcome::NotFound),
        }
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

    async fn upsert(
        &self,
        exception: &OpportunityException,
        expected: Revision,
    ) -> Result<UpdateOutcome> {
        let affected = if expected == Revision::UNRECORDED {
            sqlx::query(
                "INSERT INTO shopping_opportunity (
                     id, generated_for, effective_date, usual_time, state, note,
                     revision, created_at, updated_at
                 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
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
            .map_err(|e| map_db_error(e, "saving a shopping opportunity"))?
            .rows_affected()
        } else {
            sqlx::query(
                "UPDATE shopping_opportunity SET
                     generated_for = $1, effective_date = $2, usual_time = $3, state = $4,
                     note = $5, revision = $6, updated_at = $7
                 WHERE id = $8 AND revision = $9",
            )
            .bind(exception.generated_for)
            .bind(exception.effective_date)
            .bind(exception.usual_time)
            .bind(exception.state.code())
            .bind(exception.note.as_deref())
            .bind(exception.revision.get())
            .bind(exception.updated_at)
            .bind(exception.id.as_uuid())
            .bind(expected.get())
            .execute(&self.pool)
            .await
            .map_err(|e| map_db_error(e, "saving a shopping opportunity"))?
            .rows_affected()
        };

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }
        match self.get(exception.id).await? {
            Some(current) => Ok(UpdateOutcome::RevisionMismatch {
                actual: current.revision,
            }),
            None => Ok(UpdateOutcome::NotFound),
        }
    }

    async fn delete(&self, id: ShoppingOpportunityId, expected: Revision) -> Result<UpdateOutcome> {
        let affected =
            sqlx::query("DELETE FROM shopping_opportunity WHERE id = $1 AND revision = $2")
                .bind(id.as_uuid())
                .bind(expected.get())
                .execute(&self.pool)
                .await
                .map_err(|e| map_db_error(e, "deleting a shopping opportunity"))?
                .rows_affected();
        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }
        match self.get(id).await? {
            Some(current) => Ok(UpdateOutcome::RevisionMismatch {
                actual: current.revision,
            }),
            None => Ok(UpdateOutcome::NotFound),
        }
    }
}

pub struct PgShoppingSuggestionDismissalRepository {
    pool: PgPool,
}

impl PgShoppingSuggestionDismissalRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ShoppingSuggestionDismissalRepository for PgShoppingSuggestionDismissalRepository {
    async fn list_for_date(&self, date: Date) -> Result<Vec<DemandSubject>> {
        let rows: Vec<(Option<uuid::Uuid>, Option<uuid::Uuid>, Option<uuid::Uuid>)> =
            sqlx::query_as(
                "SELECT ingredient_id, prepared_meal_id, product_id
                 FROM shopping_suggestion_dismissal WHERE opportunity_date = $1",
            )
            .bind(date)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing dismissed shopping suggestions", e))?;
        rows.into_iter()
            .map(|(ingredient_id, prepared_meal_id, product_id)| {
                match (ingredient_id, prepared_meal_id, product_id) {
                    (Some(id), None, None) => Ok(DemandSubject::ingredient(IngredientId::from(id))),
                    (None, Some(id), None) => {
                        Ok(DemandSubject::prepared_meal(PreparedMealId::from(id)))
                    }
                    (None, None, Some(id)) => Ok(DemandSubject::product(ProductId::from(id))),
                    _ => Err(mmp_core::CoreError::conflict(
                        "A dismissed shopping suggestion has an invalid subject.",
                    )),
                }
            })
            .collect()
    }

    async fn insert(&self, date: Date, subject: DemandSubject) -> Result<()> {
        let (ingredient_id, prepared_meal_id, product_id) = dismissal_subject(subject)?;
        sqlx::query(
            "INSERT INTO shopping_suggestion_dismissal
             (opportunity_date, ingredient_id, prepared_meal_id, product_id)
             VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
        )
        .bind(date)
        .bind(ingredient_id)
        .bind(prepared_meal_id)
        .bind(product_id)
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "dismissing a shopping suggestion"))?;
        Ok(())
    }

    async fn delete(&self, date: Date, subject: DemandSubject) -> Result<UpdateOutcome> {
        let (ingredient_id, prepared_meal_id, product_id) = dismissal_subject(subject)?;
        let affected = sqlx::query(
            "DELETE FROM shopping_suggestion_dismissal WHERE opportunity_date = $1
             AND ingredient_id IS NOT DISTINCT FROM $2
             AND prepared_meal_id IS NOT DISTINCT FROM $3
             AND product_id IS NOT DISTINCT FROM $4",
        )
        .bind(date)
        .bind(ingredient_id)
        .bind(prepared_meal_id)
        .bind(product_id)
        .execute(&self.pool)
        .await
        .map_err(|e| repository_error("restoring a shopping suggestion", e))?
        .rows_affected();
        Ok(if affected == 1 {
            UpdateOutcome::Updated
        } else {
            UpdateOutcome::NotFound
        })
    }
}

fn dismissal_subject(
    subject: DemandSubject,
) -> Result<(Option<uuid::Uuid>, Option<uuid::Uuid>, Option<uuid::Uuid>)> {
    match subject {
        DemandSubject::Ingredient { ingredient_id } => {
            Ok((Some(ingredient_id.as_uuid()), None, None))
        }
        DemandSubject::PreparedMeal { prepared_meal_id } => {
            Ok((None, Some(prepared_meal_id.as_uuid()), None))
        }
        DemandSubject::Product { product_id } => Ok((None, None, Some(product_id.as_uuid()))),
        _ => Err(mmp_core::CoreError::conflict(
            "That kind of shopping suggestion cannot be dismissed.",
        )),
    }
}

macro_rules! purchase_columns {
    () => {
        "id, ingredient_id, prepared_meal_id, product_id, name, quantity_value, quantity_unit, \
         opportunity_date, state, stock_item_id, purchased_at, actor_user_id, note, revision, \
         created_at, updated_at"
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
                 id, ingredient_id, prepared_meal_id, product_id, name, quantity_value,
                 quantity_unit, opportunity_date, state, stock_item_id, purchased_at,
                 actor_user_id, note, revision, created_at, updated_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
        )
        .bind(purchase.id.as_uuid())
        .bind(purchase.ingredient_id.map(|id| id.as_uuid()))
        .bind(purchase.prepared_meal_id.map(|id| id.as_uuid()))
        .bind(purchase.product_id.map(|id| id.as_uuid()))
        .bind(purchase.name.as_deref())
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
                 ingredient_id = $2, prepared_meal_id = $3, product_id = $4, name = $5,
                 quantity_value = $6, quantity_unit = $7, opportunity_date = $8, state = $9,
                 stock_item_id = $10, note = $11, revision = $12, updated_at = $13
             WHERE id = $1 AND revision = $14",
        )
        .bind(purchase.id.as_uuid())
        .bind(purchase.ingredient_id.map(|id| id.as_uuid()))
        .bind(purchase.prepared_meal_id.map(|id| id.as_uuid()))
        .bind(purchase.product_id.map(|id| id.as_uuid()))
        .bind(purchase.name.as_deref())
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

pub struct PgFinishShopRepository {
    pool: PgPool,
}

impl PgFinishShopRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FinishShopRepository for PgFinishShopRepository {
    async fn finish_shop(
        &self,
        date: Date,
        finished: &[FinishedPurchase],
        trip: Option<&FinishedShoppingTrip>,
    ) -> Result<UpdateOutcome> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| repository_error("starting a shop", e))?;

        for held in finished {
            let purchase = &held.purchase;
            insert_stock_item(&mut tx, &held.stock.item, &held.stock.event).await?;

            let affected = sqlx::query(
                "UPDATE purchase SET state = $2, stock_item_id = $3, revision = $4, \
                 updated_at = $5 WHERE id = $1 AND revision = $6",
            )
            .bind(purchase.id.as_uuid())
            .bind(purchase.state.code())
            .bind(purchase.stock_item_id.map(|id| id.as_uuid()))
            .bind(purchase.revision.get())
            .bind(purchase.updated_at)
            .bind(held.expected.get())
            .execute(&mut *tx)
            .await
            .map_err(|e| map_db_error(e, "finishing a shop"))?
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
        }

        sqlx::query("DELETE FROM shopping_list_item WHERE opportunity_date = $1")
            .bind(date)
            .execute(&mut *tx)
            .await
            .map_err(|e| repository_error("clearing shopping list items", e))?;

        if let Some(held) = trip {
            let trip = &held.trip;
            let affected = sqlx::query(
                "UPDATE shopping_trip SET state = $2, finished_at = $3, revision = $4, \
                 updated_at = $5 WHERE id = $1 AND revision = $6",
            )
            .bind(trip.id.as_uuid())
            .bind(trip.state.code())
            .bind(trip.finished_at)
            .bind(trip.revision.get())
            .bind(trip.updated_at)
            .bind(held.expected.get())
            .execute(&mut *tx)
            .await
            .map_err(|e| map_db_error(e, "finishing a shopping trip"))?
            .rows_affected();
            if affected != 1 {
                let current: Option<(i64,)> =
                    sqlx::query_as("SELECT revision FROM shopping_trip WHERE id = $1")
                        .bind(trip.id.as_uuid())
                        .fetch_optional(&mut *tx)
                        .await
                        .map_err(|e| repository_error("checking a shopping trip revision", e))?;
                return Ok(match current {
                    Some((actual,)) => UpdateOutcome::RevisionMismatch {
                        actual: Revision::new(actual),
                    },
                    None => UpdateOutcome::NotFound,
                });
            }
        }

        tx.commit()
            .await
            .map_err(|e| repository_error("committing a finished shop", e))?;
        Ok(UpdateOutcome::Updated)
    }
}

const GET_TRIP: &str = "SELECT id, opportunity_date, state, started_at, finished_at, started_by, \
     revision, created_at, updated_at FROM shopping_trip WHERE opportunity_date = $1";
const GET_TRIP_ROWS: &str = "SELECT id, ingredient_id, prepared_meal_id, product_id, name, \
     quantity_value, quantity_unit, section FROM shopping_trip_row WHERE trip_id = $1 \
     ORDER BY position";

pub struct PgShoppingTripRepository {
    pool: PgPool,
}

impl PgShoppingTripRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ShoppingTripRepository for PgShoppingTripRepository {
    async fn for_date(&self, date: Date) -> Result<Option<ShoppingTrip>> {
        let head: Option<ShoppingTripHeadRow> = sqlx::query_as(GET_TRIP)
            .bind(date)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a shopping trip", e))?;
        let Some(head) = head else {
            return Ok(None);
        };
        let rows: Vec<ShoppingTripRowRow> = sqlx::query_as(GET_TRIP_ROWS)
            .bind(head.id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("loading a shopping trip", e))?;
        let rows: Vec<ShoppingTripRow> = rows
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<_>>()?;
        head.into_trip(rows).map(Some)
    }

    async fn insert(&self, trip: &ShoppingTrip) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| repository_error("starting a shopping trip", e))?;

        sqlx::query(
            "INSERT INTO shopping_trip (
                 id, opportunity_date, state, started_at, finished_at, started_by,
                 revision, created_at, updated_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(trip.id.as_uuid())
        .bind(trip.opportunity_date)
        .bind(trip.state.code())
        .bind(trip.started_at)
        .bind(trip.finished_at)
        .bind(trip.started_by.as_uuid())
        .bind(trip.revision.get())
        .bind(trip.created_at)
        .bind(trip.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| map_db_error(e, "saving a shopping trip"))?;

        for (position, row) in trip.rows.iter().enumerate() {
            sqlx::query(
                "INSERT INTO shopping_trip_row (
                     id, trip_id, ingredient_id, prepared_meal_id, product_id, name,
                     quantity_value, quantity_unit, section, position
                 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(row.id.as_uuid())
            .bind(trip.id.as_uuid())
            .bind(row.ingredient_id.map(|id| id.as_uuid()))
            .bind(row.prepared_meal_id.map(|id| id.as_uuid()))
            .bind(row.product_id.map(|id| id.as_uuid()))
            .bind(&row.name)
            .bind(row.quantity.map(|quantity| quantity.amount))
            .bind(row.quantity.map(|quantity| quantity.unit.code()))
            .bind(row.section.map(|section| section.code()))
            .bind(position as i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| map_db_error(e, "saving a shopping trip"))?;
        }

        tx.commit()
            .await
            .map_err(|e| repository_error("committing a shopping trip", e))?;
        Ok(())
    }

    async fn update(&self, trip: &ShoppingTrip, expected: Revision) -> Result<UpdateOutcome> {
        let affected = sqlx::query(
            "UPDATE shopping_trip SET state = $2, finished_at = $3, revision = $4, \
             updated_at = $5 WHERE id = $1 AND revision = $6",
        )
        .bind(trip.id.as_uuid())
        .bind(trip.state.code())
        .bind(trip.finished_at)
        .bind(trip.revision.get())
        .bind(trip.updated_at)
        .bind(expected.get())
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "updating a shopping trip"))?
        .rows_affected();

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }

        let current: Option<(i64,)> =
            sqlx::query_as("SELECT revision FROM shopping_trip WHERE id = $1")
                .bind(trip.id.as_uuid())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| repository_error("checking a shopping trip revision", e))?;
        Ok(match current {
            Some((actual,)) => UpdateOutcome::RevisionMismatch {
                actual: Revision::new(actual),
            },
            None => UpdateOutcome::NotFound,
        })
    }

    async fn delete(
        &self,
        id: mmp_core::domain::ShoppingTripId,
        expected: Revision,
    ) -> Result<UpdateOutcome> {
        let affected = sqlx::query("DELETE FROM shopping_trip WHERE id = $1 AND revision = $2")
            .bind(id.as_uuid())
            .bind(expected.get())
            .execute(&self.pool)
            .await
            .map_err(|e| repository_error("abandoning a shopping trip", e))?
            .rows_affected();
        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }
        let current: Option<(i64,)> =
            sqlx::query_as("SELECT revision FROM shopping_trip WHERE id = $1")
                .bind(id.as_uuid())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| repository_error("checking a shopping trip revision", e))?;
        Ok(match current {
            Some((actual,)) => UpdateOutcome::RevisionMismatch {
                actual: Revision::new(actual),
            },
            None => UpdateOutcome::NotFound,
        })
    }
}

macro_rules! list_item_columns {
    () => {
        "id, ingredient_id, prepared_meal_id, product_id, name, quantity_value, quantity_unit, \
         section, opportunity_date, created_by, revision, created_at, updated_at"
    };
}

const GET_LIST_ITEM: &str = concat!(
    "SELECT ",
    list_item_columns!(),
    " FROM shopping_list_item WHERE id = $1"
);
const LIST_LIST_ITEMS: &str = concat!(
    "SELECT ",
    list_item_columns!(),
    " FROM shopping_list_item ORDER BY created_at ASC, id ASC"
);
const INSERT_LIST_ITEM: &str = "INSERT INTO shopping_list_item (id, ingredient_id, prepared_meal_id, product_id, \
     name, quantity_value, quantity_unit, section, opportunity_date, created_by, revision, \
     created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)";
const UPDATE_LIST_ITEM: &str = "UPDATE shopping_list_item SET name = $2, quantity_value = $3, \
     quantity_unit = $4, section = $5, opportunity_date = $6, revision = $7, updated_at = $8 \
     WHERE id = $1 AND revision = $9";

pub struct PgShoppingListItemRepository {
    pool: PgPool,
}

impl PgShoppingListItemRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ShoppingListItemRepository for PgShoppingListItemRepository {
    async fn get(&self, id: ShoppingListItemId) -> Result<Option<ShoppingListItem>> {
        let row: Option<ShoppingListItemRow> = sqlx::query_as(GET_LIST_ITEM)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a shopping list item", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list(&self) -> Result<Vec<ShoppingListItem>> {
        let rows: Vec<ShoppingListItemRow> = sqlx::query_as(LIST_LIST_ITEMS)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing shopping list items", e))?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn insert(&self, item: &ShoppingListItem) -> Result<()> {
        sqlx::query(INSERT_LIST_ITEM)
            .bind(item.id.as_uuid())
            .bind(item.ingredient_id.map(|id| id.as_uuid()))
            .bind(item.prepared_meal_id.map(|id| id.as_uuid()))
            .bind(item.product_id.map(|id| id.as_uuid()))
            .bind(&item.name)
            .bind(item.quantity.map(|quantity| quantity.amount))
            .bind(item.quantity.map(|quantity| quantity.unit.code()))
            .bind(item.section.map(|section| section.code()))
            .bind(item.opportunity_date)
            .bind(item.created_by.as_uuid())
            .bind(item.revision.get())
            .bind(item.created_at)
            .bind(item.updated_at)
            .execute(&self.pool)
            .await
            .map_err(|e| map_db_error(e, "saving a shopping list item"))?;
        Ok(())
    }

    async fn update(&self, item: &ShoppingListItem, expected: Revision) -> Result<UpdateOutcome> {
        let affected = sqlx::query(UPDATE_LIST_ITEM)
            .bind(item.id.as_uuid())
            .bind(&item.name)
            .bind(item.quantity.map(|quantity| quantity.amount))
            .bind(item.quantity.map(|quantity| quantity.unit.code()))
            .bind(item.section.map(|section| section.code()))
            .bind(item.opportunity_date)
            .bind(item.revision.get())
            .bind(item.updated_at)
            .bind(expected.get())
            .execute(&self.pool)
            .await
            .map_err(|e| map_db_error(e, "updating a shopping list item"))?
            .rows_affected();

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }

        let current: Option<(i64,)> =
            sqlx::query_as("SELECT revision FROM shopping_list_item WHERE id = $1")
                .bind(item.id.as_uuid())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| repository_error("checking a shopping list item revision", e))?;
        Ok(match current {
            Some((actual,)) => UpdateOutcome::RevisionMismatch {
                actual: Revision::new(actual),
            },
            None => UpdateOutcome::NotFound,
        })
    }

    async fn delete(&self, id: ShoppingListItemId, expected: Revision) -> Result<UpdateOutcome> {
        let affected =
            sqlx::query("DELETE FROM shopping_list_item WHERE id = $1 AND revision = $2")
                .bind(id.as_uuid())
                .bind(expected.get())
                .execute(&self.pool)
                .await
                .map_err(|e| repository_error("removing a shopping list item", e))?
                .rows_affected();
        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }
        let current: Option<(i64,)> =
            sqlx::query_as("SELECT revision FROM shopping_list_item WHERE id = $1")
                .bind(id.as_uuid())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| repository_error("checking a shopping list item revision", e))?;
        Ok(match current {
            Some((actual,)) => UpdateOutcome::RevisionMismatch {
                actual: Revision::new(actual),
            },
            None => UpdateOutcome::NotFound,
        })
    }

    async fn delete_for_opportunity(&self, date: Date) -> Result<()> {
        sqlx::query("DELETE FROM shopping_list_item WHERE opportunity_date = $1")
            .bind(date)
            .execute(&self.pool)
            .await
            .map_err(|e| repository_error("clearing shopping list items", e))?;
        Ok(())
    }
}
