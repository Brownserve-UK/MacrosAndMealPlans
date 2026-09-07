use super::{PreparedBatchId, ProductId, RecipeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MealItemRef {
    Product { product_id: ProductId },
    Recipe { recipe_id: RecipeId },
    Dish { prepared_batch_id: PreparedBatchId },
}

impl MealItemRef {
    pub const fn product(id: ProductId) -> Self {
        MealItemRef::Product { product_id: id }
    }

    pub const fn recipe(id: RecipeId) -> Self {
        MealItemRef::Recipe { recipe_id: id }
    }

    pub const fn dish(id: PreparedBatchId) -> Self {
        MealItemRef::Dish {
            prepared_batch_id: id,
        }
    }

    pub const fn kind_code(&self) -> &'static str {
        match self {
            MealItemRef::Product { .. } => "product",
            MealItemRef::Recipe { .. } => "recipe",
            MealItemRef::Dish { .. } => "dish",
        }
    }

    pub const fn product_id(&self) -> Option<ProductId> {
        match self {
            MealItemRef::Product { product_id } => Some(*product_id),
            _ => None,
        }
    }

    pub const fn recipe_id(&self) -> Option<RecipeId> {
        match self {
            MealItemRef::Recipe { recipe_id } => Some(*recipe_id),
            _ => None,
        }
    }

    pub const fn prepared_batch_id(&self) -> Option<PreparedBatchId> {
        match self {
            MealItemRef::Dish { prepared_batch_id } => Some(*prepared_batch_id),
            _ => None,
        }
    }

    pub const fn is_recipe(&self) -> bool {
        matches!(self, MealItemRef::Recipe { .. })
    }

    pub const fn is_dish(&self) -> bool {
        matches!(self, MealItemRef::Dish { .. })
    }

    pub fn from_parts(
        kind: &str,
        product_id: Option<ProductId>,
        recipe_id: Option<RecipeId>,
        prepared_batch_id: Option<PreparedBatchId>,
    ) -> Result<Self, UnknownMealItemRef> {
        match (kind, product_id, recipe_id, prepared_batch_id) {
            ("product", Some(product_id), None, None) => Ok(MealItemRef::Product { product_id }),
            ("recipe", None, Some(recipe_id), None) => Ok(MealItemRef::Recipe { recipe_id }),
            ("dish", None, None, Some(prepared_batch_id)) => {
                Ok(MealItemRef::Dish { prepared_batch_id })
            }
            _ => Err(UnknownMealItemRef(kind.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a valid meal item reference")]
pub struct UnknownMealItemRef(pub String);
