use mmp_core::domain::{
    MealCategory, NewRecipe, NewRecipeComponent, NewRecipeInstruction, NutritionQuality, Patch,
    Recipe, RecipeComponent, RecipeId, RecipeInstruction, RecipePatch, RecipeRequirement,
    RecipeSummary, RecipeVisibility, UserId,
};
use mmp_core::ports::Paginated;
use mmp_core::services::{
    NutritionGapReason, RecipeNames, RecipeNutrition, RecipeNutritionGap, ResolveRequirement,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::common::{PageMeta, SortDirectionDto};
use super::consumption::AmountDto;
use super::nutrition::NutritionDto;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecipeRequirementDto {
    Ingredient { ingredient_id: Uuid },
    Product { product_id: Uuid },
    Unresolved { text: String },
}

impl From<RecipeRequirementDto> for RecipeRequirement {
    fn from(value: RecipeRequirementDto) -> Self {
        match value {
            RecipeRequirementDto::Ingredient { ingredient_id } => RecipeRequirement::Ingredient {
                ingredient_id: ingredient_id.into(),
            },
            RecipeRequirementDto::Product { product_id } => RecipeRequirement::Product {
                product_id: product_id.into(),
            },
            RecipeRequirementDto::Unresolved { text } => RecipeRequirement::Unresolved { text },
        }
    }
}

impl From<RecipeRequirement> for RecipeRequirementDto {
    fn from(value: RecipeRequirement) -> Self {
        match value {
            RecipeRequirement::Ingredient { ingredient_id } => RecipeRequirementDto::Ingredient {
                ingredient_id: ingredient_id.as_uuid(),
            },
            RecipeRequirement::Product { product_id } => RecipeRequirementDto::Product {
                product_id: product_id.as_uuid(),
            },
            RecipeRequirement::Unresolved { text } => RecipeRequirementDto::Unresolved { text },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ComponentNutritionSource {
    Known,
    Estimated,
    None,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RecipeComponentDto {
    pub id: Uuid,
    pub requirement: RecipeRequirementDto,
    pub name: String,
    pub source_text: Option<String>,
    pub nutrition_source: ComponentNutritionSource,
    pub candidate_product_count: Option<i64>,
    pub amount: AmountDto,
    pub position: i32,
}

impl RecipeComponentDto {
    fn from_domain(value: RecipeComponent, names: &RecipeNames) -> Self {
        let (name, nutrition_source, candidate_product_count) = match &value.requirement {
            RecipeRequirement::Product { product_id } => (
                names
                    .products
                    .get(product_id)
                    .cloned()
                    .unwrap_or_else(|| "Unknown product".to_owned()),
                ComponentNutritionSource::Known,
                None,
            ),
            RecipeRequirement::Ingredient { ingredient_id } => {
                let count = names
                    .candidate_counts
                    .get(ingredient_id)
                    .copied()
                    .unwrap_or(0);
                let source = if count > 0 {
                    ComponentNutritionSource::Estimated
                } else {
                    ComponentNutritionSource::None
                };
                (
                    names
                        .ingredients
                        .get(ingredient_id)
                        .cloned()
                        .unwrap_or_else(|| "Unknown ingredient".to_owned()),
                    source,
                    Some(count),
                )
            }
            RecipeRequirement::Unresolved { text } => {
                (text.clone(), ComponentNutritionSource::None, None)
            }
        };
        Self {
            id: value.id.as_uuid(),
            requirement: value.requirement.into(),
            name,
            source_text: value.source_text,
            nutrition_source,
            candidate_product_count,
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
    pub fn from_domain(value: Recipe, names: &RecipeNames) -> Self {
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
                .map(|component| RecipeComponentDto::from_domain(component, names))
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
    pub unresolved_count: i64,
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
            unresolved_count: value.unresolved_count,
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
    pub requirement: RecipeRequirementDto,
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
            requirement: value.requirement.into(),
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
    pub gaps: Vec<RecipeNutritionGapDto>,
}

impl From<RecipeNutrition> for RecipeNutritionDto {
    fn from(value: RecipeNutrition) -> Self {
        Self {
            nutrition: value.consumed.facts.into(),
            quality: value.consumed.quality,
            gaps: value
                .gaps
                .into_iter()
                .map(RecipeNutritionGapDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RecipeNutritionGapDto {
    pub component_id: Option<Uuid>,
    pub name: String,
    pub reason: NutritionGapReasonDto,
}

impl From<RecipeNutritionGap> for RecipeNutritionGapDto {
    fn from(value: RecipeNutritionGap) -> Self {
        Self {
            component_id: value.component_id.map(|id| id.as_uuid()),
            name: value.name,
            reason: value.reason.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NutritionGapReasonDto {
    Unmatched,
    NoData,
    Incomplete,
}

impl From<NutritionGapReason> for NutritionGapReasonDto {
    fn from(value: NutritionGapReason) -> Self {
        match value {
            NutritionGapReason::Unmatched => NutritionGapReasonDto::Unmatched,
            NutritionGapReason::NoData => NutritionGapReasonDto::NoData,
            NutritionGapReason::Incomplete => NutritionGapReasonDto::Incomplete,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResolveComponentRequest {
    Ingredient { ingredient_id: Uuid },
    Product { product_id: Uuid },
}

impl From<ResolveComponentRequest> for ResolveRequirement {
    fn from(value: ResolveComponentRequest) -> Self {
        match value {
            ResolveComponentRequest::Ingredient { ingredient_id } => {
                ResolveRequirement::Ingredient {
                    ingredient_id: ingredient_id.into(),
                }
            }
            ResolveComponentRequest::Product { product_id } => ResolveRequirement::Product {
                product_id: product_id.into(),
            },
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
