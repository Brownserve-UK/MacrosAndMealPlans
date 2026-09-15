use std::collections::BTreeMap;

use mmp_core::domain::NutritionFacts;
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

macro_rules! nutrient_fields {
    ($($field:ident),* $(,)?) => {
        #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
        pub struct NutritionDto {
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub basis: Option<super::common::QuantityDto>,
            $(
                #[serde(default, with = "rust_decimal::serde::float_option")]
                #[schema(value_type = Option<f64>)]
                pub $field: Option<Decimal>,
            )*
            #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
            pub extra: BTreeMap<String, f64>,
        }

        impl From<NutritionFacts> for NutritionDto {
            fn from(facts: NutritionFacts) -> Self {
                Self {
                    basis: facts.basis.map(Into::into),
                    $($field: facts.$field,)*
                    extra: facts
                        .extra
                        .into_iter()
                        .filter_map(|(k, v)| v.to_f64().map(|v| (k, v)))
                        .collect(),
                }
            }
        }

        impl From<NutritionDto> for NutritionFacts {
            fn from(dto: NutritionDto) -> Self {
                Self {
                    basis: dto.basis.map(Into::into),
                    $($field: dto.$field,)*
                    extra: dto
                        .extra
                        .into_iter()
                        .filter_map(|(k, v)| Decimal::from_f64(v).map(|v| (k, v)))
                        .collect(),
                }
            }
        }
    };
}

nutrient_fields!(
    energy_kcal,
    protein_g,
    carbohydrate_g,
    sugar_g,
    fat_g,
    saturated_fat_g,
    fibre_g,
    salt_g,
    cholesterol_mg,
);

#[cfg(test)]
#[path = "nutrition_tests.rs"]
mod tests;
