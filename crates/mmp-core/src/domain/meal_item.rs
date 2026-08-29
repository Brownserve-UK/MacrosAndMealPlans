use super::{ProductId, RecipeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MealItemRef {
    Product { product_id: ProductId },
    Recipe { recipe_id: RecipeId },
}

impl MealItemRef {
    pub const fn product(id: ProductId) -> Self {
        MealItemRef::Product { product_id: id }
    }

    pub const fn recipe(id: RecipeId) -> Self {
        MealItemRef::Recipe { recipe_id: id }
    }

    pub const fn kind_code(&self) -> &'static str {
        match self {
            MealItemRef::Product { .. } => "product",
            MealItemRef::Recipe { .. } => "recipe",
        }
    }

    pub const fn product_id(&self) -> Option<ProductId> {
        match self {
            MealItemRef::Product { product_id } => Some(*product_id),
            MealItemRef::Recipe { .. } => None,
        }
    }

    pub const fn recipe_id(&self) -> Option<RecipeId> {
        match self {
            MealItemRef::Recipe { recipe_id } => Some(*recipe_id),
            MealItemRef::Product { .. } => None,
        }
    }

    pub const fn is_recipe(&self) -> bool {
        matches!(self, MealItemRef::Recipe { .. })
    }

    pub fn from_parts(
        kind: &str,
        product_id: Option<ProductId>,
        recipe_id: Option<RecipeId>,
    ) -> Result<Self, UnknownMealItemRef> {
        match (kind, product_id, recipe_id) {
            ("product", Some(product_id), None) => Ok(MealItemRef::Product { product_id }),
            ("recipe", None, Some(recipe_id)) => Ok(MealItemRef::Recipe { recipe_id }),
            _ => Err(UnknownMealItemRef(kind.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a valid meal item reference")]
pub struct UnknownMealItemRef(pub String);
