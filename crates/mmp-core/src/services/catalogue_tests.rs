use std::sync::Arc;

use crate::CoreError;
use crate::domain::{
    CatalogueOrigin, IngredientPatch, NewIngredient, NewProduct, NutritionFacts, Patch,
    ProductPatch, Provenance, Quantity, Revision, Unit,
};
use crate::ports::{FixedClock, IngredientQuery, PageRequest};
use crate::services::CatalogueService;
use crate::testing::{InMemoryIngredientRepository, InMemoryProductRepository};
use rust_decimal::Decimal;
use time::OffsetDateTime;
use time::macros::datetime;

struct Harness {
    service: CatalogueService,
    ingredients: InMemoryIngredientRepository,
    products: InMemoryProductRepository,
}

fn harness() -> Harness {
    harness_at(datetime!(2026-08-22 09:00 UTC))
}

fn harness_at(now: OffsetDateTime) -> Harness {
    let ingredients = InMemoryIngredientRepository::new();
    let products = InMemoryProductRepository::new();
    let service = CatalogueService::new(
        Arc::new(ingredients.clone()),
        Arc::new(products.clone()),
        Arc::new(FixedClock::new(now)),
    );
    Harness {
        service,
        ingredients,
        products,
    }
}

fn new_ingredient(name: &str) -> NewIngredient {
    NewIngredient {
        id: None,
        name: name.to_owned(),
        default_unit: Unit::Gram,
        shopping_section: None,
        track_stock: None,
        provenance: Provenance::local(),
    }
}

fn new_product(name: &str) -> NewProduct {
    NewProduct {
        id: None,
        name: name.to_owned(),
        brand: None,
        barcode: None,
        retailer: None,
        shopping_section: None,
        track_stock: None,
        package_quantity: None,
        servings_per_pack: None,
        mapped_ingredient_id: None,
        nutrition: NutritionFacts::default(),
        provenance: Provenance::local(),
    }
}

#[tokio::test]
async fn creates_an_ingredient_at_the_initial_revision() {
    let h = harness();
    let created = h
        .service
        .create_ingredient(new_ingredient("Whole Milk"))
        .await
        .unwrap();

    assert_eq!(created.name, "Whole Milk");
    assert_eq!(created.revision, Revision::INITIAL);
    assert_eq!(h.ingredients.count(), 1);
}

#[tokio::test]
async fn trims_whitespace_from_names() {
    let h = harness();
    let created = h
        .service
        .create_ingredient(new_ingredient("  Basmati Rice  "))
        .await
        .unwrap();
    assert_eq!(created.name, "Basmati Rice");
}

#[tokio::test]
async fn rejects_a_duplicate_ingredient_name_regardless_of_case() {
    let h = harness();
    h.service
        .create_ingredient(new_ingredient("Whole Milk"))
        .await
        .unwrap();
    let err = h
        .service
        .create_ingredient(new_ingredient("whole milk"))
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Duplicate { field: "name", .. }));
}

#[tokio::test]
async fn updating_with_a_stale_revision_conflicts() {
    let h = harness();
    let created = h
        .service
        .create_ingredient(new_ingredient("Whole Milk"))
        .await
        .unwrap();

    let patch = IngredientPatch {
        name: Some("Semi Skimmed Milk".to_owned()),
        ..Default::default()
    };
    h.service
        .update_ingredient(created.id, created.revision, patch.clone())
        .await
        .unwrap();

    let err = h
        .service
        .update_ingredient(created.id, created.revision, patch)
        .await
        .unwrap_err();

    match err {
        CoreError::RevisionMismatch {
            expected, actual, ..
        } => {
            assert_eq!(expected, Revision::INITIAL);
            assert_eq!(actual, Revision::new(2));
        }
        other => panic!("expected a revision mismatch, got {other:?}"),
    }
}

#[tokio::test]
async fn a_successful_update_advances_the_revision() {
    let h = harness();
    let created = h
        .service
        .create_ingredient(new_ingredient("Whole Milk"))
        .await
        .unwrap();
    let updated = h
        .service
        .update_ingredient(
            created.id,
            created.revision,
            IngredientPatch {
                default_unit: Some(Unit::Millilitre),
                shopping_section: Default::default(),
                track_stock: Default::default(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.revision, Revision::new(2));
    assert_eq!(updated.default_unit, Unit::Millilitre);
}

#[tokio::test]
async fn an_empty_patch_leaves_the_revision_alone() {
    let h = harness();
    let created = h
        .service
        .create_ingredient(new_ingredient("Whole Milk"))
        .await
        .unwrap();
    let updated = h
        .service
        .update_ingredient(created.id, created.revision, IngredientPatch::default())
        .await
        .unwrap();
    assert_eq!(updated.revision, Revision::INITIAL);
}

#[tokio::test]
async fn editing_a_seeded_ingredient_marks_it_locally_modified() {
    let h = harness();
    let created = h
        .service
        .create_ingredient(NewIngredient {
            provenance: Provenance::seeded("whole-milk"),
            ..new_ingredient("Whole Milk")
        })
        .await
        .unwrap();

    assert!(created.provenance.accepts_seed_refresh());

    let updated = h
        .service
        .update_ingredient(
            created.id,
            created.revision,
            IngredientPatch {
                name: Some("Full Fat Milk".to_owned()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.provenance.origin, CatalogueOrigin::Seeded);
    assert!(updated.provenance.locally_modified);
    assert!(!updated.provenance.accepts_seed_refresh());
}

#[tokio::test]
async fn editing_a_local_ingredient_does_not_set_the_modified_flag() {
    let h = harness();
    let created = h
        .service
        .create_ingredient(new_ingredient("Whole Milk"))
        .await
        .unwrap();
    let updated = h
        .service
        .update_ingredient(
            created.id,
            created.revision,
            IngredientPatch {
                name: Some("Cow Milk".to_owned()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(!updated.provenance.locally_modified);
}

#[tokio::test]
async fn renaming_to_its_own_name_is_allowed() {
    let h = harness();
    let created = h
        .service
        .create_ingredient(new_ingredient("Whole Milk"))
        .await
        .unwrap();
    let updated = h
        .service
        .update_ingredient(
            created.id,
            created.revision,
            IngredientPatch {
                name: Some("Whole Milk".to_owned()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.name, "Whole Milk");
}

#[tokio::test]
async fn archiving_hides_an_ingredient_from_the_default_listing() {
    let h = harness();
    let created = h
        .service
        .create_ingredient(new_ingredient("Whole Milk"))
        .await
        .unwrap();
    h.service
        .set_ingredient_archived(created.id, created.revision, true)
        .await
        .unwrap();

    let visible = h
        .service
        .list_ingredients(&IngredientQuery::default())
        .await
        .unwrap();
    assert_eq!(visible.total, 0);

    let all = h
        .service
        .list_ingredients(&IngredientQuery {
            include_archived: true,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(all.total, 1);
}

#[tokio::test]
async fn maps_a_product_to_an_ingredient() {
    let h = harness();
    let milk = h
        .service
        .create_ingredient(new_ingredient("Whole Milk"))
        .await
        .unwrap();
    let product = h
        .service
        .create_product(new_product("Tesco Whole Milk 1L"))
        .await
        .unwrap();

    let mapped = h
        .service
        .set_product_mapping(product.id, product.revision, Some(milk.id))
        .await
        .unwrap();

    assert_eq!(mapped.mapped_ingredient_id, Some(milk.id));
    assert_eq!(mapped.revision, Revision::new(2));
}

#[tokio::test]
async fn clears_a_product_mapping() {
    let h = harness();
    let milk = h
        .service
        .create_ingredient(new_ingredient("Whole Milk"))
        .await
        .unwrap();
    let product = h
        .service
        .create_product(new_product("Tesco Whole Milk 1L"))
        .await
        .unwrap();
    let mapped = h
        .service
        .set_product_mapping(product.id, product.revision, Some(milk.id))
        .await
        .unwrap();

    let cleared = h
        .service
        .set_product_mapping(mapped.id, mapped.revision, None)
        .await
        .unwrap();
    assert_eq!(cleared.mapped_ingredient_id, None);
}

#[tokio::test]
async fn refuses_to_map_a_product_to_an_unknown_ingredient() {
    let h = harness();
    let product = h
        .service
        .create_product(new_product("Mystery Milk"))
        .await
        .unwrap();
    let missing = crate::domain::IngredientId::new();

    let err = h
        .service
        .set_product_mapping(product.id, product.revision, Some(missing))
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::NotFound { .. }));
}

#[tokio::test]
async fn refuses_to_map_a_product_to_an_archived_ingredient() {
    let h = harness();
    let milk = h
        .service
        .create_ingredient(new_ingredient("Whole Milk"))
        .await
        .unwrap();
    h.service
        .set_ingredient_archived(milk.id, milk.revision, true)
        .await
        .unwrap();
    let product = h
        .service
        .create_product(new_product("Tesco Whole Milk 1L"))
        .await
        .unwrap();

    let err = h
        .service
        .set_product_mapping(product.id, product.revision, Some(milk.id))
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn rejects_a_duplicate_barcode() {
    let h = harness();
    h.service
        .create_product(NewProduct {
            barcode: Some("5000119012345".to_owned()),
            ..new_product("Tesco Whole Milk 1L")
        })
        .await
        .unwrap();

    let err = h
        .service
        .create_product(NewProduct {
            barcode: Some("5000119012345".to_owned()),
            ..new_product("Tesco Whole Milk 2L")
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        CoreError::Duplicate {
            field: "barcode",
            ..
        }
    ));
}

#[tokio::test]
async fn clearing_an_optional_product_field_is_distinct_from_leaving_it_alone() {
    let h = harness();
    let product = h
        .service
        .create_product(NewProduct {
            brand: Some("Tesco".to_owned()),
            retailer: Some("Tesco".to_owned()),
            package_quantity: Some(Quantity::new(Decimal::ONE, Unit::Litre)),
            ..new_product("Tesco Whole Milk 1L")
        })
        .await
        .unwrap();

    let updated = h
        .service
        .update_product(
            product.id,
            product.revision,
            ProductPatch {
                brand: Patch::Clear,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.brand, None);
    assert_eq!(updated.retailer.as_deref(), Some("Tesco"));
    assert!(updated.package_quantity.is_some());
}

#[tokio::test]
async fn lists_products_filtered_by_mapped_ingredient() {
    let h = harness();
    let milk = h
        .service
        .create_ingredient(new_ingredient("Whole Milk"))
        .await
        .unwrap();
    let tesco = h
        .service
        .create_product(new_product("Tesco Whole Milk 1L"))
        .await
        .unwrap();
    h.service
        .create_product(new_product("Hovis Bread"))
        .await
        .unwrap();
    h.service
        .set_product_mapping(tesco.id, tesco.revision, Some(milk.id))
        .await
        .unwrap();

    let listed = h
        .service
        .list_products(&crate::ports::ProductQuery {
            mapped_ingredient_id: Some(milk.id),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(listed.total, 1);
    assert_eq!(listed.items[0].name, "Tesco Whole Milk 1L");
    assert_eq!(h.products.count(), 2);
}

#[tokio::test]
async fn paginates_listings() {
    let h = harness();
    for n in 0..7 {
        h.service
            .create_ingredient(new_ingredient(&format!("Ingredient {n}")))
            .await
            .unwrap();
    }

    let page = h
        .service
        .list_ingredients(&IngredientQuery {
            page: PageRequest::new(2, 3),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(page.total, 7);
    assert_eq!(page.items.len(), 3);
    assert_eq!(page.total_pages(), 3);
}

#[tokio::test]
async fn searches_ingredients_by_name() {
    let h = harness();
    h.service
        .create_ingredient(new_ingredient("Chicken Breast"))
        .await
        .unwrap();
    h.service
        .create_ingredient(new_ingredient("Whole Milk"))
        .await
        .unwrap();

    let found = h
        .service
        .list_ingredients(&IngredientQuery {
            search: Some("chick".to_owned()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(found.total, 1);
    assert_eq!(found.items[0].ingredient.name, "Chicken Breast");
}

fn seed(key: &str, name: &str) -> crate::services::SeedIngredient {
    crate::services::SeedIngredient {
        seed_key: key.to_owned(),
        name: name.to_owned(),
        default_unit: Unit::Gram,
        shopping_section: None,
        track_stock: None,
    }
}

#[tokio::test]
async fn seeding_creates_entries_with_stable_identifiers() {
    let h = harness();
    let seeds = vec![
        seed("whole-milk", "Whole Milk"),
        seed("basmati-rice", "Basmati Rice"),
    ];

    let report = h.service.apply_seed_ingredients(&seeds).await.unwrap();
    assert_eq!(report.created, 2);
    assert_eq!(report.total(), 2);

    let milk = h
        .service
        .get_ingredient(crate::domain::IngredientId::seeded("whole-milk"))
        .await
        .unwrap();
    assert_eq!(milk.name, "Whole Milk");
    assert_eq!(milk.provenance.origin, CatalogueOrigin::Seeded);
}

#[tokio::test]
async fn reseeding_is_idempotent() {
    let h = harness();
    let seeds = vec![seed("whole-milk", "Whole Milk")];

    h.service.apply_seed_ingredients(&seeds).await.unwrap();
    let second = h.service.apply_seed_ingredients(&seeds).await.unwrap();

    assert_eq!(second.created, 0);
    assert_eq!(second.updated, 1);
    assert_eq!(h.ingredients.count(), 1);
}

#[tokio::test]
async fn reseeding_applies_upstream_corrections() {
    let h = harness();
    h.service
        .apply_seed_ingredients(&[seed("whole-milk", "Whole Milk")])
        .await
        .unwrap();

    let mut corrected = seed("whole-milk", "Whole Milk");
    corrected.name = "Whole Cow Milk".to_owned();
    corrected.default_unit = Unit::Millilitre;
    h.service
        .apply_seed_ingredients(&[corrected])
        .await
        .unwrap();

    let milk = h
        .service
        .get_ingredient(crate::domain::IngredientId::seeded("whole-milk"))
        .await
        .unwrap();
    assert_eq!(milk.name, "Whole Cow Milk");
    assert_eq!(milk.default_unit, Unit::Millilitre);
}

#[tokio::test]
async fn a_locally_edited_seed_entry_survives_reseeding() {
    let h = harness();
    let seeds = vec![seed("whole-milk", "Whole Milk")];
    h.service.apply_seed_ingredients(&seeds).await.unwrap();

    let milk = h
        .service
        .get_ingredient(crate::domain::IngredientId::seeded("whole-milk"))
        .await
        .unwrap();
    h.service
        .update_ingredient(
            milk.id,
            milk.revision,
            IngredientPatch {
                name: Some("Full Fat Milk".to_owned()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let report = h.service.apply_seed_ingredients(&seeds).await.unwrap();
    assert_eq!(report.preserved, 1);
    assert_eq!(report.updated, 0);

    let after = h.service.get_ingredient(milk.id).await.unwrap();
    assert_eq!(
        after.name, "Full Fat Milk",
        "the local correction must survive"
    );
}

#[tokio::test]
async fn a_seed_entry_colliding_with_a_local_name_is_reported_not_fatal() {
    let h = harness();
    h.service
        .create_ingredient(new_ingredient("Whole Milk"))
        .await
        .unwrap();

    let report = h
        .service
        .apply_seed_ingredients(&[seed("whole-milk", "Whole Milk")])
        .await
        .unwrap();

    assert_eq!(report.conflicted, 1);
    assert_eq!(report.created, 0);
}

#[tokio::test]
async fn seed_entries_are_validated() {
    let h = harness();
    let bad = crate::services::SeedIngredient {
        seed_key: String::new(),
        name: "Whole Milk".to_owned(),
        default_unit: Unit::Gram,
        shopping_section: None,
        track_stock: None,
    };
    assert!(h.service.apply_seed_ingredients(&[bad]).await.is_err());
}

#[tokio::test]
async fn nutrition_must_be_measured_the_same_way_as_the_pack() {
    let h = harness();
    let err = h
        .service
        .create_product(NewProduct {
            package_quantity: Some(Quantity::new(Decimal::ONE, Unit::Litre)),
            nutrition: NutritionFacts {
                basis: Some(Quantity::new(Decimal::new(100, 0), Unit::Gram)),
                energy_kcal: Some(Decimal::new(64, 0)),
                ..Default::default()
            },
            ..new_product("Tesco Whole Milk 1L")
        })
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)), "got {err:?}");
}

#[tokio::test]
async fn a_matching_dimension_is_accepted() {
    let h = harness();
    let created = h
        .service
        .create_product(NewProduct {
            package_quantity: Some(Quantity::new(Decimal::ONE, Unit::Litre)),
            nutrition: NutritionFacts {
                basis: Some(Quantity::new(Decimal::new(100, 0), Unit::Millilitre)),
                energy_kcal: Some(Decimal::new(64, 0)),
                ..Default::default()
            },
            ..new_product("Tesco Whole Milk 1L")
        })
        .await
        .unwrap();
    assert!(created.nutrition.basis.is_some());
}

#[tokio::test]
async fn a_per_serving_label_is_representable() {
    let h = harness();
    let created = h
        .service
        .create_product(NewProduct {
            package_quantity: Some(Quantity::new(Decimal::new(500, 0), Unit::Gram)),
            nutrition: NutritionFacts {
                basis: Some(Quantity::new(Decimal::new(30, 0), Unit::Gram)),
                energy_kcal: Some(Decimal::new(120, 0)),
                ..Default::default()
            },
            ..new_product("Granola 500g")
        })
        .await
        .unwrap();
    assert_eq!(
        created.nutrition.basis,
        Some(Quantity::new(Decimal::new(30, 0), Unit::Gram)),
        "a per-serving label is what packets actually print"
    );
}

#[tokio::test]
async fn counted_packs_take_a_counted_basis() {
    let h = harness();
    let created = h
        .service
        .create_product(NewProduct {
            package_quantity: Some(Quantity::new(Decimal::new(6, 0), Unit::Item)),
            nutrition: NutritionFacts {
                basis: Some(Quantity::new(Decimal::ONE, Unit::Item)),
                energy_kcal: Some(Decimal::new(72, 0)),
                ..Default::default()
            },
            ..new_product("Free Range Eggs, 6")
        })
        .await
        .unwrap();
    assert!(created.nutrition.basis.is_some());
}

#[tokio::test]
async fn servings_per_pack_needs_a_pack_size() {
    let h = harness();
    let err = h
        .service
        .create_product(NewProduct {
            servings_per_pack: Some(4),
            ..new_product("Frozen Lasagne")
        })
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)), "got {err:?}");
}

#[tokio::test]
async fn servings_per_pack_is_valid_on_a_count_pack() {
    let h = harness();
    let created = h
        .service
        .create_product(NewProduct {
            package_quantity: Some(Quantity::new(Decimal::ONE, Unit::Item)),
            servings_per_pack: Some(4),
            ..new_product("Stonebaked Pizza")
        })
        .await
        .unwrap();
    assert_eq!(created.servings_per_pack, Some(4));
}

#[tokio::test]
async fn the_invariant_is_enforced_on_update_too() {
    let h = harness();
    let created = h
        .service
        .create_product(NewProduct {
            package_quantity: Some(Quantity::new(Decimal::ONE, Unit::Litre)),
            ..new_product("Tesco Whole Milk 1L")
        })
        .await
        .unwrap();

    let err = h
        .service
        .update_product(
            created.id,
            created.revision,
            ProductPatch {
                nutrition: Some(NutritionFacts {
                    basis: Some(Quantity::new(Decimal::new(100, 0), Unit::Gram)),
                    energy_kcal: Some(Decimal::new(64, 0)),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn a_listing_reports_how_many_products_fulfil_each_ingredient() {
    let h = harness();
    let milk = h
        .service
        .create_ingredient(new_ingredient("Whole Milk"))
        .await
        .unwrap();
    h.service
        .create_ingredient(new_ingredient("Coriander"))
        .await
        .unwrap();

    for name in ["Tesco Whole Milk 1L", "Sainsbury's Whole Milk 1L"] {
        let product = h.service.create_product(new_product(name)).await.unwrap();
        h.service
            .set_product_mapping(product.id, product.revision, Some(milk.id))
            .await
            .unwrap();
    }

    let page = h
        .service
        .list_ingredients(&IngredientQuery::default())
        .await
        .unwrap();

    let listed = |name: &str| {
        page.items
            .iter()
            .find(|i| i.ingredient.name == name)
            .expect("should be listed")
            .clone()
    };

    assert_eq!(listed("Whole Milk").mapped_product_count, 2);
    assert!(listed("Whole Milk").has_nutrition_source());

    assert_eq!(listed("Coriander").mapped_product_count, 0);
    assert!(
        !listed("Coriander").has_nutrition_source(),
        "with no product there is nowhere for nutrition to come from"
    );
}

#[tokio::test]
async fn archiving_a_product_removes_it_from_the_ingredient_count() {
    let h = harness();
    let milk = h
        .service
        .create_ingredient(new_ingredient("Whole Milk"))
        .await
        .unwrap();
    let product = h
        .service
        .create_product(new_product("Tesco Whole Milk 1L"))
        .await
        .unwrap();
    let mapped = h
        .service
        .set_product_mapping(product.id, product.revision, Some(milk.id))
        .await
        .unwrap();

    assert_eq!(
        h.service
            .count_products_for_ingredient(milk.id)
            .await
            .unwrap(),
        1
    );

    h.service
        .set_product_archived(mapped.id, mapped.revision, true)
        .await
        .unwrap();

    assert_eq!(
        h.service
            .count_products_for_ingredient(milk.id)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn nutrition_for_scales_a_products_nutrition_to_the_amount() {
    let h = harness();
    let mut input = new_product("Whole Milk");
    input.nutrition = NutritionFacts {
        basis: Some(Quantity::new(Decimal::new(100, 0), Unit::Millilitre)),
        energy_kcal: Some(Decimal::new(64, 0)),
        ..Default::default()
    };
    let created = h.service.create_product(input).await.unwrap();

    let amount = crate::domain::ConsumedAmount::Measure(Quantity::new(
        Decimal::new(150, 0),
        Unit::Millilitre,
    ));
    let scaled = h.service.nutrition_for(created.id, &amount).await.unwrap();

    assert_eq!(scaled.facts.energy_kcal, Some(Decimal::new(96, 0)));
}
