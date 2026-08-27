use mmp_core::domain::{
    ConsumedNutrition, MealCategory, NewRecipe, NewRecipeComponent, NewRecipeInstruction,
    NutritionQuality, Patch, Product, Recipe, RecipeComponent, RecipeId, RecipeInstruction,
    RecipePatch, RecipeSummary, RecipeVisibility, UserId,
};
use mmp_core::ports::Paginated;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::common::{PageMeta, SortDirectionDto};
use super::consumption::AmountDto;
use super::nutrition::NutritionDto;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RecipeComponentDto {
    pub id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
    pub amount: AmountDto,
    pub position: i32,
}

impl RecipeComponentDto {
    fn from_domain(value: RecipeComponent, products: &[Product]) -> Self {
        let product_name = products
            .iter()
            .find(|product| product.id == value.product_id)
            .map(|product| product.name.clone())
            .unwrap_or_else(|| "Unknown product".to_owned());
        Self {
            id: value.id.as_uuid(),
            product_id: value.product_id.as_uuid(),
            product_name,
            amount: value.amount.into(),
            position: value.position,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RecipeInstructionDto {
    pub id: Uuid,
    pub text: String,
    pub position: i32,
}

impl From<RecipeInstruction> for RecipeInstructionDto {
    fn from(value: RecipeInstruction) -> Self {
        Self {
            id: value.id.as_uuid(),
            text: value.text,
            position: value.position,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RecipeDto {
    pub id: Uuid,
    #[schema(example = "Chicken Curry")]
    pub name: String,
    pub description: Option<String>,
    #[schema(example = 4)]
    pub servings: i32,
    pub preparation_minutes: Option<i32>,
    pub cooking_minutes: Option<i32>,
    pub notes: Option<String>,
    pub components: Vec<RecipeComponentDto>,
    pub instructions: Vec<RecipeInstructionDto>,
    pub meal_categories: Vec<MealCategory>,
    pub country_categories: Vec<String>,
    pub tags: Vec<String>,
    pub photo_version: Option<i64>,
    pub owner_id: Uuid,
    pub visibility: RecipeVisibility,
    pub created_by: Uuid,
    pub updated_by: Uuid,
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

impl RecipeDto {
    pub fn from_domain(value: Recipe, products: &[Product]) -> Self {
        Self {
            id: value.id.as_uuid(),
            name: value.name,
            description: value.description,
            servings: value.servings,
            preparation_minutes: value.preparation_minutes,
            cooking_minutes: value.cooking_minutes,
            notes: value.notes,
            components: value
                .components
                .into_iter()
                .map(|component| RecipeComponentDto::from_domain(component, products))
                .collect(),
            instructions: value.instructions.into_iter().map(Into::into).collect(),
            meal_categories: value.meal_categories,
            country_categories: value.country_categories,
            tags: value.tags,
            photo_version: value.photo_version,
            owner_id: value.owner_id.as_uuid(),
            visibility: value.visibility,
            created_by: value.created_by.as_uuid(),
            updated_by: value.updated_by.as_uuid(),
            revision: value.revision.get(),
            created_at: value.created_at,
            updated_at: value.updated_at,
            archived_at: value.archived_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RecipeSummaryDto {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub servings: i32,
    pub preparation_minutes: Option<i32>,
    pub cooking_minutes: Option<i32>,
    pub component_count: i64,
    pub meal_categories: Vec<MealCategory>,
    pub country_categories: Vec<String>,
    pub tags: Vec<String>,
    pub photo_version: Option<i64>,
    pub revision: i64,
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

impl From<RecipeSummary> for RecipeSummaryDto {
    fn from(value: RecipeSummary) -> Self {
        Self {
            id: value.id.as_uuid(),
            name: value.name,
            description: value.description,
            servings: value.servings,
            preparation_minutes: value.preparation_minutes,
            cooking_minutes: value.cooking_minutes,
            component_count: value.component_count,
            meal_categories: value.meal_categories,
            country_categories: value.country_categories,
            tags: value.tags,
            photo_version: value.photo_version,
            revision: value.revision.get(),
            updated_at: value.updated_at,
            archived_at: value.archived_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RecipeComponentRequest {
    #[serde(default)]
    pub id: Option<Uuid>,
    pub product_id: Uuid,
    pub amount: AmountDto,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RecipeInstructionRequest {
    #[serde(default)]
    pub id: Option<Uuid>,
    pub text: String,
}

impl From<RecipeInstructionRequest> for NewRecipeInstruction {
    fn from(value: RecipeInstructionRequest) -> Self {
        Self {
            id: value.id.map(Into::into),
            text: value.text,
        }
    }
}

impl From<RecipeComponentRequest> for NewRecipeComponent {
    fn from(value: RecipeComponentRequest) -> Self {
        Self {
            id: value.id.map(Into::into),
            product_id: value.product_id.into(),
            amount: value.amount.into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateRecipeRequest {
    #[serde(default)]
    pub id: Option<Uuid>,
    #[schema(example = "Chicken Curry")]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[schema(example = 4)]
    pub servings: i32,
    #[serde(default)]
    pub preparation_minutes: Option<i32>,
    #[serde(default)]
    pub cooking_minutes: Option<i32>,
    #[serde(default)]
    pub notes: Option<String>,
    pub components: Vec<RecipeComponentRequest>,
    #[serde(default)]
    pub instructions: Vec<RecipeInstructionRequest>,
    #[serde(default)]
    pub meal_categories: Vec<MealCategory>,
    #[serde(default)]
    pub country_categories: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl CreateRecipeRequest {
    pub fn into_domain(self, actor_id: UserId) -> NewRecipe {
        NewRecipe {
            id: self.id.map(RecipeId::from),
            name: self.name,
            description: self.description,
            servings: self.servings,
            preparation_minutes: self.preparation_minutes,
            cooking_minutes: self.cooking_minutes,
            notes: self.notes,
            components: self.components.into_iter().map(Into::into).collect(),
            instructions: self.instructions.into_iter().map(Into::into).collect(),
            meal_categories: self.meal_categories,
            country_categories: self.country_categories,
            tags: self.tags,
            actor_id,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateRecipeRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub description: Patch<String>,
    #[serde(default)]
    pub servings: Option<i32>,
    #[serde(default)]
    #[schema(value_type = Option<i32>)]
    pub preparation_minutes: Patch<i32>,
    #[serde(default)]
    #[schema(value_type = Option<i32>)]
    pub cooking_minutes: Patch<i32>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub notes: Patch<String>,
    #[serde(default)]
    pub components: Option<Vec<RecipeComponentRequest>>,
    #[serde(default)]
    pub instructions: Option<Vec<RecipeInstructionRequest>>,
    #[serde(default)]
    pub meal_categories: Option<Vec<MealCategory>>,
    #[serde(default)]
    pub country_categories: Option<Vec<String>>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

impl From<UpdateRecipeRequest> for RecipePatch {
    fn from(value: UpdateRecipeRequest) -> Self {
        Self {
            name: value.name,
            description: value.description,
            servings: value.servings,
            preparation_minutes: value.preparation_minutes,
            cooking_minutes: value.cooking_minutes,
            notes: value.notes,
            components: value
                .components
                .map(|components| components.into_iter().map(Into::into).collect()),
            instructions: value
                .instructions
                .map(|instructions| instructions.into_iter().map(Into::into).collect()),
            meal_categories: value.meal_categories,
            country_categories: value.country_categories,
            tags: value.tags,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RecipeNutritionDto {
    pub nutrition: NutritionDto,
    pub quality: NutritionQuality,
}

impl From<ConsumedNutrition> for RecipeNutritionDto {
    fn from(value: ConsumedNutrition) -> Self {
        Self {
            nutrition: value.facts.into(),
            quality: value.quality,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RecipeNutritionPreviewRequest {
    #[schema(example = 4)]
    pub servings: i32,
    pub components: Vec<RecipeComponentRequest>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RecipePage {
    pub items: Vec<RecipeSummaryDto>,
    #[serde(flatten)]
    pub meta: PageMeta,
}

impl From<Paginated<RecipeSummary>> for RecipePage {
    fn from(value: Paginated<RecipeSummary>) -> Self {
        let meta = PageMeta::of(&value);
        Self {
            items: value.items.into_iter().map(Into::into).collect(),
            meta,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RecipeListQuery {
    pub q: Option<String>,
    pub include_archived: Option<bool>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub sort: Option<SortDirectionDto>,
}
