use mmp_core::domain::{
    MealTemplate, MealTemplateComponent, MealTemplatePatch, NewMealTemplate,
    NewMealTemplateComponent, UserId,
};
use mmp_core::ports::{MealTemplateQuery, PageRequest, Paginated, SortDirection};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::common::PageMeta;
use super::consumption::AmountDto;
use super::meal_plan::{ItemRefRequest, MealItemRefDto};

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MealTemplateComponentRequest {
    #[serde(flatten)]
    pub item: ItemRefRequest,
    pub amount: AmountDto,
}

impl From<MealTemplateComponentRequest> for NewMealTemplateComponent {
    fn from(value: MealTemplateComponentRequest) -> Self {
        Self {
            item: value.item.into(),
            amount: value.amount.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealTemplateComponentDto {
    pub id: Uuid,
    #[serde(flatten)]
    pub item: MealItemRefDto,
    pub amount: AmountDto,
    pub position: i32,
}

impl From<MealTemplateComponent> for MealTemplateComponentDto {
    fn from(value: MealTemplateComponent) -> Self {
        Self {
            id: value.id.as_uuid(),
            item: value.item.into(),
            amount: value.amount.into(),
            position: value.position,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealTemplateDto {
    pub id: Uuid,
    #[schema(example = "Fish fingers, chips and peas")]
    pub name: String,
    pub components: Vec<MealTemplateComponentDto>,
    pub owner_id: Uuid,
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

impl From<MealTemplate> for MealTemplateDto {
    fn from(value: MealTemplate) -> Self {
        Self {
            id: value.id.as_uuid(),
            name: value.name,
            components: value.components.into_iter().map(Into::into).collect(),
            owner_id: value.owner_id.as_uuid(),
            revision: value.revision.get(),
            created_at: value.created_at,
            updated_at: value.updated_at,
            archived_at: value.archived_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateMealTemplateRequest {
    #[schema(example = "Fish fingers, chips and peas")]
    pub name: String,
    pub components: Vec<MealTemplateComponentRequest>,
}

impl CreateMealTemplateRequest {
    pub fn into_domain(self, owner_id: UserId) -> NewMealTemplate {
        NewMealTemplate {
            id: None,
            owner_id,
            name: self.name,
            components: self.components.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateMealTemplateRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub components: Option<Vec<MealTemplateComponentRequest>>,
}

impl From<UpdateMealTemplateRequest> for MealTemplatePatch {
    fn from(value: UpdateMealTemplateRequest) -> Self {
        Self {
            name: value.name,
            components: value
                .components
                .map(|components| components.into_iter().map(Into::into).collect()),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MealTemplatePage {
    pub items: Vec<MealTemplateDto>,
    #[serde(flatten)]
    pub meta: PageMeta,
}

impl From<Paginated<MealTemplate>> for MealTemplatePage {
    fn from(value: Paginated<MealTemplate>) -> Self {
        let meta = PageMeta::of(&value);
        Self {
            items: value.items.into_iter().map(Into::into).collect(),
            meta,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct MealTemplateListQuery {
    pub q: Option<String>,
    pub include_archived: Option<bool>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

impl MealTemplateListQuery {
    pub fn into_domain(self, owner_id: UserId) -> MealTemplateQuery {
        MealTemplateQuery {
            owner_id,
            search: self.q.filter(|q| !q.trim().is_empty()),
            include_archived: self.include_archived.unwrap_or(false),
            page: PageRequest::new(
                self.page.unwrap_or(1),
                self.per_page.unwrap_or(PageRequest::DEFAULT_PER_PAGE),
            ),
            sort: SortDirection::Ascending,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateMealTemplateFromEntryRequest {
    #[schema(example = "Fish fingers, chips and peas")]
    pub name: String,
}
