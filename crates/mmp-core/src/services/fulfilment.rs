use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::domain::{
    ConsumedAmount, DeductionTarget, DemandGap, DemandSubject, Fulfilment, IngredientId, Product,
    ProductId, Quantity, Recipe, RecipeComponentId, RecipeRequirement,
};
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

pub(crate) struct RecipeWant {
    pub recipe_component_id: RecipeComponentId,
    pub target: DeductionTarget,
    pub want: Quantity,
}

#[derive(Default)]
pub(crate) struct RecipeExpansion {
    pub wants: Vec<RecipeWant>,
    pub subject_gaps: Vec<(DemandSubject, DemandGap)>,
    pub loose_gaps: Vec<DemandGap>,
}

pub(crate) fn expand_recipe(
    recipe: &Recipe,
    servings: Decimal,
    fulfilments: &RecipeFulfilments,
) -> RecipeExpansion {
    let mut out = RecipeExpansion::default();
    let scale = servings / Decimal::from(recipe.servings.max(1));

    for component in &recipe.components {
        let scaled = scale_amount(&component.amount, scale);
        match &component.requirement {
            RecipeRequirement::Unresolved { .. } => {
                out.loose_gaps.push(DemandGap::UnresolvedRecipeLine);
            }
            RecipeRequirement::Product { product_id } => {
                let subject = DemandSubject::product(*product_id);
                let Fulfilment::Pinned(product) = fulfilments.get(&component.requirement) else {
                    out.subject_gaps.push((subject, DemandGap::ProductMissing));
                    continue;
                };
                match scaled.resolve(product) {
                    Ok(want) => out.wants.push(RecipeWant {
                        recipe_component_id: component.id,
                        target: DeductionTarget::product(*product_id),
                        want,
                    }),
                    Err(_) => out
                        .subject_gaps
                        .push((subject, DemandGap::AmountUnresolvable)),
                }
            }
            RecipeRequirement::Ingredient { ingredient_id } => {
                let subject = DemandSubject::ingredient(*ingredient_id);
                let Fulfilment::Candidates(products) = fulfilments.get(&component.requirement)
                else {
                    out.subject_gaps
                        .push((subject, DemandGap::IngredientHasNoProducts));
                    continue;
                };
                if products.is_empty() {
                    out.subject_gaps
                        .push((subject, DemandGap::IngredientHasNoProducts));
                    continue;
                }
                let ConsumedAmount::Measure(want) = scaled else {
                    out.subject_gaps
                        .push((subject, DemandGap::AmountUnresolvable));
                    continue;
                };
                out.wants.push(RecipeWant {
                    recipe_component_id: component.id,
                    target: DeductionTarget::pool(
                        *ingredient_id,
                        products.iter().map(|product| product.id).collect(),
                    ),
                    want,
                });
            }
        }
    }

    out
}

fn scale_amount(amount: &ConsumedAmount, scale: Decimal) -> ConsumedAmount {
    match amount {
        ConsumedAmount::Measure(quantity) => {
            ConsumedAmount::Measure(Quantity::new(quantity.amount * scale, quantity.unit))
        }
        ConsumedAmount::Servings(value) => ConsumedAmount::Servings(value * scale),
        ConsumedAmount::Packs(value) => ConsumedAmount::Packs(value * scale),
    }
}
