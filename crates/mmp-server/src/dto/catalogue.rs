use mmp_core::domain::{
    CatalogueOrigin, Ingredient, IngredientId, IngredientPatch, IngredientSummary, NewIngredient,
    NewProduct, Patch, Product, ProductPatch, Provenance, Unit,
};
use mmp_core::ports::Paginated;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::common::{PageMeta, ProvenanceDto, QuantityDto, SortDirectionDto};
use super::nutrition::NutritionDto;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IngredientDto {
    pub id: Uuid,
    #[schema(example = "Whole Milk")]
    pub name: String,
    pub default_unit: Unit,
    pub provenance: ProvenanceDto,
    #[schema(example = 1)]
    pub revision: i64,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
    #[serde(
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub archived_at: Option<OffsetDateTime>,
}

impl From<Ingredient> for IngredientDto {
    fn from(value: Ingredient) -> Self {
        Self {
            id: value.id.as_uuid(),
            name: value.name,
            default_unit: value.default_unit,
            provenance: value.provenance.into(),
            revision: value.revision.get(),
            created_at: value.created_at,
            updated_at: value.updated_at,
            archived_at: value.archived_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateIngredientRequest {
    #[serde(default)]
    pub id: Option<Uuid>,
    #[schema(example = "Whole Milk")]
    pub name: String,
    pub default_unit: Unit,
}

impl From<CreateIngredientRequest> for NewIngredient {
    fn from(value: CreateIngredientRequest) -> Self {
        Self {
            id: value.id.map(IngredientId::from),
            name: value.name,
            default_unit: value.default_unit,
            provenance: Provenance::local(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateIngredientRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub default_unit: Option<Unit>,
}

impl From<UpdateIngredientRequest> for IngredientPatch {
    fn from(value: UpdateIngredientRequest) -> Self {
        Self {
            name: value.name,
            default_unit: value.default_unit,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IngredientListItemDto {
    #[serde(flatten)]
    pub ingredient: IngredientDto,
    #[schema(example = 2)]
    pub mapped_product_count: i64,
}

impl From<IngredientSummary> for IngredientListItemDto {
    fn from(value: IngredientSummary) -> Self {
        Self {
            ingredient: value.ingredient.into(),
            mapped_product_count: value.mapped_product_count,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IngredientPage {
    pub items: Vec<IngredientListItemDto>,
    #[serde(flatten)]
    pub meta: PageMeta,
}

impl From<Paginated<IngredientSummary>> for IngredientPage {
    fn from(value: Paginated<IngredientSummary>) -> Self {
        let meta = PageMeta::of(&value);
        Self {
            items: value.items.into_iter().map(Into::into).collect(),
            meta,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProductDto {
    pub id: Uuid,
    #[schema(example = "Tesco Whole Milk 1L")]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "5000119012345")]
    pub barcode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retailer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shopping_section: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_quantity: Option<QuantityDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 6)]
    pub servings_per_pack: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapped_ingredient_id: Option<Uuid>,
    pub nutrition: NutritionDto,
    pub provenance: ProvenanceDto,
    pub revision: i64,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
    #[serde(
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub archived_at: Option<OffsetDateTime>,
}

impl From<Product> for ProductDto {
    fn from(value: Product) -> Self {
        Self {
            id: value.id.as_uuid(),
            name: value.name,
            brand: value.brand,
            barcode: value.barcode,
            retailer: value.retailer,
            shopping_section: value.shopping_section,
            package_quantity: value.package_quantity.map(Into::into),
            servings_per_pack: value.servings_per_pack,
            mapped_ingredient_id: value.mapped_ingredient_id.map(|id| id.as_uuid()),
            nutrition: value.nutrition.into(),
            provenance: value.provenance.into(),
            revision: value.revision.get(),
            created_at: value.created_at,
            updated_at: value.updated_at,
            archived_at: value.archived_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateProductRequest {
    #[serde(default)]
    pub id: Option<Uuid>,
    #[schema(example = "Tesco Whole Milk 1L")]
    pub name: String,
    #[serde(default)]
    pub brand: Option<String>,
    #[serde(default)]
    pub barcode: Option<String>,
    #[serde(default)]
    pub retailer: Option<String>,
    #[serde(default)]
    pub shopping_section: Option<String>,
    #[serde(default)]
    pub package_quantity: Option<QuantityDto>,
    #[serde(default)]
    pub servings_per_pack: Option<i32>,
    #[serde(default)]
    pub mapped_ingredient_id: Option<Uuid>,
    #[serde(default)]
    pub nutrition: NutritionDto,
}

impl From<CreateProductRequest> for NewProduct {
    fn from(value: CreateProductRequest) -> Self {
        Self {
            id: value.id.map(Into::into),
            name: value.name,
            brand: value.brand,
            barcode: value.barcode,
            retailer: value.retailer,
            shopping_section: value.shopping_section,
            package_quantity: value.package_quantity.map(Into::into),
            servings_per_pack: value.servings_per_pack,
            mapped_ingredient_id: value.mapped_ingredient_id.map(IngredientId::from),
            nutrition: value.nutrition.into(),
            provenance: Provenance::local(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateProductRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub brand: Patch<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub barcode: Patch<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub retailer: Patch<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub shopping_section: Patch<String>,
    #[serde(default)]
    #[schema(value_type = Option<QuantityDto>)]
    pub package_quantity: Patch<QuantityDto>,
    #[serde(default)]
    #[schema(value_type = Option<i32>)]
    pub servings_per_pack: Patch<i32>,
    #[serde(default)]
    pub nutrition: Option<NutritionDto>,
}

impl From<UpdateProductRequest> for ProductPatch {
    fn from(value: UpdateProductRequest) -> Self {
        Self {
            name: value.name,
            brand: value.brand,
            barcode: value.barcode,
            retailer: value.retailer,
            shopping_section: value.shopping_section,
            package_quantity: value.package_quantity.map(Into::into),
            servings_per_pack: value.servings_per_pack,
            nutrition: value.nutrition.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetMappingRequest {
    pub ingredient_id: Uuid,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProductPage {
    pub items: Vec<ProductDto>,
    #[serde(flatten)]
    pub meta: PageMeta,
}

impl From<Paginated<Product>> for ProductPage {
    fn from(value: Paginated<Product>) -> Self {
        let meta = PageMeta::of(&value);
        Self {
            items: value.items.into_iter().map(Into::into).collect(),
            meta,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct IngredientListQuery {
    pub q: Option<String>,
    pub origin: Option<CatalogueOrigin>,
    pub include_archived: Option<bool>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub sort: Option<SortDirectionDto>,
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ProductListQuery {
    pub q: Option<String>,
    pub origin: Option<CatalogueOrigin>,
    pub barcode: Option<String>,
    pub retailer: Option<String>,
    pub mapped_ingredient_id: Option<Uuid>,
    pub include_archived: Option<bool>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub sort: Option<SortDirectionDto>,
}
