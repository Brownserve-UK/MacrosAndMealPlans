use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    Mass,
    Volume,
    Count,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum Unit {
    #[serde(rename = "mg")]
    Milligram,
    #[serde(rename = "g")]
    Gram,
    #[serde(rename = "kg")]
    Kilogram,
    #[serde(rename = "oz")]
    Ounce,
    #[serde(rename = "lb")]
    Pound,
    #[serde(rename = "ml")]
    Millilitre,
    #[serde(rename = "l")]
    Litre,
    #[serde(rename = "tsp")]
    Teaspoon,
    #[serde(rename = "tbsp")]
    Tablespoon,
    #[serde(rename = "fl_oz")]
    FluidOunce,
    #[serde(rename = "cup")]
    Cup,
    #[serde(rename = "item")]
    Item,
    #[serde(rename = "piece")]
    Piece,
    #[serde(rename = "slice")]
    Slice,
    #[serde(rename = "clove")]
    Clove,
    #[serde(rename = "can")]
    Can,
    #[serde(rename = "pack")]
    Pack,
    #[serde(rename = "bunch")]
    Bunch,
}

impl Unit {
    pub const ALL: [Unit; 18] = [
        Unit::Milligram,
        Unit::Gram,
        Unit::Kilogram,
        Unit::Ounce,
        Unit::Pound,
        Unit::Millilitre,
        Unit::Litre,
        Unit::Teaspoon,
        Unit::Tablespoon,
        Unit::FluidOunce,
        Unit::Cup,
        Unit::Item,
        Unit::Piece,
        Unit::Slice,
        Unit::Clove,
        Unit::Can,
        Unit::Pack,
        Unit::Bunch,
    ];

    pub const fn dimension(&self) -> Dimension {
        match self {
            Unit::Milligram | Unit::Gram | Unit::Kilogram | Unit::Ounce | Unit::Pound => {
                Dimension::Mass
            }
            Unit::Millilitre
            | Unit::Litre
            | Unit::Teaspoon
            | Unit::Tablespoon
            | Unit::FluidOunce
            | Unit::Cup => Dimension::Volume,
            Unit::Item
            | Unit::Piece
            | Unit::Slice
            | Unit::Clove
            | Unit::Can
            | Unit::Pack
            | Unit::Bunch => Dimension::Count,
        }
    }

    pub const fn code(&self) -> &'static str {
        match self {
            Unit::Milligram => "mg",
            Unit::Gram => "g",
            Unit::Kilogram => "kg",
            Unit::Ounce => "oz",
            Unit::Pound => "lb",
            Unit::Millilitre => "ml",
            Unit::Litre => "l",
            Unit::Teaspoon => "tsp",
            Unit::Tablespoon => "tbsp",
            Unit::FluidOunce => "fl_oz",
            Unit::Cup => "cup",
            Unit::Item => "item",
            Unit::Piece => "piece",
            Unit::Slice => "slice",
            Unit::Clove => "clove",
            Unit::Can => "can",
            Unit::Pack => "pack",
            Unit::Bunch => "bunch",
        }
    }

    pub const fn label(&self) -> &'static str {
        match self {
            Unit::Milligram => "milligram",
            Unit::Gram => "gram",
            Unit::Kilogram => "kilogram",
            Unit::Ounce => "ounce",
            Unit::Pound => "pound",
            Unit::Millilitre => "millilitre",
            Unit::Litre => "litre",
            Unit::Teaspoon => "teaspoon",
            Unit::Tablespoon => "tablespoon",
            Unit::FluidOunce => "fluid ounce",
            Unit::Cup => "cup",
            Unit::Item => "item",
            Unit::Piece => "piece",
            Unit::Slice => "slice",
            Unit::Clove => "clove",
            Unit::Can => "can",
            Unit::Pack => "pack",
            Unit::Bunch => "bunch",
        }
    }

    // Base factor is the multiplier to convert a unit to the base unit of its dimension (gram for mass, millilitre for volume).
    // TODO - verify standards: metric teaspoon and tablespoon (5 ml / 15 ml), imperial fluid ounce
    // (28.4130625 ml) and US legal cup (240 ml).
    pub fn base_factor(&self) -> Option<Decimal> {
        let factor = match self {
            Unit::Milligram => Decimal::new(1, 3),
            Unit::Gram => Decimal::ONE,
            Unit::Kilogram => Decimal::new(1000, 0),
            Unit::Ounce => Decimal::new(28_349_523_125, 9),
            Unit::Pound => Decimal::new(45_359_237, 5),
            Unit::Millilitre => Decimal::ONE,
            Unit::Litre => Decimal::new(1000, 0),
            Unit::Teaspoon => Decimal::new(5, 0),
            Unit::Tablespoon => Decimal::new(15, 0),
            Unit::FluidOunce => Decimal::new(284_130_625, 7),
            Unit::Cup => Decimal::new(240, 0),
            _ => return None,
        };
        Some(factor)
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a known unit")]
pub struct UnknownUnit(pub String);

impl FromStr for Unit {
    type Err = UnknownUnit;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Unit::ALL
            .into_iter()
            .find(|u| u.code() == s)
            .ok_or_else(|| UnknownUnit(s.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConversionError {
    #[error("{from} and {to} measure different things, so there is no safe conversion")]
    IncompatibleDimensions { from: Unit, to: Unit },

    #[error(
        "converting {from} to {to} depends on the food involved, and no such conversion is known"
    )]
    ContextualConversionRequired { from: Unit, to: Unit },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Quantity {
    pub amount: Decimal,
    pub unit: Unit,
}

impl Quantity {
    pub const fn new(amount: Decimal, unit: Unit) -> Self {
        Self { amount, unit }
    }

    pub fn convert_to(&self, target: Unit) -> Result<Quantity, ConversionError> {
        if self.unit == target {
            return Ok(*self);
        }

        if self.unit.dimension() != target.dimension() {
            return Err(ConversionError::IncompatibleDimensions {
                from: self.unit,
                to: target,
            });
        }

        let (Some(from_factor), Some(to_factor)) = (self.unit.base_factor(), target.base_factor())
        else {
            return Err(ConversionError::ContextualConversionRequired {
                from: self.unit,
                to: target,
            });
        };

        Ok(Quantity::new(self.amount * from_factor / to_factor, target))
    }

    pub fn compare(&self, other: &Quantity) -> Result<std::cmp::Ordering, ConversionError> {
        let other_converted = other.convert_to(self.unit)?;
        Ok(self.amount.cmp(&other_converted.amount))
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.amount.normalize(), self.unit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    fn q(amount: i64, scale: u32, unit: Unit) -> Quantity {
        Quantity::new(Decimal::new(amount, scale), unit)
    }

    #[test]
    fn converts_within_mass() {
        let converted = q(2, 0, Unit::Kilogram).convert_to(Unit::Gram).unwrap();
        assert_eq!(converted.amount, Decimal::new(2000, 0));
        assert_eq!(converted.unit, Unit::Gram);
    }

    #[test]
    fn converts_within_volume() {
        let converted = q(15, 1, Unit::Litre).convert_to(Unit::Millilitre).unwrap();
        assert_eq!(converted.amount, Decimal::new(1500, 0));
    }

    #[test]
    fn converts_cooking_volumes() {
        assert_eq!(
            q(1, 0, Unit::Tablespoon)
                .convert_to(Unit::Teaspoon)
                .unwrap()
                .amount,
            Decimal::new(3, 0)
        );
    }

    #[test]
    fn refuses_mass_to_volume() {
        let err = q(100, 0, Unit::Gram)
            .convert_to(Unit::Millilitre)
            .unwrap_err();
        assert!(matches!(
            err,
            ConversionError::IncompatibleDimensions { .. }
        ));
    }

    #[test]
    fn refuses_count_to_count_as_contextual() {
        let err = q(1, 0, Unit::Bunch).convert_to(Unit::Item).unwrap_err();
        assert!(matches!(
            err,
            ConversionError::ContextualConversionRequired { .. }
        ));
    }

    #[test]
    fn identical_count_units_need_no_conversion() {
        let same = q(3, 0, Unit::Clove).convert_to(Unit::Clove).unwrap();
        assert_eq!(same.amount, Decimal::new(3, 0));
    }

    #[test]
    fn compares_across_compatible_units() {
        let ordering = q(1, 0, Unit::Kilogram)
            .compare(&q(999, 0, Unit::Gram))
            .unwrap();
        assert_eq!(ordering, Ordering::Greater);
    }

    #[test]
    fn comparison_is_unknown_without_a_safe_conversion() {
        assert!(q(85, 0, Unit::Gram).compare(&q(1, 0, Unit::Bunch)).is_err());
    }

    #[test]
    fn unit_codes_round_trip() {
        for unit in Unit::ALL {
            assert_eq!(Unit::from_str(unit.code()).unwrap(), unit);
        }
    }

    #[test]
    fn serde_representation_matches_the_unit_code() {
        for unit in Unit::ALL {
            let json = serde_json::to_string(&unit).unwrap();
            assert_eq!(json, format!("\"{}\"", unit.code()), "{unit:?}");
            let back: Unit = serde_json::from_str(&json).unwrap();
            assert_eq!(back, unit);
        }
    }

    #[test]
    fn count_units_have_no_base_factor() {
        for unit in Unit::ALL {
            assert_eq!(
                unit.base_factor().is_none(),
                unit.dimension() == Dimension::Count
            );
        }
    }
}
