use std::collections::HashMap;

use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{IngredientId, Product, ProductId, Revision};
use mmp_core::ports::{Paginated, ProductQuery, ProductRepository, SortDirection, UpdateOutcome};
use sqlx::PgPool;

use crate::error::{map_db_error, repository_error};
use crate::rows::{ProductRow, nutrition_bindings};

macro_rules! columns {
    () => {
        "id, name, brand, barcode, retailer, shopping_section, package_quantity_amount, package_quantity_unit, servings_per_pack, mapped_ingredient_id, nutrition_basis_amount, nutrition_basis_unit, energy_kcal, protein_g, carbohydrate_g, sugar_g, fat_g, saturated_fat_g, fibre_g, salt_g, cholesterol_mg, nutrition_extra, origin, seed_key, source_provider, source_external_id, locally_modified, revision, created_at, updated_at, archived_at"
    };
}

macro_rules! filter {
    () => {
        " WHERE ($1 OR archived_at IS NULL) AND ($2::text IS NULL OR origin = $2) AND ($3::text IS NULL OR name ILIKE '%' || $3 || '%') AND ($4::text IS NULL OR barcode = $4) AND ($5::text IS NULL OR retailer ILIKE '%' || $5 || '%') AND ($6::uuid IS NULL OR mapped_ingredient_id = $6) AND ($7::bool IS NULL OR (mapped_ingredient_id IS NULL) = $7)"
    };
}

const GET_BY_ID: &str = concat!("SELECT ", columns!(), " FROM product WHERE id = $1");
const GET_BY_BARCODE: &str = concat!("SELECT ", columns!(), " FROM product WHERE barcode = $1");
const GET_BY_SEED_KEY: &str = concat!("SELECT ", columns!(), " FROM product WHERE seed_key = $1");
const COUNT: &str = concat!("SELECT count(*) FROM product", filter!());
const LIST_ASC: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM product",
    filter!(),
    " ORDER BY CASE WHEN $3::text IS NULL THEN 0 ELSE similarity(name, $3) END DESC, lower(name) ASC LIMIT $8 OFFSET $9"
);
const LIST_DESC: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM product",
    filter!(),
    " ORDER BY CASE WHEN $3::text IS NULL THEN 0 ELSE similarity(name, $3) END DESC, lower(name) DESC LIMIT $8 OFFSET $9"
);
const CURRENT_REVISION: &str = "SELECT revision FROM product WHERE id = $1";
const COUNT_BY_INGREDIENT: &str = "SELECT mapped_ingredient_id, count(*) FROM product \
     WHERE mapped_ingredient_id = ANY($1) AND archived_at IS NULL \
     GROUP BY mapped_ingredient_id";
const LIST_BY_INGREDIENT: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM product WHERE mapped_ingredient_id = ANY($1) AND archived_at IS NULL \
     ORDER BY mapped_ingredient_id, lower(name)"
);

pub struct PgProductRepository {
    pool: PgPool,
}

impl PgProductRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProductRepository for PgProductRepository {
    async fn get(&self, id: ProductId) -> Result<Option<Product>> {
        let row: Option<ProductRow> = sqlx::query_as(GET_BY_ID)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a product", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_barcode(&self, barcode: &str) -> Result<Option<Product>> {
        let row: Option<ProductRow> = sqlx::query_as(GET_BY_BARCODE)
            .bind(barcode)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("looking up a product by barcode", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_seed_key(&self, seed_key: &str) -> Result<Option<Product>> {
        let row: Option<ProductRow> = sqlx::query_as(GET_BY_SEED_KEY)
            .bind(seed_key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("looking up a product by seed key", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list(&self, query: &ProductQuery) -> Result<Paginated<Product>> {
        let origin = query.origin.map(|o| o.code());
        let search = query.search.as_deref();
        let mapped = query.mapped_ingredient_id.map(|id| id.as_uuid());
        let list_sql = match query.sort {
            SortDirection::Ascending => LIST_ASC,
            SortDirection::Descending => LIST_DESC,
        };

        let total: (i64,) = sqlx::query_as(COUNT)
            .bind(query.include_archived)
            .bind(origin)
            .bind(search)
            .bind(query.barcode.as_deref())
            .bind(query.retailer.as_deref())
            .bind(mapped)
            .bind(query.unmapped)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| repository_error("counting products", e))?;

        let rows: Vec<ProductRow> = sqlx::query_as(list_sql)
            .bind(query.include_archived)
            .bind(origin)
            .bind(search)
            .bind(query.barcode.as_deref())
            .bind(query.retailer.as_deref())
            .bind(mapped)
            .bind(query.unmapped)
            .bind(query.page.limit())
            .bind(query.page.offset())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing products", e))?;

        let items = rows
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<Product>>>()?;
        Ok(Paginated::new(items, total.0, query.page))
    }

    async fn count_by_ingredient(
        &self,
        ingredient_ids: &[IngredientId],
    ) -> Result<HashMap<IngredientId, i64>> {
        if ingredient_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let uuids: Vec<uuid::Uuid> = ingredient_ids.iter().map(|id| id.as_uuid()).collect();
        let rows: Vec<(uuid::Uuid, i64)> = sqlx::query_as(COUNT_BY_INGREDIENT)
            .bind(&uuids)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("counting products by ingredient", e))?;

        let mut counts: HashMap<IngredientId, i64> =
            ingredient_ids.iter().map(|id| (*id, 0)).collect();
        for (id, count) in rows {
            counts.insert(IngredientId::from(id), count);
        }
        Ok(counts)
    }

    async fn list_by_ingredient(
        &self,
        ingredient_ids: &[IngredientId],
    ) -> Result<HashMap<IngredientId, Vec<Product>>> {
        if ingredient_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let uuids: Vec<uuid::Uuid> = ingredient_ids.iter().map(|id| id.as_uuid()).collect();
        let rows: Vec<ProductRow> = sqlx::query_as(LIST_BY_INGREDIENT)
            .bind(&uuids)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing products by ingredient", e))?;

        let mut grouped: HashMap<IngredientId, Vec<Product>> = HashMap::new();
        for row in rows {
            let product: Product = row.try_into()?;
            if let Some(ingredient_id) = product.mapped_ingredient_id {
                grouped.entry(ingredient_id).or_default().push(product);
            }
        }
        Ok(grouped)
    }

    async fn insert(&self, product: &Product) -> Result<()> {
        let n = nutrition_bindings(&product.nutrition);
        sqlx::query(
            "INSERT INTO product (
                 id, name, brand, barcode, retailer, shopping_section,
                 package_quantity_amount, package_quantity_unit, servings_per_pack,
                 mapped_ingredient_id,
                 nutrition_basis_amount, nutrition_basis_unit,
                 energy_kcal, protein_g, carbohydrate_g, sugar_g, fat_g,
                 saturated_fat_g, fibre_g, salt_g, cholesterol_mg, nutrition_extra,
                 origin, seed_key, source_provider, source_external_id, locally_modified,
                 revision, created_at, updated_at, archived_at
             ) VALUES (
                 $1, $2, $3, $4, $5, $6,
                 $7, $8, $9,
                 $10,
                 $11, $12,
                 $13, $14, $15, $16, $17,
                 $18, $19, $20, $21, $22,
                 $23, $24, $25, $26, $27,
                 $28, $29, $30, $31
             )",
        )
        .bind(product.id.as_uuid())
        .bind(&product.name)
        .bind(&product.brand)
        .bind(&product.barcode)
        .bind(&product.retailer)
        .bind(&product.shopping_section)
        .bind(product.package_quantity.map(|q| q.amount))
        .bind(product.package_quantity.map(|q| q.unit.code()))
        .bind(product.servings_per_pack)
        .bind(product.mapped_ingredient_id.map(|id| id.as_uuid()))
        .bind(n.basis_amount)
        .bind(n.basis_unit)
        .bind(n.energy_kcal)
        .bind(n.protein_g)
        .bind(n.carbohydrate_g)
        .bind(n.sugar_g)
        .bind(n.fat_g)
        .bind(n.saturated_fat_g)
        .bind(n.fibre_g)
        .bind(n.salt_g)
        .bind(n.cholesterol_mg)
        .bind(n.extra)
        .bind(product.provenance.origin.code())
        .bind(&product.provenance.seed_key)
        .bind(&product.provenance.source_provider)
        .bind(&product.provenance.source_external_id)
        .bind(product.provenance.locally_modified)
        .bind(product.revision.get())
        .bind(product.created_at)
        .bind(product.updated_at)
        .bind(product.archived_at)
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "creating a product"))?;
        Ok(())
    }

    async fn update(&self, product: &Product, expected: Revision) -> Result<UpdateOutcome> {
        let n = nutrition_bindings(&product.nutrition);
        let affected = sqlx::query(
            "UPDATE product SET
                 name = $2, brand = $3, barcode = $4, retailer = $5, shopping_section = $6,
                 package_quantity_amount = $7, package_quantity_unit = $8,
                 servings_per_pack = $9,
                 mapped_ingredient_id = $10,
                 nutrition_basis_amount = $11, nutrition_basis_unit = $12,
                 energy_kcal = $13, protein_g = $14, carbohydrate_g = $15,
                 sugar_g = $16, fat_g = $17, saturated_fat_g = $18, fibre_g = $19, salt_g = $20,
                 cholesterol_mg = $21, nutrition_extra = $22,
                 origin = $23, seed_key = $24, source_provider = $25, source_external_id = $26,
                 locally_modified = $27,
                 revision = $28, updated_at = $29, archived_at = $30
             WHERE id = $1 AND revision = $31",
        )
        .bind(product.id.as_uuid())
        .bind(&product.name)
        .bind(&product.brand)
        .bind(&product.barcode)
        .bind(&product.retailer)
        .bind(&product.shopping_section)
        .bind(product.package_quantity.map(|q| q.amount))
        .bind(product.package_quantity.map(|q| q.unit.code()))
        .bind(product.servings_per_pack)
        .bind(product.mapped_ingredient_id.map(|id| id.as_uuid()))
        .bind(n.basis_amount)
        .bind(n.basis_unit)
        .bind(n.energy_kcal)
        .bind(n.protein_g)
        .bind(n.carbohydrate_g)
        .bind(n.sugar_g)
        .bind(n.fat_g)
        .bind(n.saturated_fat_g)
        .bind(n.fibre_g)
        .bind(n.salt_g)
        .bind(n.cholesterol_mg)
        .bind(n.extra)
        .bind(product.provenance.origin.code())
        .bind(&product.provenance.seed_key)
        .bind(&product.provenance.source_provider)
        .bind(&product.provenance.source_external_id)
        .bind(product.provenance.locally_modified)
        .bind(product.revision.get())
        .bind(product.updated_at)
        .bind(product.archived_at)
        .bind(expected.get())
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "updating a product"))?
        .rows_affected();

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }

        let current: Option<(i64,)> = sqlx::query_as(CURRENT_REVISION)
            .bind(product.id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("re-reading a product revision", e))?;

        Ok(match current {
            Some((actual,)) => UpdateOutcome::RevisionMismatch {
                actual: Revision::new(actual),
            },
            None => UpdateOutcome::NotFound,
        })
    }
}
