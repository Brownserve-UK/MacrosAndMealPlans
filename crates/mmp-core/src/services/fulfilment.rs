use std::collections::HashMap;

use crate::domain::{Fulfilment, IngredientId, Product, ProductId, RecipeRequirement};
use crate::error::Result;
use crate::ports::ProductRepository;

pub(crate) struct RecipeFulfilments {
    pinned: HashMap<ProductId, Product>,
    candidates: HashMap<IngredientId, Vec<Product>>,
    empty: Vec<Product>,
}

impl RecipeFulfilments {
    pub(crate) async fn load(
        products: &dyn ProductRepository,
        requirements: &[&RecipeRequirement],
    ) -> Result<Self> {
        let mut pinned_ids = Vec::new();
        let mut ingredient_ids = Vec::new();
        for &requirement in requirements {
            match requirement {
                RecipeRequirement::Product { product_id } => pinned_ids.push(*product_id),
                RecipeRequirement::Ingredient { ingredient_id } => {
                    ingredient_ids.push(*ingredient_id)
                }
                RecipeRequirement::Unresolved { .. } => {}
            }
        }

        let pinned = products
            .get_many(&pinned_ids)
            .await?
            .into_iter()
            .map(|product| (product.id, product))
            .collect();
        let candidates = products.list_by_ingredient(&ingredient_ids).await?;

        Ok(Self {
            pinned,
            candidates,
            empty: Vec::new(),
        })
    }

    pub(crate) fn get(&self, requirement: &RecipeRequirement) -> Fulfilment<'_> {
        match requirement {
            RecipeRequirement::Product { product_id } => self
                .pinned
                .get(product_id)
                .map_or(Fulfilment::None, Fulfilment::Pinned),
            RecipeRequirement::Ingredient { ingredient_id } => {
                Fulfilment::Candidates(self.candidates.get(ingredient_id).unwrap_or(&self.empty))
            }
            RecipeRequirement::Unresolved { .. } => Fulfilment::None,
        }
    }
}
