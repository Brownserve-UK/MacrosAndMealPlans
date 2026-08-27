use mmp_core::domain::{
    ConsumedNutrition, NewRecipe, NewRecipeComponent, NutritionQuality, Recipe, RecipeComponent,
    RecipeId, RecipePatch, RecipeVisibility, UserId,
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
    pub amount: AmountDto,
    pub position: i32,
}

impl From<RecipeComponent> for RecipeComponentDto {
    fn from(value: RecipeComponent) -> Self {
        Self {
            id: value.id.as_uuid(),
            product_id: value.product_id.as_uuid(),
            amount: value.amount.into(),
            position: value.position,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RecipeDto {
    pub id: Uuid,
    #[schema(example = "Chicken Curry")]
    pub name: String,
    #[schema(example = 4)]
    pub servings: i32,
    pub components: Vec<RecipeComponentDto>,
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

impl From<Recipe> for RecipeDto {
    fn from(value: Recipe) -> Self {
        Self {
            id: value.id.as_uuid(),
            name: value.name,
            servings: value.servings,
            components: value.components.into_iter().map(Into::into).collect(),
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

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RecipeComponentRequest {
    #[serde(default)]
    pub id: Option<Uuid>,
    pub product_id: Uuid,
    pub amount: AmountDto,
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
    #[schema(example = 4)]
    pub servings: i32,
    pub components: Vec<RecipeComponentRequest>,
}

impl CreateRecipeRequest {
    pub fn into_domain(self, actor_id: UserId) -> NewRecipe {
        NewRecipe {
            id: self.id.map(RecipeId::from),
            name: self.name,
            servings: self.servings,
            components: self.components.into_iter().map(Into::into).collect(),
            actor_id,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateRecipeRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub servings: Option<i32>,
    #[serde(default)]
    pub components: Option<Vec<RecipeComponentRequest>>,
}

impl From<UpdateRecipeRequest> for RecipePatch {
    fn from(value: UpdateRecipeRequest) -> Self {
        Self {
            name: value.name,
            servings: value.servings,
            components: value
                .components
                .map(|components| components.into_iter().map(Into::into).collect()),
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
    pub items: Vec<RecipeDto>,
    #[serde(flatten)]
    pub meta: PageMeta,
}

impl From<Paginated<Recipe>> for RecipePage {
    fn from(value: Paginated<Recipe>) -> Self {
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
