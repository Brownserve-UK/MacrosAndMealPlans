use std::str::FromStr;

use rust_decimal::Decimal;

use super::*;

fn decimal(value: &str) -> Decimal {
    Decimal::from_str(value).unwrap()
}

#[test]
fn suggested_macros_use_the_emphasis_and_round_to_five_grams() {
    let general =
        suggest_macro_targets(decimal("80"), decimal("2000"), NutritionEmphasis::General).unwrap();
    let muscle =
        suggest_macro_targets(decimal("80"), decimal("2000"), NutritionEmphasis::Muscle).unwrap();
    let endurance =
        suggest_macro_targets(decimal("80"), decimal("2000"), NutritionEmphasis::Endurance)
            .unwrap();

    assert_eq!(
        general,
        MacroTargets {
            protein_g: decimal("130"),
            carbohydrate_g: decimal("235"),
            fat_g: decimal("60")
        }
    );
    assert_eq!(
        muscle,
        MacroTargets {
            protein_g: decimal("175"),
            carbohydrate_g: decimal("200"),
            fat_g: decimal("55")
        }
    );
    assert_eq!(
        endurance,
        MacroTargets {
            protein_g: decimal("130"),
            carbohydrate_g: decimal("260"),
            fat_g: decimal("50")
        }
    );
}

#[test]
fn manually_defined_macros_must_all_be_positive() {
    let result = MacroTargets {
        protein_g: Decimal::ZERO,
        carbohydrate_g: decimal("200"),
        fat_g: decimal("60"),
    }
    .validate();

    assert!(result.is_err());
}
