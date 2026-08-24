use time::OffsetDateTime;

use super::{
    IngredientId, NutritionFacts, Patch, ProductId, Provenance, Quantity, Revision, validate_name,
};
use crate::error::ValidationErrors;

pub const MAX_BARCODE_LEN: usize = 18;
pub const MIN_BARCODE_LEN: usize = 4;
pub const MAX_SHORT_TEXT_LEN: usize = 120;

// Product == a purchasable item that fulfils a recipe ingredient or component of a meal.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Product {
    pub id: ProductId,
    pub name: String,
    pub brand: Option<String>,
    pub barcode: Option<String>,
    pub retailer: Option<String>,
    pub shopping_section: Option<String>,
    pub package_quantity: Option<Quantity>,
    pub servings_per_pack: Option<i32>,
    pub mapped_ingredient_id: Option<IngredientId>,
    pub nutrition: NutritionFacts,
    pub provenance: Provenance,
    pub revision: Revision,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub archived_at: Option<OffsetDateTime>,
}

impl Product {
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    pub fn validate_invariants(&self) -> crate::error::Result<()> {
        let mut errors = ValidationErrors::new();

        if let (Some(basis), Some(package)) = (self.nutrition.basis, self.package_quantity)
            && basis.unit.dimension() != package.unit.dimension()
        {
            errors.push(
                "nutrition.basis",
                format!("Must match the pack, measured in {}", package.unit),
            );
        }

        if self.servings_per_pack.is_some() && self.package_quantity.is_none() {
            errors.push("servings_per_pack", "Needs a pack size to divide up");
        }

        errors.into_result()
    }
}

#[derive(Debug, Clone)]
pub struct NewProduct {
    pub id: Option<ProductId>,
    pub name: String,
    pub brand: Option<String>,
    pub barcode: Option<String>,
    pub retailer: Option<String>,
    pub shopping_section: Option<String>,
    pub package_quantity: Option<Quantity>,
    pub servings_per_pack: Option<i32>,
    pub mapped_ingredient_id: Option<IngredientId>,
    pub nutrition: NutritionFacts,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Default)]
pub struct ProductPatch {
    pub name: Option<String>,
    pub brand: Patch<String>,
    pub barcode: Patch<String>,
    pub retailer: Patch<String>,
    pub shopping_section: Patch<String>,
    pub package_quantity: Patch<Quantity>,
    pub servings_per_pack: Patch<i32>,
    pub nutrition: Option<NutritionFacts>,
}

impl ProductPatch {
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.brand.is_unchanged()
            && self.barcode.is_unchanged()
            && self.retailer.is_unchanged()
            && self.shopping_section.is_unchanged()
            && self.package_quantity.is_unchanged()
            && self.servings_per_pack.is_unchanged()
            && self.nutrition.is_none()
    }
}

fn validate_barcode(barcode: &str, errors: &mut ValidationErrors) {
    let trimmed = barcode.trim();
    if !trimmed.chars().all(|c| c.is_ascii_digit()) {
        errors.push("barcode", "Digits only");
    } else if trimmed.len() < MIN_BARCODE_LEN || trimmed.len() > MAX_BARCODE_LEN {
        errors.push(
            "barcode",
            format!("Must be {MIN_BARCODE_LEN} to {MAX_BARCODE_LEN} digits"),
        );
    }
}

fn validate_short_text(field: &str, value: &str, errors: &mut ValidationErrors) {
    if value.trim().chars().count() > MAX_SHORT_TEXT_LEN {
        errors.push(field, "Too long");
    }
}

fn validate_package_quantity(quantity: &Quantity, errors: &mut ValidationErrors) {
    if quantity.amount <= rust_decimal::Decimal::ZERO {
        errors.push("package_quantity.amount", "Must be more than zero");
    }
}

fn validate_servings_per_pack(servings: i32, errors: &mut ValidationErrors) {
    if servings <= 0 {
        errors.push("servings_per_pack", "Must be more than zero");
    }
}

impl NewProduct {
    pub fn validate(&self) -> crate::error::Result<()> {
        let mut errors = ValidationErrors::new();
        validate_name("name", &self.name, &mut errors);
        if let Some(brand) = &self.brand {
            validate_short_text("brand", brand, &mut errors);
        }
        if let Some(retailer) = &self.retailer {
            validate_short_text("retailer", retailer, &mut errors);
        }
        if let Some(section) = &self.shopping_section {
            validate_short_text("shopping_section", section, &mut errors);
        }
        if let Some(barcode) = &self.barcode {
            validate_barcode(barcode, &mut errors);
        }
        if let Some(quantity) = &self.package_quantity {
            validate_package_quantity(quantity, &mut errors);
        }
        if let Some(servings) = self.servings_per_pack {
            validate_servings_per_pack(servings, &mut errors);
        }
        self.nutrition.validate("nutrition", &mut errors);
        errors.into_result()
    }
}

impl ProductPatch {
    pub fn validate(&self) -> crate::error::Result<()> {
        let mut errors = ValidationErrors::new();
        if let Some(name) = &self.name {
            validate_name("name", name, &mut errors);
        }
        if let Patch::Set(brand) = self.brand.as_ref() {
            validate_short_text("brand", brand, &mut errors);
        }
        if let Patch::Set(retailer) = self.retailer.as_ref() {
            validate_short_text("retailer", retailer, &mut errors);
        }
        if let Patch::Set(section) = self.shopping_section.as_ref() {
            validate_short_text("shopping_section", section, &mut errors);
        }
        if let Patch::Set(barcode) = self.barcode.as_ref() {
            validate_barcode(barcode, &mut errors);
        }
        if let Patch::Set(quantity) = self.package_quantity.as_ref() {
            validate_package_quantity(quantity, &mut errors);
        }
        if let Patch::Set(servings) = self.servings_per_pack.as_ref() {
            validate_servings_per_pack(*servings, &mut errors);
        }
        if let Some(nutrition) = &self.nutrition {
            nutrition.validate("nutrition", &mut errors);
        }
        errors.into_result()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Unit;
    use rust_decimal::Decimal;

    fn new_product() -> NewProduct {
        NewProduct {
            id: None,
            name: "Tesco Whole Milk 1L".to_owned(),
            brand: Some("Tesco".to_owned()),
            barcode: Some("5000119012345".to_owned()),
            retailer: Some("Tesco".to_owned()),
            shopping_section: Some("Dairy".to_owned()),
            package_quantity: Some(Quantity::new(Decimal::new(1, 0), Unit::Litre)),
            servings_per_pack: None,
            mapped_ingredient_id: None,
            nutrition: NutritionFacts::default(),
            provenance: Provenance::local(),
        }
    }

    #[test]
    fn a_well_formed_product_validates() {
        assert!(new_product().validate().is_ok());
    }

    #[test]
    fn a_non_numeric_barcode_is_rejected() {
        let mut product = new_product();
        product.barcode = Some("50001A9012345".to_owned());
        assert!(product.validate().is_err());
    }

    #[test]
    fn a_short_barcode_is_rejected() {
        let mut product = new_product();
        product.barcode = Some("123".to_owned());
        assert!(product.validate().is_err());
    }

    #[test]
    fn a_zero_package_quantity_is_rejected() {
        let mut product = new_product();
        product.package_quantity = Some(Quantity::new(Decimal::ZERO, Unit::Litre));
        assert!(product.validate().is_err());
    }

    #[test]
    fn a_zero_servings_per_pack_is_rejected() {
        let mut product = new_product();
        product.servings_per_pack = Some(0);
        assert!(product.validate().is_err());
    }

    #[test]
    fn a_product_needs_neither_barcode_nor_package_size() {
        let mut product = new_product();
        product.barcode = None;
        product.package_quantity = None;
        assert!(product.validate().is_ok());
    }

    #[test]
    fn clearing_a_barcode_skips_format_validation() {
        let patch = ProductPatch {
            barcode: Patch::Clear,
            ..Default::default()
        };
        assert!(patch.validate().is_ok());
    }

    #[test]
    fn an_empty_patch_is_detected() {
        assert!(ProductPatch::default().is_empty());
    }
}
