use super::{IngredientId, PreparedMealId, ProductId, RecipeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MealItemRef {
    Product { product_id: ProductId },
    Recipe { recipe_id: RecipeId },
    Dish { recipe_id: RecipeId },
    Ingredient { ingredient_id: IngredientId },
    PreparedMeal { prepared_meal_id: PreparedMealId },
}

impl MealItemRef {
    pub const fn product(id: ProductId) -> Self {
        MealItemRef::Product { product_id: id }
    }

    pub const fn recipe(id: RecipeId) -> Self {
        MealItemRef::Recipe { recipe_id: id }
    }

    pub const fn dish(id: RecipeId) -> Self {
        MealItemRef::Dish { recipe_id: id }
    }

    pub const fn ingredient(id: IngredientId) -> Self {
        MealItemRef::Ingredient { ingredient_id: id }
    }

    pub const fn prepared_meal(id: PreparedMealId) -> Self {
        MealItemRef::PreparedMeal {
            prepared_meal_id: id,
        }
    }

    pub const fn kind_code(&self) -> &'static str {
        match self {
            MealItemRef::Product { .. } => "product",
            MealItemRef::Recipe { .. } => "recipe",
            MealItemRef::Dish { .. } => "dish",
            MealItemRef::Ingredient { .. } => "ingredient",
            MealItemRef::PreparedMeal { .. } => "prepared_meal",
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
            MealItemRef::Recipe { recipe_id } | MealItemRef::Dish { recipe_id } => Some(*recipe_id),
            _ => None,
        }
    }

    pub const fn ingredient_id(&self) -> Option<IngredientId> {
        match self {
            MealItemRef::Ingredient { ingredient_id } => Some(*ingredient_id),
            _ => None,
        }
    }

    pub const fn prepared_meal_id(&self) -> Option<PreparedMealId> {
        match self {
            MealItemRef::PreparedMeal { prepared_meal_id } => Some(*prepared_meal_id),
            _ => None,
        }
    }

    pub const fn is_recipe(&self) -> bool {
        matches!(self, MealItemRef::Recipe { .. })
    }

    pub const fn is_generic_food(&self) -> bool {
        matches!(
            self,
            MealItemRef::Ingredient { .. } | MealItemRef::PreparedMeal { .. }
        )
    }

    pub fn from_parts(
        kind: &str,
        product_id: Option<ProductId>,
        recipe_id: Option<RecipeId>,
        ingredient_id: Option<IngredientId>,
        prepared_meal_id: Option<PreparedMealId>,
    ) -> Result<Self, UnknownMealItemRef> {
        match (kind, product_id, recipe_id, ingredient_id, prepared_meal_id) {
            ("product", Some(product_id), None, None, None) => {
                Ok(MealItemRef::Product { product_id })
            }
            ("recipe", None, Some(recipe_id), None, None) => Ok(MealItemRef::Recipe { recipe_id }),
            ("dish", None, Some(recipe_id), None, None) => Ok(MealItemRef::Dish { recipe_id }),
            ("ingredient", None, None, Some(ingredient_id), None) => {
                Ok(MealItemRef::Ingredient { ingredient_id })
            }
            ("prepared_meal", None, None, None, Some(prepared_meal_id)) => {
                Ok(MealItemRef::PreparedMeal { prepared_meal_id })
            }
            _ => Err(UnknownMealItemRef(kind.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a valid meal item reference")]
pub struct UnknownMealItemRef(pub String);
