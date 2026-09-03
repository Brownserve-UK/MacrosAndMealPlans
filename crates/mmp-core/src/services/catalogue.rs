use std::sync::Arc;

use time::OffsetDateTime;

use crate::domain::{
    CatalogueOrigin, ConsumedAmount, ConsumedNutrition, Ingredient, IngredientId, IngredientPatch,
    IngredientSummary, NewIngredient, NewProduct, Product, ProductId, ProductPatch, Provenance,
    Revision, nutrition_for,
};
use crate::error::{CoreError, Result, ValidationErrors};
use crate::ports::{
    Clock, IngredientQuery, IngredientRepository, Paginated, ProductQuery, ProductRepository,
    UpdateOutcome,
};

const INGREDIENT: &str = "ingredient";
const PRODUCT: &str = "product";

#[derive(Clone)]
pub struct CatalogueService {
    ingredients: Arc<dyn IngredientRepository>,
    products: Arc<dyn ProductRepository>,
    clock: Arc<dyn Clock>,
}

impl CatalogueService {
    pub fn new(
        ingredients: Arc<dyn IngredientRepository>,
        products: Arc<dyn ProductRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            ingredients,
            products,
            clock,
        }
    }

    pub async fn create_ingredient(&self, input: NewIngredient) -> Result<Ingredient> {
        input.validate()?;
        let name = input.name.trim().to_owned();
        self.ensure_ingredient_name_free(&name, None).await?;

        let now = self.clock.now();
        let ingredient = Ingredient {
            id: input.id.unwrap_or_default(),
            name,
            default_unit: input.default_unit,
            shopping_section: input.shopping_section,
            track_stock: input.track_stock,
            provenance: input.provenance,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
            archived_at: None,
        };

        self.ingredients.insert(&ingredient).await?;
        Ok(ingredient)
    }

    pub async fn get_ingredient(&self, id: IngredientId) -> Result<Ingredient> {
        self.ingredients
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(INGREDIENT, id))
    }

    pub async fn list_ingredients(
        &self,
        query: &IngredientQuery,
    ) -> Result<Paginated<IngredientSummary>> {
        let page = self.ingredients.list(query).await?;
        let ids: Vec<IngredientId> = page.items.iter().map(|i| i.id).collect();
        let counts = self.products.count_by_ingredient(&ids).await?;

        let items = page
            .items
            .into_iter()
            .map(|ingredient| IngredientSummary {
                mapped_product_count: counts.get(&ingredient.id).copied().unwrap_or(0),
                ingredient,
            })
            .collect();

        Ok(Paginated {
            items,
            total: page.total,
            page: page.page,
            per_page: page.per_page,
        })
    }

    pub async fn count_products_for_ingredient(&self, id: IngredientId) -> Result<i64> {
        Ok(self
            .products
            .count_by_ingredient(&[id])
            .await?
            .get(&id)
            .copied()
            .unwrap_or(0))
    }

    pub async fn update_ingredient(
        &self,
        id: IngredientId,
        expected: Revision,
        patch: IngredientPatch,
    ) -> Result<Ingredient> {
        patch.validate()?;
        let mut current = self.get_ingredient(id).await?;
        require_revision(INGREDIENT, id, expected, current.revision)?;

        if patch.is_empty() {
            return Ok(current);
        }

        if let Some(name) = patch.name {
            let name = name.trim().to_owned();
            if !name.eq_ignore_ascii_case(&current.name) {
                self.ensure_ingredient_name_free(&name, Some(id)).await?;
            }
            current.name = name;
        }
        if let Some(unit) = patch.default_unit {
            current.default_unit = unit;
        }
        current.shopping_section = patch.shopping_section.apply(current.shopping_section);
        current.track_stock = patch.track_stock.apply(current.track_stock);

        self.stamp_update(
            &mut current.provenance,
            &mut current.revision,
            &mut current.updated_at,
        );
        self.commit_ingredient(&current, expected).await?;
        Ok(current)
    }

    pub async fn set_ingredient_archived(
        &self,
        id: IngredientId,
        expected: Revision,
        archived: bool,
    ) -> Result<Ingredient> {
        let mut current = self.get_ingredient(id).await?;
        require_revision(INGREDIENT, id, expected, current.revision)?;

        if current.is_archived() == archived {
            return Ok(current);
        }

        let now = self.clock.now();
        current.archived_at = archived.then_some(now);
        self.stamp_update(
            &mut current.provenance,
            &mut current.revision,
            &mut current.updated_at,
        );
        self.commit_ingredient(&current, expected).await?;
        Ok(current)
    }

    pub async fn create_product(&self, input: NewProduct) -> Result<Product> {
        input.validate()?;
        let name = input.name.trim().to_owned();
        let barcode = normalise_optional(input.barcode);

        if let Some(barcode) = &barcode {
            self.ensure_barcode_free(barcode, None).await?;
        }
        if let Some(ingredient_id) = input.mapped_ingredient_id {
            self.ensure_mappable_ingredient(ingredient_id).await?;
        }

        let now = self.clock.now();
        let product = Product {
            id: input.id.unwrap_or_default(),
            name,
            brand: normalise_optional(input.brand),
            barcode,
            retailer: normalise_optional(input.retailer),
            shopping_section: normalise_optional(input.shopping_section),
            track_stock: input.track_stock,
            package_quantity: input.package_quantity,
            servings_per_pack: input.servings_per_pack,
            mapped_ingredient_id: input.mapped_ingredient_id,
            nutrition: input.nutrition,
            provenance: input.provenance,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
            archived_at: None,
        };

        product.validate_invariants()?;
        self.products.insert(&product).await?;
        Ok(product)
    }

    pub async fn get_product(&self, id: ProductId) -> Result<Product> {
        self.products
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(PRODUCT, id))
    }

    pub async fn list_products(&self, query: &ProductQuery) -> Result<Paginated<Product>> {
        self.products.list(query).await
    }

    pub async fn nutrition_for(
        &self,
        id: ProductId,
        amount: &ConsumedAmount,
    ) -> Result<ConsumedNutrition> {
        let product = self.get_product(id).await?;
        Ok(nutrition_for(&product, amount))
    }

    pub async fn update_product(
        &self,
        id: ProductId,
        expected: Revision,
        patch: ProductPatch,
    ) -> Result<Product> {
        patch.validate()?;
        let mut current = self.get_product(id).await?;
        require_revision(PRODUCT, id, expected, current.revision)?;

        if patch.is_empty() {
            return Ok(current);
        }

        if let Some(name) = patch.name {
            current.name = name.trim().to_owned();
        }
        current.brand = patch
            .brand
            .map(|v| v.trim().to_owned())
            .apply(current.brand);
        current.retailer = patch
            .retailer
            .map(|v| v.trim().to_owned())
            .apply(current.retailer);
        current.shopping_section = patch
            .shopping_section
            .map(|v| v.trim().to_owned())
            .apply(current.shopping_section);
        current.track_stock = patch.track_stock.apply(current.track_stock);
        current.package_quantity = patch.package_quantity.apply(current.package_quantity);
        current.servings_per_pack = patch.servings_per_pack.apply(current.servings_per_pack);

        let new_barcode = patch
            .barcode
            .map(|v| v.trim().to_owned())
            .apply(current.barcode.clone());
        if new_barcode != current.barcode
            && let Some(barcode) = &new_barcode
        {
            self.ensure_barcode_free(barcode, Some(id)).await?;
        }
        current.barcode = new_barcode;

        if let Some(nutrition) = patch.nutrition {
            current.nutrition = nutrition;
        }

        current.validate_invariants()?;
        self.stamp_update(
            &mut current.provenance,
            &mut current.revision,
            &mut current.updated_at,
        );
        self.commit_product(&current, expected).await?;
        Ok(current)
    }

    pub async fn set_product_archived(
        &self,
        id: ProductId,
        expected: Revision,
        archived: bool,
    ) -> Result<Product> {
        let mut current = self.get_product(id).await?;
        require_revision(PRODUCT, id, expected, current.revision)?;

        if current.is_archived() == archived {
            return Ok(current);
        }

        let now = self.clock.now();
        current.archived_at = archived.then_some(now);
        self.stamp_update(
            &mut current.provenance,
            &mut current.revision,
            &mut current.updated_at,
        );
        self.commit_product(&current, expected).await?;
        Ok(current)
    }

    pub async fn set_product_mapping(
        &self,
        id: ProductId,
        expected: Revision,
        ingredient_id: Option<IngredientId>,
    ) -> Result<Product> {
        let mut current = self.get_product(id).await?;
        require_revision(PRODUCT, id, expected, current.revision)?;

        if let Some(ingredient_id) = ingredient_id {
            self.ensure_mappable_ingredient(ingredient_id).await?;
        }

        if current.mapped_ingredient_id == ingredient_id {
            return Ok(current);
        }

        current.mapped_ingredient_id = ingredient_id;
        self.stamp_update(
            &mut current.provenance,
            &mut current.revision,
            &mut current.updated_at,
        );
        self.commit_product(&current, expected).await?;
        Ok(current)
    }

    pub(crate) fn now(&self) -> OffsetDateTime {
        self.clock.now()
    }

    pub(crate) async fn find_ingredient_by_seed_key(
        &self,
        seed_key: &str,
    ) -> Result<Option<Ingredient>> {
        self.ingredients.find_by_seed_key(seed_key).await
    }

    pub(crate) async fn insert_ingredient(&self, ingredient: &Ingredient) -> Result<()> {
        self.ingredients.insert(ingredient).await
    }

    pub(crate) async fn commit_seeded_ingredient(
        &self,
        ingredient: &Ingredient,
        expected: Revision,
    ) -> Result<()> {
        self.commit_ingredient(ingredient, expected).await
    }

    async fn commit_ingredient(&self, ingredient: &Ingredient, expected: Revision) -> Result<()> {
        match self.ingredients.update(ingredient, expected).await? {
            UpdateOutcome::Updated => Ok(()),
            UpdateOutcome::RevisionMismatch { actual } => Err(CoreError::RevisionMismatch {
                resource: INGREDIENT,
                id: ingredient.id.to_string(),
                expected,
                actual,
            }),
            UpdateOutcome::NotFound => Err(CoreError::not_found(INGREDIENT, ingredient.id)),
        }
    }

    async fn commit_product(&self, product: &Product, expected: Revision) -> Result<()> {
        match self.products.update(product, expected).await? {
            UpdateOutcome::Updated => Ok(()),
            UpdateOutcome::RevisionMismatch { actual } => Err(CoreError::RevisionMismatch {
                resource: PRODUCT,
                id: product.id.to_string(),
                expected,
                actual,
            }),
            UpdateOutcome::NotFound => Err(CoreError::not_found(PRODUCT, product.id)),
        }
    }

    fn stamp_update(
        &self,
        provenance: &mut Provenance,
        revision: &mut Revision,
        updated_at: &mut OffsetDateTime,
    ) {
        if provenance.origin != CatalogueOrigin::Local {
            provenance.locally_modified = true;
        }
        *revision = revision.next();
        *updated_at = self.clock.now();
    }

    async fn ensure_ingredient_name_free(
        &self,
        name: &str,
        allow: Option<IngredientId>,
    ) -> Result<()> {
        if let Some(existing) = self.ingredients.find_by_name(name).await?
            && Some(existing.id) != allow
        {
            return Err(CoreError::duplicate(INGREDIENT, "name", name));
        }
        Ok(())
    }

    async fn ensure_barcode_free(&self, barcode: &str, allow: Option<ProductId>) -> Result<()> {
        if let Some(existing) = self.products.find_by_barcode(barcode).await?
            && Some(existing.id) != allow
        {
            return Err(CoreError::duplicate(PRODUCT, "barcode", barcode));
        }
        Ok(())
    }

    async fn ensure_mappable_ingredient(&self, id: IngredientId) -> Result<()> {
        let ingredient = self.get_ingredient(id).await?;
        if ingredient.is_archived() {
            let mut errors = ValidationErrors::new();
            errors.push("mapped_ingredient_id", "That ingredient is archived");
            return errors.into_result();
        }
        Ok(())
    }
}

fn require_revision(
    resource: &'static str,
    id: impl std::fmt::Display,
    expected: Revision,
    actual: Revision,
) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(CoreError::RevisionMismatch {
            resource,
            id: id.to_string(),
            expected,
            actual,
        })
    }
}

fn normalise_optional(value: Option<String>) -> Option<String> {
    value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

#[cfg(test)]
#[path = "catalogue_tests.rs"]
mod tests;
