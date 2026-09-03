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
        track_stock: None,
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
