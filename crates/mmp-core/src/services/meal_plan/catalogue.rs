use std::collections::HashMap;

use crate::domain::{
    ConsumedAmount, ConsumedNutrition, MealItemRef, PreparedBatch, Product, ProductId, Recipe,
    RecipeId, RecipeRequirement, nutrition_for, recipe_nutrition, recipe_nutrition_for,
};
use crate::error::Result;

use super::MealPlanService;
use crate::services::fulfilment::{RecipeFulfilments, RecipeWant, expand_recipe};

pub(super) struct RecipeCard {
    pub(super) name: String,
    pub(super) per_serving: ConsumedNutrition,
    pub(super) archived: bool,
}

pub(super) struct ResolvedItem {
    pub(super) name: String,
    pub(super) nutrition: ConsumedNutrition,
    pub(super) resolvable: bool,
}

pub(super) struct ItemCatalogue {
    pub(super) products: HashMap<ProductId, Product>,
    pub(super) recipes: HashMap<RecipeId, RecipeCard>,
    pub(super) definitions: HashMap<RecipeId, Recipe>,
    pub(super) dishes: HashMap<RecipeId, PreparedBatch>,
    pub(super) fulfilments: RecipeFulfilments,
}

impl ItemCatalogue {
    pub(super) fn name_of(&self, item: MealItemRef) -> String {
        match item {
            MealItemRef::Product { product_id } => self
                .products
                .get(&product_id)
                .map(|product| product.name.clone())
                .unwrap_or_else(|| "Missing product".to_owned()),
            MealItemRef::Recipe { recipe_id } => self
                .recipes
                .get(&recipe_id)
                .map(|card| card.name.clone())
                .unwrap_or_else(|| "Missing recipe".to_owned()),
            MealItemRef::Dish { recipe_id } => self
                .dishes
                .get(&recipe_id)
                .map(|batch| batch.item_name.clone())
                .unwrap_or_else(|| "Missing cooked food".to_owned()),
        }
    }

    pub(super) fn recipe_wants(
        &self,
        recipe_id: RecipeId,
        amount: &ConsumedAmount,
    ) -> Vec<RecipeWant> {
        let Some(recipe) = self.definitions.get(&recipe_id) else {
            return Vec::new();
        };
        let ConsumedAmount::Servings(servings) = *amount else {
            return Vec::new();
        };
        expand_recipe(recipe, servings, &self.fulfilments).wants
    }

    pub(super) fn resolve(&self, item: MealItemRef, amount: &ConsumedAmount) -> ResolvedItem {
        match item {
            MealItemRef::Product { product_id } => match self.products.get(&product_id) {
                Some(product) => ResolvedItem {
                    name: product.name.clone(),
                    nutrition: nutrition_for(product, amount),
                    resolvable: amount.resolve(product).is_ok(),
                },
                None => ResolvedItem {
                    name: "Missing product".to_owned(),
                    nutrition: ConsumedNutrition::unknown(),
                    resolvable: false,
                },
            },
            MealItemRef::Recipe { recipe_id } => match self.recipes.get(&recipe_id) {
                Some(card) => ResolvedItem {
                    name: card.name.clone(),
                    nutrition: recipe_nutrition_for(&card.per_serving, amount),
                    resolvable: matches!(amount, ConsumedAmount::Servings(_)) && !card.archived,
                },
                None => ResolvedItem {
                    name: "Missing recipe".to_owned(),
                    nutrition: ConsumedNutrition::unknown(),
                    resolvable: false,
                },
            },
            MealItemRef::Dish { recipe_id } => match self.dishes.get(&recipe_id) {
                Some(batch) => ResolvedItem {
                    name: batch.item_name.clone(),
                    nutrition: recipe_nutrition_for(&batch.nutrition, amount),
                    resolvable: matches!(amount, ConsumedAmount::Servings(_)),
                },
                None => ResolvedItem {
                    name: "Missing cooked food".to_owned(),
                    nutrition: ConsumedNutrition::unknown(),
                    resolvable: false,
                },
            },
        }
    }
}

fn recipe_card(recipe: &Recipe, fulfilments: &RecipeFulfilments) -> RecipeCard {
    let per_serving = recipe_nutrition(
        recipe
            .components
            .iter()
            .map(|component| (&component.amount, fulfilments.get(&component.requirement))),
        recipe.servings,
    );
    RecipeCard {
        name: recipe.name.clone(),
        per_serving,
        archived: recipe.is_archived(),
    }
}

impl MealPlanService {
    pub(super) async fn catalogue_for(
        &self,
        items: impl IntoIterator<Item = MealItemRef>,
    ) -> Result<ItemCatalogue> {
        let mut product_ids: Vec<ProductId> = Vec::new();
        let mut recipe_ids: Vec<RecipeId> = Vec::new();
        let mut dish_ids: Vec<RecipeId> = Vec::new();
        for item in items {
            match item {
                MealItemRef::Product { product_id } => product_ids.push(product_id),
                MealItemRef::Recipe { recipe_id } => recipe_ids.push(recipe_id),
                MealItemRef::Dish { recipe_id } => dish_ids.push(recipe_id),
            }
        }

        let recipes = self.recipes.get_many(&recipe_ids).await?;
        let requirements: Vec<&RecipeRequirement> = recipes
            .iter()
            .flat_map(|recipe| {
                recipe
                    .components
                    .iter()
                    .map(|component| &component.requirement)
            })
            .collect();
        let fulfilments = RecipeFulfilments::load(&*self.products, &requirements).await?;

        let products: HashMap<ProductId, Product> = self
            .products
            .get_many(&product_ids)
            .await?
            .into_iter()
            .map(|product| (product.id, product))
            .collect();
        let cards: HashMap<RecipeId, RecipeCard> = recipes
            .iter()
            .map(|recipe| (recipe.id, recipe_card(recipe, &fulfilments)))
            .collect();
        let definitions: HashMap<RecipeId, Recipe> = recipes
            .into_iter()
            .map(|recipe| (recipe.id, recipe))
            .collect();

        let mut dishes: HashMap<RecipeId, PreparedBatch> = HashMap::new();
        for recipe_id in dish_ids {
            if dishes.contains_key(&recipe_id) {
                continue;
            }
            if let Some(batch) = self
                .batches
                .held_for_recipe(recipe_id)
                .await?
                .into_iter()
                .next()
            {
                dishes.insert(recipe_id, batch);
            }
        }

        Ok(ItemCatalogue {
            products,
            recipes: cards,
            definitions,
            dishes,
            fulfilments,
        })
    }
}
