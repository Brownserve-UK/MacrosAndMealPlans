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
mod tests {
    use super::*;
    use mmp_core::domain::{Quantity, Unit};

    fn per_100g() -> Quantity {
        Quantity::new(Decimal::new(100, 0), Unit::Gram)
    }

    #[test]
    fn unknown_nutrition_round_trips_as_null_not_zero() {
        let dto = NutritionDto::from(NutritionFacts::default());
        let json = serde_json::to_value(&dto).unwrap();
        assert!(json.get("energy_kcal").unwrap().is_null());
        assert!(json.get("basis").is_none());
    }

    #[test]
    fn values_are_json_numbers_not_strings() {
        let facts = NutritionFacts {
            basis: Some(per_100g()),
            energy_kcal: Some(Decimal::new(645, 1)),
            ..Default::default()
        };
        let json = serde_json::to_value(NutritionDto::from(facts)).unwrap();
        assert_eq!(json["energy_kcal"], serde_json::json!(64.5));
    }

    #[test]
    fn the_basis_survives_the_round_trip() {
        let facts = NutritionFacts {
            basis: Some(Quantity::new(Decimal::new(30, 0), Unit::Gram)),
            energy_kcal: Some(Decimal::new(120, 0)),
            ..Default::default()
        };
        let back: NutritionFacts = NutritionDto::from(facts).into();
        assert_eq!(
            back.basis,
            Some(Quantity::new(Decimal::new(30, 0), Unit::Gram))
        );
    }

    #[test]
    fn a_zero_survives_the_round_trip_as_a_known_value() {
        let facts = NutritionFacts {
            basis: Some(per_100g()),
            fat_g: Some(Decimal::ZERO),
            ..Default::default()
        };
        let back: NutritionFacts = NutritionDto::from(facts).into();
        assert_eq!(back.fat_g, Some(Decimal::ZERO));
        assert!(!back.is_unknown());
    }

    #[test]
    fn extra_nutrients_survive_the_round_trip() {
        let mut facts = NutritionFacts {
            basis: Some(per_100g()),
            ..Default::default()
        };
        facts
            .extra
            .insert("vitamin_c_mg".to_owned(), Decimal::new(125, 1));
        let back: NutritionFacts = NutritionDto::from(facts).into();
        assert_eq!(back.extra.get("vitamin_c_mg"), Some(&Decimal::new(125, 1)));
    }

    #[test]
    fn absent_fields_deserialise_as_unknown() {
        let dto: NutritionDto = serde_json::from_str("{}").unwrap();
        assert_eq!(dto.energy_kcal, None);
        let facts: NutritionFacts = dto.into();
        assert!(facts.is_unknown());
    }
}
