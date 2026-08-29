use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{
    NewStockEvent, ProductId, Revision, StockEvent, StockItem, StockItemId, StockLevel,
};
use mmp_core::ports::{Paginated, StockQuery, StockRepository, UpdateOutcome};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{map_db_error, repository_error};
use crate::rows::{StockEventRow, StockItemRow};

macro_rules! columns {
    () => {
        "id, product_id, tracking_mode, quantity_value, quantity_unit, estimated_low, \
         estimated_high, storage_location, source_date, source_date_kind, usability_deadline, \
         usability_deadline_basis, note, revision, created_at, updated_at, archived_at"
    };
}

const GET_BY_ID: &str = concat!("SELECT ", columns!(), " FROM stock_item WHERE id = $1");
const CURRENT_REVISION: &str = "SELECT revision FROM stock_item WHERE id = $1";

const LIST_ASC: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM stock_item WHERE ($1 OR archived_at IS NULL) \
     AND ($2::uuid IS NULL OR product_id = $2) \
     ORDER BY created_at ASC, id ASC LIMIT $3 OFFSET $4"
);
const LIST_DESC: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM stock_item WHERE ($1 OR archived_at IS NULL) \
     AND ($2::uuid IS NULL OR product_id = $2) \
     ORDER BY created_at DESC, id DESC LIMIT $3 OFFSET $4"
);
const COUNT: &str = "SELECT count(*) FROM stock_item \
     WHERE ($1 OR archived_at IS NULL) AND ($2::uuid IS NULL OR product_id = $2)";
const LIST_FOR_PRODUCTS: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM stock_item WHERE product_id = ANY($1) ORDER BY created_at ASC, id ASC"
);

pub struct PgStockRepository {
    pool: PgPool,
}

impl PgStockRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

struct LevelBindings {
    tracking_mode: &'static str,
    quantity_value: Option<rust_decimal::Decimal>,
    quantity_unit: Option<&'static str>,
    estimated_low: Option<rust_decimal::Decimal>,
    estimated_high: Option<rust_decimal::Decimal>,
}

fn level_bindings(level: &StockLevel) -> LevelBindings {
    match level {
        StockLevel::Exact { quantity } => LevelBindings {
            tracking_mode: "exact",
            quantity_value: Some(quantity.amount),
            quantity_unit: Some(quantity.unit.code()),
            estimated_low: None,
            estimated_high: None,
        },
        StockLevel::Estimated { low, high, unit } => LevelBindings {
            tracking_mode: "estimated",
            quantity_value: None,
            quantity_unit: Some(unit.code()),
            estimated_low: Some(*low),
            estimated_high: Some(*high),
        },
        StockLevel::NotTracked => LevelBindings {
            tracking_mode: "not_tracked",
            quantity_value: None,
            quantity_unit: None,
            estimated_low: None,
            estimated_high: None,
        },
    }
}

async fn write_event<'e, E>(exec: E, item_id: StockItemId, event: &NewStockEvent) -> Result<()>
where
    E: sqlx::PgExecutor<'e>,
{
    let (delta, unit) = match &event.quantity_delta {
        Some(quantity) => (Some(quantity.amount), Some(quantity.unit.code())),
        None => (None, None),
    };
    sqlx::query(
        "INSERT INTO stock_event (
             id, stock_item_id, event_kind, quantity_delta, quantity_unit,
             actor_user_id, subject_member_id, note
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(Uuid::now_v7())
    .bind(item_id.as_uuid())
    .bind(event.kind.code())
    .bind(delta)
    .bind(unit)
    .bind(event.actor_user_id.map(|id| id.as_uuid()))
    .bind(event.subject_member_id.map(|id| id.as_uuid()))
    .bind(event.note.as_deref())
    .execute(exec)
    .await
    .map_err(|e| map_db_error(e, "recording a stock event"))?;
    Ok(())
}

#[async_trait]
impl StockRepository for PgStockRepository {
    async fn get(&self, id: StockItemId) -> Result<Option<StockItem>> {
        let row: Option<StockItemRow> = sqlx::query_as(GET_BY_ID)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a stock item", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list(&self, query: &StockQuery) -> Result<Paginated<StockItem>> {
        let sql = match query.sort {
            mmp_core::ports::SortDirection::Descending => LIST_DESC,
            mmp_core::ports::SortDirection::Ascending => LIST_ASC,
        };
        let rows: Vec<StockItemRow> = sqlx::query_as(sql)
            .bind(query.include_archived)
            .bind(query.product_id.map(|id| id.as_uuid()))
            .bind(query.page.limit())
            .bind(query.page.offset())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing stock items", e))?;

        let total: (i64,) = sqlx::query_as(COUNT)
            .bind(query.include_archived)
            .bind(query.product_id.map(|id| id.as_uuid()))
            .fetch_one(&self.pool)
            .await
            .map_err(|e| repository_error("counting stock items", e))?;

        let items = rows
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<StockItem>>>()?;
        Ok(Paginated::new(items, total.0, query.page))
    }

    async fn list_for_products(&self, product_ids: &[ProductId]) -> Result<Vec<StockItem>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<Uuid> = product_ids.iter().map(|id| id.as_uuid()).collect();
        let rows: Vec<StockItemRow> = sqlx::query_as(LIST_FOR_PRODUCTS)
            .bind(&ids)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing stock for products", e))?;
        rows.into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<StockItem>>>()
    }

    async fn insert(&self, item: &StockItem, event: &NewStockEvent) -> Result<()> {
        let bindings = level_bindings(&item.level);
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| repository_error("starting a stock insert", e))?;

        sqlx::query(
            "INSERT INTO stock_item (
                 id, product_id, tracking_mode, quantity_value, quantity_unit,
                 estimated_low, estimated_high, storage_location,
                 source_date, source_date_kind, usability_deadline, usability_deadline_basis,
                 note, revision, created_at, updated_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
        )
        .bind(item.id.as_uuid())
        .bind(item.product_id.as_uuid())
        .bind(bindings.tracking_mode)
        .bind(bindings.quantity_value)
        .bind(bindings.quantity_unit)
        .bind(bindings.estimated_low)
        .bind(bindings.estimated_high)
        .bind(item.storage_location.code())
        .bind(item.source_date.as_ref().map(|d| d.date))
        .bind(item.source_date.as_ref().map(|d| d.kind.code()))
        .bind(item.usability_deadline.as_ref().map(|d| d.date))
        .bind(
            item.usability_deadline
                .as_ref()
                .and_then(|d| d.basis.clone()),
        )
        .bind(item.note.as_deref())
        .bind(item.revision.get())
        .bind(item.created_at)
        .bind(item.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| map_db_error(e, "creating a stock item"))?;

        write_event(&mut *tx, item.id, event).await?;

        tx.commit()
            .await
            .map_err(|e| repository_error("committing a stock item", e))?;
        Ok(())
    }

    async fn update(
        &self,
        item: &StockItem,
        expected: Revision,
        event: &NewStockEvent,
    ) -> Result<UpdateOutcome> {
        let bindings = level_bindings(&item.level);
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| repository_error("starting a stock update", e))?;

        let affected = sqlx::query(
            "UPDATE stock_item SET
                 tracking_mode = $2, quantity_value = $3, quantity_unit = $4,
                 estimated_low = $5, estimated_high = $6, storage_location = $7,
                 source_date = $8, source_date_kind = $9,
                 usability_deadline = $10, usability_deadline_basis = $11,
                 note = $12, archived_at = $13, revision = $14, updated_at = $15
             WHERE id = $1 AND revision = $16",
        )
        .bind(item.id.as_uuid())
        .bind(bindings.tracking_mode)
        .bind(bindings.quantity_value)
        .bind(bindings.quantity_unit)
        .bind(bindings.estimated_low)
        .bind(bindings.estimated_high)
        .bind(item.storage_location.code())
        .bind(item.source_date.as_ref().map(|d| d.date))
        .bind(item.source_date.as_ref().map(|d| d.kind.code()))
        .bind(item.usability_deadline.as_ref().map(|d| d.date))
        .bind(
            item.usability_deadline
                .as_ref()
                .and_then(|d| d.basis.clone()),
        )
        .bind(item.note.as_deref())
        .bind(item.archived_at)
        .bind(item.revision.get())
        .bind(item.updated_at)
        .bind(expected.get())
        .execute(&mut *tx)
        .await
        .map_err(|e| map_db_error(e, "updating a stock item"))?
        .rows_affected();

        if affected != 1 {
            tx.rollback()
                .await
                .map_err(|e| repository_error("rolling back a stock update", e))?;
            let current: Option<(i64,)> = sqlx::query_as(CURRENT_REVISION)
                .bind(item.id.as_uuid())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| repository_error("re-reading a stock item revision", e))?;
            return Ok(match current {
                Some((actual,)) => UpdateOutcome::RevisionMismatch {
                    actual: Revision::new(actual),
                },
                None => UpdateOutcome::NotFound,
            });
        }

        write_event(&mut *tx, item.id, event).await?;
        tx.commit()
            .await
            .map_err(|e| repository_error("committing a stock update", e))?;
        Ok(UpdateOutcome::Updated)
    }

    async fn list_events(&self, id: StockItemId) -> Result<Vec<StockEvent>> {
        let rows: Vec<StockEventRow> = sqlx::query_as(
            "SELECT id, stock_item_id, event_kind, quantity_delta, quantity_unit, \
             actor_user_id, subject_member_id, note, occurred_at \
             FROM stock_event WHERE stock_item_id = $1 ORDER BY occurred_at DESC, id DESC",
        )
        .bind(id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| repository_error("listing stock events", e))?;
        rows.into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<StockEvent>>>()
    }
}
