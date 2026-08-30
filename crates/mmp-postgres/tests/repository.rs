#![cfg(feature = "db-tests")]

use mmp_core::CoreError;
use mmp_core::domain::{
    AccessScope, CatalogueOrigin, ConsumedAmount, ConsumptionRecord, ConsumptionRecordId,
    HouseholdMember, HouseholdMemberId, Ingredient, IngredientId, MealCategory, MealItemRef,
    MealParticipant, MealPlanComponent, MealPlanComponentId, MealPlanComponentSnapshot,
    MealPlanEntry, MealPlanEntryId, MealPlanScope, MealPlanStatus, MealSlot, MemberAccessGrant,
    NewStockEvent, NutritionFacts, NutritionGoals, NutritionQuality, NutritionTarget,
    NutritionTargetId, Product, ProductId, Provenance, Quantity, Recipe, RecipeComponent,
    RecipeComponentId, RecipeId, RecipeInstruction, RecipeInstructionId, RecipePhoto,
    RecipePhotoDerivatives, RecipeRequirement, RecipeVisibility, Revision, Role, StockEventKind,
    StockItemId, StockLevel, StorageLocation, Unit, User, UserId,
};
use mmp_core::ports::{
    AccessGrantRepository, ConsumptionQuery, ConsumptionRecordRepository,
    HouseholdMemberRepository, HouseholdSettingsRepository, IngredientQuery, IngredientRepository,
    MealPlanComponentUpdate, MealPlanQuery, MealPlanRepository, MemberQuery,
    NutritionTargetRepository, PageRequest, ProductQuery, ProductRepository, RecipeQuery,
    RecipeRepository, SnapshotOp, SortDirection, StockQuery, StockRepository, UpdateOutcome,
    UserRepository,
};
use mmp_postgres::{
    PgAccessGrantRepository, PgConsumptionRecordRepository, PgHouseholdMemberRepository,
    PgHouseholdSettingsRepository, PgIngredientRepository, PgMealPlanRepository,
    PgNutritionTargetRepository, PgProductRepository, PgRecipeRepository, PgStockRepository,
    PgUserRepository,
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::OffsetDateTime;
use time::macros::{date, time};
use uuid::Uuid;

fn ingredient(name: &str) -> Ingredient {
    let now = OffsetDateTime::now_utc();
    Ingredient {
        id: IngredientId::new(),
        name: name.to_owned(),
        default_unit: Unit::Gram,
        provenance: Provenance::local(),
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    }
}

fn product(name: &str) -> Product {
    let now = OffsetDateTime::now_utc();
    Product {
        id: ProductId::new(),
        name: name.to_owned(),
        brand: None,
        barcode: None,
        retailer: None,
        shopping_section: None,
        package_quantity: None,
        servings_per_pack: None,
        mapped_ingredient_id: None,
        nutrition: NutritionFacts::default(),
        provenance: Provenance::local(),
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    }
}

#[sqlx::test]
async fn round_trips_an_ingredient(pool: PgPool) {
    let repo = PgIngredientRepository::new(pool);
    let mut original = ingredient("Whole Milk");
    original.default_unit = Unit::Millilitre;

    repo.insert(&original).await.unwrap();
    let loaded = repo.get(original.id).await.unwrap().expect("should exist");

    assert_eq!(loaded.name, "Whole Milk");
    assert_eq!(loaded.default_unit, Unit::Millilitre);
    assert_eq!(loaded.revision, Revision::INITIAL);
}

#[sqlx::test]
async fn round_trips_product_nutrition(pool: PgPool) {
    let repo = PgProductRepository::new(pool);
    let mut original = product("Tesco Whole Milk 1L");
    original.nutrition = NutritionFacts {
        basis: Some(Quantity::new(Decimal::new(100, 0), Unit::Millilitre)),
        energy_kcal: Some(Decimal::new(64, 0)),
        fat_g: Some(Decimal::new(36, 1)),
        ..Default::default()
    };

    repo.insert(&original).await.unwrap();
    let loaded = repo.get(original.id).await.unwrap().expect("should exist");

    assert_eq!(loaded.nutrition.energy_kcal, Some(Decimal::new(64, 0)));
    assert_eq!(loaded.nutrition.fat_g, Some(Decimal::new(36, 1)));
    assert_eq!(
        loaded.nutrition.protein_g, None,
        "unknown must stay unknown"
    );
}

#[sqlx::test]
async fn an_ingredient_has_no_nutrition_columns(pool: PgPool) {
    let exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM information_schema.columns
         WHERE table_name = 'ingredient' AND column_name = 'energy_kcal')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!exists.0, "nutrition lives on products, not ingredients");
}

#[sqlx::test]
async fn every_unit_the_domain_knows_is_accepted_by_the_schema(pool: PgPool) {
    let repo = PgIngredientRepository::new(pool);
    for unit in Unit::ALL {
        let mut row = ingredient(&format!("Test {}", unit.code()));
        row.default_unit = unit;
        repo.insert(&row)
            .await
            .unwrap_or_else(|e| panic!("the CHECK constraint rejected `{}`: {e}", unit.code()));
        let loaded = repo.get(row.id).await.unwrap().unwrap();
        assert_eq!(loaded.default_unit, unit);
    }
}

#[sqlx::test]
async fn every_unit_is_accepted_as_a_nutrition_basis(pool: PgPool) {
    let repo = PgProductRepository::new(pool);
    for unit in Unit::ALL {
        let mut row = product(&format!("Basis {}", unit.code()));
        row.nutrition = NutritionFacts {
            basis: Some(Quantity::new(Decimal::new(100, 0), unit)),
            energy_kcal: Some(Decimal::ONE),
            ..Default::default()
        };
        repo.insert(&row)
            .await
            .unwrap_or_else(|e| panic!("the CHECK rejected basis unit `{}`: {e}", unit.code()));
        let loaded = repo.get(row.id).await.unwrap().unwrap();
        assert_eq!(loaded.nutrition.basis.unwrap().unit, unit);
    }
}

#[sqlx::test]
async fn a_serving_sized_basis_round_trips(pool: PgPool) {
    let repo = PgProductRepository::new(pool);
    let mut row = product("Granola 500g");
    row.package_quantity = Some(Quantity::new(Decimal::new(500, 0), Unit::Gram));
    row.nutrition = NutritionFacts {
        basis: Some(Quantity::new(Decimal::new(30, 0), Unit::Gram)),
        energy_kcal: Some(Decimal::new(120, 0)),
        ..Default::default()
    };
    repo.insert(&row).await.unwrap();

    let loaded = repo.get(row.id).await.unwrap().unwrap();
    assert_eq!(
        loaded.nutrition.basis,
        Some(Quantity::new(Decimal::new(30, 0), Unit::Gram))
    );
}

#[sqlx::test]
async fn every_origin_is_accepted_by_the_schema(pool: PgPool) {
    let repo = PgIngredientRepository::new(pool);
    for origin in CatalogueOrigin::ALL {
        let mut row = ingredient(&format!("Test {}", origin.code()));
        row.provenance = Provenance {
            origin,
            seed_key: Some(format!("test-{}", origin.code())),
            source_provider: None,
            source_external_id: None,
            locally_modified: false,
        };
        repo.insert(&row)
            .await
            .unwrap_or_else(|e| panic!("the CHECK constraint rejected `{}`: {e}", origin.code()));
    }
}

#[sqlx::test]
async fn extra_nutrients_survive_the_jsonb_round_trip(pool: PgPool) {
    let repo = PgProductRepository::new(pool);
    let mut row = product("Orange Juice 1L");
    row.nutrition = NutritionFacts {
        basis: Some(Quantity::new(Decimal::new(100, 0), Unit::Gram)),
        ..Default::default()
    };
    row.nutrition
        .extra
        .insert("vitamin_c_mg".to_owned(), Decimal::new(532, 1));
    repo.insert(&row).await.unwrap();

    let loaded = repo.get(row.id).await.unwrap().unwrap();
    assert_eq!(
        loaded.nutrition.extra.get("vitamin_c_mg"),
        Some(&Decimal::new(532, 1))
    );
}

#[sqlx::test]
async fn a_duplicate_name_is_reported_as_a_duplicate(pool: PgPool) {
    let repo = PgIngredientRepository::new(pool);
    repo.insert(&ingredient("Whole Milk")).await.unwrap();

    let err = repo.insert(&ingredient("whole milk")).await.unwrap_err();
    assert!(
        matches!(err, CoreError::Duplicate { field: "name", .. }),
        "the unique index must surface as a Duplicate, got {err:?}"
    );
}

#[sqlx::test]
async fn negative_nutrition_is_refused_by_the_database(pool: PgPool) {
    let repo = PgProductRepository::new(pool);
    let mut row = product("Impossible Product");
    row.nutrition = NutritionFacts {
        basis: Some(Quantity::new(Decimal::new(100, 0), Unit::Gram)),
        protein_g: Some(Decimal::new(-1, 0)),
        ..Default::default()
    };
    assert!(repo.insert(&row).await.is_err());
}

#[sqlx::test]
async fn updating_with_the_current_revision_succeeds(pool: PgPool) {
    let repo = PgIngredientRepository::new(pool);
    let original = ingredient("Whole Milk");
    repo.insert(&original).await.unwrap();

    let mut next = original.clone();
    next.name = "Full Fat Milk".to_owned();
    next.revision = original.revision.next();

    let outcome = repo.update(&next, original.revision).await.unwrap();
    assert_eq!(outcome, UpdateOutcome::Updated);

    let loaded = repo.get(original.id).await.unwrap().unwrap();
    assert_eq!(loaded.name, "Full Fat Milk");
    assert_eq!(loaded.revision, Revision::new(2));
}

#[sqlx::test]
async fn updating_with_a_stale_revision_reports_the_actual(pool: PgPool) {
    let repo = PgIngredientRepository::new(pool);
    let original = ingredient("Whole Milk");
    repo.insert(&original).await.unwrap();

    let mut next = original.clone();
    next.revision = original.revision.next();
    repo.update(&next, original.revision).await.unwrap();

    let mut stale = original.clone();
    stale.name = "Something Else".to_owned();
    stale.revision = Revision::new(2);
    let outcome = repo.update(&stale, Revision::INITIAL).await.unwrap();

    assert_eq!(
        outcome,
        UpdateOutcome::RevisionMismatch {
            actual: Revision::new(2)
        }
    );
}

#[sqlx::test]
async fn updating_something_that_does_not_exist_is_not_found(pool: PgPool) {
    let repo = PgIngredientRepository::new(pool);
    let outcome = repo
        .update(&ingredient("Ghost"), Revision::INITIAL)
        .await
        .unwrap();
    assert_eq!(outcome, UpdateOutcome::NotFound);
}

#[sqlx::test]
async fn listing_hides_archived_records_by_default(pool: PgPool) {
    let repo = PgIngredientRepository::new(pool);
    let mut archived = ingredient("Retired Ingredient");
    archived.archived_at = Some(OffsetDateTime::now_utc());
    repo.insert(&archived).await.unwrap();
    repo.insert(&ingredient("Whole Milk")).await.unwrap();

    let visible = repo.list(&IngredientQuery::default()).await.unwrap();
    assert_eq!(visible.total, 1);

    let all = repo
        .list(&IngredientQuery {
            include_archived: true,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(all.total, 2);
}

#[sqlx::test]
async fn listing_searches_by_name(pool: PgPool) {
    let repo = PgIngredientRepository::new(pool);
    repo.insert(&ingredient("Chicken Breast")).await.unwrap();
    repo.insert(&ingredient("Whole Milk")).await.unwrap();

    let found = repo
        .list(&IngredientQuery {
            search: Some("chick".to_owned()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(found.total, 1);
    assert_eq!(found.items[0].name, "Chicken Breast");
}

#[sqlx::test]
async fn listing_filters_by_origin(pool: PgPool) {
    let repo = PgIngredientRepository::new(pool);
    let mut seeded = ingredient("Seeded Thing");
    seeded.provenance = Provenance::seeded("seeded-thing");
    repo.insert(&seeded).await.unwrap();
    repo.insert(&ingredient("Local Thing")).await.unwrap();

    let found = repo
        .list(&IngredientQuery {
            origin: Some(CatalogueOrigin::Seeded),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(found.total, 1);
    assert_eq!(found.items[0].name, "Seeded Thing");
}

#[sqlx::test]
async fn listing_paginates(pool: PgPool) {
    let repo = PgIngredientRepository::new(pool);
    for n in 0..7 {
        repo.insert(&ingredient(&format!("Ingredient {n}")))
            .await
            .unwrap();
    }

    let page = repo
        .list(&IngredientQuery {
            page: PageRequest::new(2, 3),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(page.total, 7);
    assert_eq!(page.items.len(), 3);
    assert_eq!(page.total_pages(), 3);
}

#[sqlx::test]
async fn listing_filters_to_ingredients_without_products(pool: PgPool) {
    let ingredients = PgIngredientRepository::new(pool.clone());
    let products = PgProductRepository::new(pool);

    let milk = ingredient("Whole Milk");
    let flour = ingredient("Plain Flour");
    let coriander = ingredient("Coriander");
    ingredients.insert(&milk).await.unwrap();
    ingredients.insert(&flour).await.unwrap();
    ingredients.insert(&coriander).await.unwrap();

    let mut tesco_milk = product("Tesco Whole Milk 1L");
    tesco_milk.mapped_ingredient_id = Some(milk.id);
    products.insert(&tesco_milk).await.unwrap();

    let mut archived = product("Discontinued Flour");
    archived.mapped_ingredient_id = Some(flour.id);
    archived.archived_at = Some(OffsetDateTime::now_utc());
    products.insert(&archived).await.unwrap();

    let needs = ingredients
        .list(&IngredientQuery {
            needs_products: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
    let mut names: Vec<&str> = needs.items.iter().map(|i| i.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(needs.total, 2);
    assert_eq!(names, ["Coriander", "Plain Flour"]);

    let has = ingredients
        .list(&IngredientQuery {
            needs_products: Some(false),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(has.total, 1);
    assert_eq!(has.items[0].name, "Whole Milk");
}

#[sqlx::test]
async fn the_needs_products_filter_paginates_over_the_filtered_set(pool: PgPool) {
    let ingredients = PgIngredientRepository::new(pool.clone());
    let products = PgProductRepository::new(pool);

    for n in 0..5 {
        ingredients
            .insert(&ingredient(&format!("Bare Ingredient {n}")))
            .await
            .unwrap();
    }
    let stocked = ingredient("Stocked Ingredient");
    ingredients.insert(&stocked).await.unwrap();
    let mut p = product("A Product");
    p.mapped_ingredient_id = Some(stocked.id);
    products.insert(&p).await.unwrap();

    let page = ingredients
        .list(&IngredientQuery {
            needs_products: Some(true),
            page: PageRequest::new(2, 3),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(
        page.total, 5,
        "the count must exclude the stocked ingredient"
    );
    assert_eq!(page.items.len(), 2);
    assert_eq!(page.total_pages(), 2);
}

#[sqlx::test]
async fn finds_a_seeded_ingredient_by_its_key(pool: PgPool) {
    let repo = PgIngredientRepository::new(pool);
    let mut seeded = ingredient("Whole Milk");
    seeded.id = IngredientId::seeded("whole-milk");
    seeded.provenance = Provenance::seeded("whole-milk");
    repo.insert(&seeded).await.unwrap();

    let found = repo.find_by_seed_key("whole-milk").await.unwrap().unwrap();
    assert_eq!(found.id, IngredientId::seeded("whole-milk"));
}

#[sqlx::test]
async fn round_trips_a_product_with_a_package_size(pool: PgPool) {
    let repo = PgProductRepository::new(pool);
    let mut row = product("Tesco Whole Milk 1L");
    row.brand = Some("Tesco".to_owned());
    row.barcode = Some("5000119012345".to_owned());
    row.package_quantity = Some(Quantity::new(Decimal::ONE, Unit::Litre));

    repo.insert(&row).await.unwrap();
    let loaded = repo.get(row.id).await.unwrap().unwrap();

    assert_eq!(loaded.brand.as_deref(), Some("Tesco"));
    assert_eq!(
        loaded.package_quantity,
        Some(Quantity::new(Decimal::ONE, Unit::Litre))
    );
}

#[sqlx::test]
async fn round_trips_servings_per_pack(pool: PgPool) {
    let repo = PgProductRepository::new(pool);
    let mut row = product("Stonebaked Pizza");
    row.package_quantity = Some(Quantity::new(Decimal::ONE, Unit::Item));
    row.servings_per_pack = Some(4);

    repo.insert(&row).await.unwrap();
    let loaded = repo.get(row.id).await.unwrap().unwrap();

    assert_eq!(loaded.servings_per_pack, Some(4));
}

#[sqlx::test]
async fn a_duplicate_barcode_is_reported_as_a_duplicate(pool: PgPool) {
    let repo = PgProductRepository::new(pool);
    let mut first = product("Tesco Whole Milk 1L");
    first.barcode = Some("5000119012345".to_owned());
    repo.insert(&first).await.unwrap();

    let mut second = product("Tesco Whole Milk 2L");
    second.barcode = Some("5000119012345".to_owned());
    let err = repo.insert(&second).await.unwrap_err();
    assert!(matches!(
        err,
        CoreError::Duplicate {
            field: "barcode",
            ..
        }
    ));
}

#[sqlx::test]
async fn several_products_may_have_no_barcode(pool: PgPool) {
    let repo = PgProductRepository::new(pool);
    repo.insert(&product("Loose Apples")).await.unwrap();
    repo.insert(&product("Loose Bananas")).await.unwrap();
    let all = repo.list(&ProductQuery::default()).await.unwrap();
    assert_eq!(all.total, 2, "the barcode index must only apply where set");
}

#[sqlx::test]
async fn mapping_to_a_missing_ingredient_is_rejected(pool: PgPool) {
    let repo = PgProductRepository::new(pool);
    let mut row = product("Mystery Milk");
    row.mapped_ingredient_id = Some(IngredientId::new());

    let err = repo.insert(&row).await.unwrap_err();
    assert!(
        matches!(err, CoreError::NotFound { .. }),
        "the foreign key must surface as NotFound, got {err:?}"
    );
}

#[sqlx::test]
async fn lists_products_filtered_by_mapped_ingredient(pool: PgPool) {
    let ingredients = PgIngredientRepository::new(pool.clone());
    let products = PgProductRepository::new(pool);

    let milk = ingredient("Whole Milk");
    ingredients.insert(&milk).await.unwrap();

    let mut tesco = product("Tesco Whole Milk 1L");
    tesco.mapped_ingredient_id = Some(milk.id);
    products.insert(&tesco).await.unwrap();
    products.insert(&product("Hovis Bread")).await.unwrap();

    let found = products
        .list(&ProductQuery {
            mapped_ingredient_id: Some(milk.id),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(found.total, 1);
    assert_eq!(found.items[0].name, "Tesco Whole Milk 1L");
}

#[sqlx::test]
async fn lists_only_unmapped_products(pool: PgPool) {
    let ingredients = PgIngredientRepository::new(pool.clone());
    let products = PgProductRepository::new(pool);

    let milk = ingredient("Whole Milk");
    ingredients.insert(&milk).await.unwrap();

    let mut mapped = product("Tesco Whole Milk 1L");
    mapped.mapped_ingredient_id = Some(milk.id);
    products.insert(&mapped).await.unwrap();
    products.insert(&product("Mystery Snack")).await.unwrap();
    products.insert(&product("Unlabelled Sauce")).await.unwrap();

    let unmapped = products
        .list(&ProductQuery {
            unmapped: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
    let mut names: Vec<&str> = unmapped.items.iter().map(|p| p.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(unmapped.total, 2);
    assert_eq!(names, ["Mystery Snack", "Unlabelled Sauce"]);

    let mapped_only = products
        .list(&ProductQuery {
            unmapped: Some(false),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(mapped_only.total, 1);
    assert_eq!(mapped_only.items[0].name, "Tesco Whole Milk 1L");
}

#[sqlx::test]
async fn decimal_precision_is_preserved(pool: PgPool) {
    let repo = PgProductRepository::new(pool);
    let mut row = product("Precise Product");
    row.nutrition = NutritionFacts {
        basis: Some(Quantity::new(Decimal::new(100, 0), Unit::Gram)),
        energy_kcal: Some(Decimal::new(123456, 3)),
        ..Default::default()
    };
    repo.insert(&row).await.unwrap();

    let loaded = repo.get(row.id).await.unwrap().unwrap();
    assert_eq!(loaded.nutrition.energy_kcal, Some(Decimal::new(123456, 3)));
}

#[sqlx::test]
async fn counts_products_per_ingredient_in_one_query(pool: PgPool) {
    let ingredients = PgIngredientRepository::new(pool.clone());
    let products = PgProductRepository::new(pool);

    let milk = ingredient("Whole Milk");
    let coriander = ingredient("Coriander");
    ingredients.insert(&milk).await.unwrap();
    ingredients.insert(&coriander).await.unwrap();

    for name in ["Tesco Whole Milk 1L", "Sainsbury's Whole Milk 1L"] {
        let mut p = product(name);
        p.mapped_ingredient_id = Some(milk.id);
        products.insert(&p).await.unwrap();
    }

    let counts = products
        .count_by_ingredient(&[milk.id, coriander.id])
        .await
        .unwrap();

    assert_eq!(counts.get(&milk.id), Some(&2));
    assert_eq!(
        counts.get(&coriander.id),
        Some(&0),
        "an ingredient with no products must report zero, not be absent"
    );
}

#[sqlx::test]
async fn archived_products_do_not_count_towards_an_ingredient(pool: PgPool) {
    let ingredients = PgIngredientRepository::new(pool.clone());
    let products = PgProductRepository::new(pool);

    let milk = ingredient("Whole Milk");
    ingredients.insert(&milk).await.unwrap();

    let mut live = product("Tesco Whole Milk 1L");
    live.mapped_ingredient_id = Some(milk.id);
    products.insert(&live).await.unwrap();

    let mut gone = product("Discontinued Milk");
    gone.mapped_ingredient_id = Some(milk.id);
    gone.archived_at = Some(OffsetDateTime::now_utc());
    products.insert(&gone).await.unwrap();

    let counts = products.count_by_ingredient(&[milk.id]).await.unwrap();
    assert_eq!(counts.get(&milk.id), Some(&1));
}

#[sqlx::test]
async fn counting_nothing_is_not_an_error(pool: PgPool) {
    let products = PgProductRepository::new(pool);
    assert!(products.count_by_ingredient(&[]).await.unwrap().is_empty());
}

fn member(name: &str) -> HouseholdMember {
    let now = OffsetDateTime::now_utc();
    HouseholdMember {
        id: HouseholdMemberId::new(),
        display_name: name.to_owned(),
        linked_user_id: None,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    }
}

fn user(username: &str, roles: Vec<Role>) -> User {
    let now = OffsetDateTime::now_utc();
    User {
        id: UserId::new(),
        username: username.to_owned(),
        display_name: None,
        auth_subject: None,
        roles,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    }
}

#[sqlx::test]
async fn round_trips_a_household_member(pool: PgPool) {
    let repo = PgHouseholdMemberRepository::new(pool);
    let original = member("Joe");

    repo.insert(&original).await.unwrap();
    let loaded = repo.get(original.id).await.unwrap().expect("should exist");

    assert_eq!(loaded.display_name, "Joe");
    assert_eq!(loaded.linked_user_id, None);
    assert_eq!(loaded.revision, Revision::INITIAL);
}

#[sqlx::test]
async fn a_duplicate_member_name_is_reported_as_a_duplicate(pool: PgPool) {
    let repo = PgHouseholdMemberRepository::new(pool);
    repo.insert(&member("Joe")).await.unwrap();

    let mut clash = member("joe");
    clash.display_name = "joe".to_owned();
    let err = repo.insert(&clash).await.unwrap_err();

    assert!(matches!(err, CoreError::Duplicate { .. }), "{err:?}");
}

#[sqlx::test]
async fn a_member_revision_mismatch_is_detected(pool: PgPool) {
    let repo = PgHouseholdMemberRepository::new(pool);
    let original = member("Joe");
    repo.insert(&original).await.unwrap();

    let mut next = original.clone();
    next.display_name = "Joseph".to_owned();
    next.revision = original.revision.next();

    let outcome = repo.update(&next, Revision::new(99)).await.unwrap();
    assert!(matches!(
        outcome,
        UpdateOutcome::RevisionMismatch { actual } if actual == Revision::INITIAL
    ));
}

#[sqlx::test]
async fn round_trips_a_user_with_its_roles(pool: PgPool) {
    let repo = PgUserRepository::new(pool);
    let original = user("joe", vec![Role::Admin, Role::BasicUser]);

    repo.insert(&original).await.unwrap();
    let loaded = repo.get(original.id).await.unwrap().expect("should exist");

    assert_eq!(loaded.username, "joe");
    assert_eq!(loaded.roles, vec![Role::Admin, Role::BasicUser]);
}

#[sqlx::test]
async fn every_role_the_domain_knows_is_accepted_by_the_schema(pool: PgPool) {
    let repo = PgUserRepository::new(pool);
    let original = user("everyone", Role::ALL.to_vec());

    repo.insert(&original).await.unwrap();
    let loaded = repo.get(original.id).await.unwrap().unwrap();

    assert_eq!(loaded.roles.len(), Role::ALL.len());
}

#[sqlx::test]
async fn setting_roles_replaces_the_previous_set(pool: PgPool) {
    let repo = PgUserRepository::new(pool);
    let original = user("joe", vec![Role::Admin, Role::BasicUser]);
    repo.insert(&original).await.unwrap();

    let mut next = original.clone();
    next.roles = vec![Role::Nutritionist];
    next.revision = original.revision.next();
    repo.update(&next, original.revision).await.unwrap();

    let loaded = repo.get(original.id).await.unwrap().unwrap();
    assert_eq!(loaded.roles, vec![Role::Nutritionist]);
}

#[sqlx::test]
async fn a_duplicate_username_is_reported_as_a_duplicate(pool: PgPool) {
    let repo = PgUserRepository::new(pool);
    repo.insert(&user("joe", vec![Role::BasicUser]))
        .await
        .unwrap();

    let err = repo
        .insert(&user("JOE", vec![Role::BasicUser]))
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Duplicate { .. }), "{err:?}");
}

#[sqlx::test]
async fn users_are_counted_by_role_excluding_archived(pool: PgPool) {
    let repo = PgUserRepository::new(pool);
    let keeper = user("admin", vec![Role::Admin]);
    let spare = user("root", vec![Role::Admin]);
    repo.insert(&keeper).await.unwrap();
    repo.insert(&spare).await.unwrap();

    assert_eq!(repo.count_with_role(Role::Admin, false).await.unwrap(), 2);

    let mut archived = spare.clone();
    archived.archived_at = Some(OffsetDateTime::now_utc());
    archived.revision = spare.revision.next();
    repo.update(&archived, spare.revision).await.unwrap();

    assert_eq!(repo.count_with_role(Role::Admin, false).await.unwrap(), 1);
    assert_eq!(repo.count_with_role(Role::Admin, true).await.unwrap(), 2);
}

#[sqlx::test]
async fn one_account_cannot_be_linked_to_two_members(pool: PgPool) {
    let users = PgUserRepository::new(pool.clone());
    let members = PgHouseholdMemberRepository::new(pool);

    let account = user("joe", vec![Role::BasicUser]);
    users.insert(&account).await.unwrap();

    let mut first = member("Joe");
    first.linked_user_id = Some(account.id);
    members.insert(&first).await.unwrap();

    let mut second = member("Jo");
    second.linked_user_id = Some(account.id);
    let err = members.insert(&second).await.unwrap_err();

    assert!(matches!(err, CoreError::Duplicate { .. }), "{err:?}");
}

#[sqlx::test]
async fn archiving_an_account_leaves_its_member_linked(pool: PgPool) {
    let users = PgUserRepository::new(pool.clone());
    let members = PgHouseholdMemberRepository::new(pool);

    let account = user("joe", vec![Role::BasicUser]);
    users.insert(&account).await.unwrap();

    let mut linked = member("Joe");
    linked.linked_user_id = Some(account.id);
    members.insert(&linked).await.unwrap();

    let mut archived = account.clone();
    archived.archived_at = Some(OffsetDateTime::now_utc());
    archived.revision = account.revision.next();
    users.update(&archived, account.revision).await.unwrap();

    let loaded = members.get(linked.id).await.unwrap().unwrap();
    assert_eq!(loaded.linked_user_id, Some(account.id));
    assert_eq!(loaded.revision, Revision::INITIAL);
}

#[sqlx::test]
async fn a_member_can_be_found_by_its_linked_account(pool: PgPool) {
    let users = PgUserRepository::new(pool.clone());
    let members = PgHouseholdMemberRepository::new(pool);

    let account = user("joe", vec![Role::BasicUser]);
    users.insert(&account).await.unwrap();
    let mut linked = member("Joe");
    linked.linked_user_id = Some(account.id);
    members.insert(&linked).await.unwrap();

    let found = members
        .find_by_linked_user(account.id)
        .await
        .unwrap()
        .expect("should exist");
    assert_eq!(found.id, linked.id);
}

#[sqlx::test]
async fn members_can_be_filtered_by_whether_they_have_an_account(pool: PgPool) {
    let users = PgUserRepository::new(pool.clone());
    let members = PgHouseholdMemberRepository::new(pool);

    let account = user("joe", vec![Role::BasicUser]);
    users.insert(&account).await.unwrap();
    let mut linked = member("Joe");
    linked.linked_user_id = Some(account.id);
    members.insert(&linked).await.unwrap();
    members.insert(&member("Ann")).await.unwrap();

    let with = members
        .list(&MemberQuery {
            with_account: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(with.total, 1);
    assert_eq!(with.items[0].display_name, "Joe");

    let without = members
        .list(&MemberQuery {
            with_account: Some(false),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(without.total, 1);
    assert_eq!(without.items[0].display_name, "Ann");
}

#[sqlx::test]
async fn access_grants_round_trip(pool: PgPool) {
    let users = PgUserRepository::new(pool.clone());
    let members = PgHouseholdMemberRepository::new(pool.clone());
    let grants = PgAccessGrantRepository::new(pool);

    let account = user("nutritionist", vec![Role::Nutritionist]);
    users.insert(&account).await.unwrap();
    let subject = member("Joe");
    members.insert(&subject).await.unwrap();

    assert!(
        !grants
            .exists(account.id, subject.id, AccessScope::HealthData)
            .await
            .unwrap()
    );

    grants
        .upsert(&MemberAccessGrant {
            grantee_user_id: account.id,
            subject_member_id: subject.id,
            scope: AccessScope::HealthData,
            granted_at: OffsetDateTime::now_utc(),
            granted_by: None,
        })
        .await
        .unwrap();

    assert!(
        grants
            .exists(account.id, subject.id, AccessScope::HealthData)
            .await
            .unwrap()
    );
    assert_eq!(grants.list_for_member(subject.id).await.unwrap().len(), 1);

    assert!(
        grants
            .revoke(account.id, subject.id, AccessScope::HealthData)
            .await
            .unwrap()
    );
    assert!(grants.list_for_member(subject.id).await.unwrap().is_empty());
}

#[sqlx::test]
async fn granting_the_same_scope_twice_does_not_duplicate(pool: PgPool) {
    let users = PgUserRepository::new(pool.clone());
    let members = PgHouseholdMemberRepository::new(pool.clone());
    let grants = PgAccessGrantRepository::new(pool);

    let account = user("viewer", vec![Role::BasicUser]);
    users.insert(&account).await.unwrap();
    let subject = member("Joe");
    members.insert(&subject).await.unwrap();

    for _ in 0..2 {
        grants
            .upsert(&MemberAccessGrant {
                grantee_user_id: account.id,
                subject_member_id: subject.id,
                scope: AccessScope::HealthData,
                granted_at: OffsetDateTime::now_utc(),
                granted_by: Some(account.id),
            })
            .await
            .unwrap();
    }

    assert_eq!(grants.list_for_member(subject.id).await.unwrap().len(), 1);
}

#[sqlx::test]
async fn catalogue_records_carry_optional_audit_columns(pool: PgPool) {
    let users = PgUserRepository::new(pool.clone());
    let account = user("joe", vec![Role::BasicUser]);
    users.insert(&account).await.unwrap();

    let repo = PgIngredientRepository::new(pool.clone());
    let created = ingredient("Coriander");
    repo.insert(&created).await.unwrap();

    sqlx::query("UPDATE ingredient SET created_by = $2 WHERE id = $1")
        .bind(created.id.as_uuid())
        .bind(account.id.as_uuid())
        .execute(&pool)
        .await
        .expect("the audit column should accept a user id");

    let stored: (Option<uuid::Uuid>,) =
        sqlx::query_as("SELECT created_by FROM ingredient WHERE id = $1")
            .bind(created.id.as_uuid())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored.0, Some(account.id.as_uuid()));
}

async fn seed_member_and_product(pool: &PgPool) -> (HouseholdMemberId, ProductId) {
    let members = PgHouseholdMemberRepository::new(pool.clone());
    let seeded_member = member("Joe");
    members.insert(&seeded_member).await.unwrap();

    let products = PgProductRepository::new(pool.clone());
    let mut seeded_product = product("Whole Milk");
    seeded_product.package_quantity = Some(Quantity::new(Decimal::new(1000, 0), Unit::Millilitre));
    seeded_product.nutrition = NutritionFacts {
        basis: Some(Quantity::new(Decimal::new(100, 0), Unit::Millilitre)),
        energy_kcal: Some(Decimal::new(64, 0)),
        ..Default::default()
    };
    products.insert(&seeded_product).await.unwrap();

    (seeded_member.id, seeded_product.id)
}

fn consumption_record(member_id: HouseholdMemberId, product_id: ProductId) -> ConsumptionRecord {
    let now = OffsetDateTime::now_utc();
    ConsumptionRecord {
        id: ConsumptionRecordId::new(),
        member_id,
        item: MealItemRef::product(product_id),
        recorded_by: None,
        meal_plan_entry_id: None,
        meal_plan_component_id: None,
        slot: MealSlot::Breakfast,
        amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(150, 0), Unit::Millilitre)),
        consumed_on: date!(2026 - 08 - 22),
        consumed_at: Some(now),
        nutrition: NutritionFacts {
            basis: Some(Quantity::new(Decimal::new(150, 0), Unit::Millilitre)),
            energy_kcal: Some(Decimal::new(96, 0)),
            ..Default::default()
        },
        quality: NutritionQuality::Partial,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
    }
}

#[sqlx::test]
async fn round_trips_a_measured_consumption_record(pool: PgPool) {
    let (member_id, product_id) = seed_member_and_product(&pool).await;
    let repo = PgConsumptionRecordRepository::new(pool);
    let original = consumption_record(member_id, product_id);

    repo.insert(&original).await.unwrap();
    let loaded = repo.get(original.id).await.unwrap().expect("should exist");

    assert_eq!(loaded.amount, original.amount);
    assert_eq!(loaded.slot, MealSlot::Breakfast);
    assert_eq!(loaded.nutrition.energy_kcal, Some(Decimal::new(96, 0)));
    assert_eq!(loaded.quality, NutritionQuality::Partial);
    assert_eq!(loaded.consumed_on, date!(2026 - 08 - 22));
    assert_eq!(loaded.revision, Revision::INITIAL);
}

#[sqlx::test]
async fn round_trips_a_consumption_record_with_an_unknown_time(pool: PgPool) {
    let (member_id, product_id) = seed_member_and_product(&pool).await;
    let repo = PgConsumptionRecordRepository::new(pool);
    let mut original = consumption_record(member_id, product_id);
    original.consumed_at = None;

    repo.insert(&original).await.unwrap();
    let loaded = repo.get(original.id).await.unwrap().unwrap();

    assert_eq!(loaded.consumed_at, None);
}

#[sqlx::test]
async fn round_trips_a_servings_amount_with_no_unit(pool: PgPool) {
    let (member_id, product_id) = seed_member_and_product(&pool).await;
    let repo = PgConsumptionRecordRepository::new(pool);
    let mut original = consumption_record(member_id, product_id);
    original.amount = ConsumedAmount::Servings(Decimal::new(15, 1));

    repo.insert(&original).await.unwrap();
    let loaded = repo.get(original.id).await.unwrap().unwrap();

    assert_eq!(loaded.amount, ConsumedAmount::Servings(Decimal::new(15, 1)));
}

#[sqlx::test]
async fn round_trips_a_packs_amount(pool: PgPool) {
    let (member_id, product_id) = seed_member_and_product(&pool).await;
    let repo = PgConsumptionRecordRepository::new(pool);
    let mut original = consumption_record(member_id, product_id);
    original.amount = ConsumedAmount::Packs(Decimal::new(5, 1));

    repo.insert(&original).await.unwrap();
    let loaded = repo.get(original.id).await.unwrap().unwrap();

    assert_eq!(loaded.amount, ConsumedAmount::Packs(Decimal::new(5, 1)));
}

#[sqlx::test]
async fn every_unit_the_domain_knows_is_accepted_as_a_measured_amount(pool: PgPool) {
    let (member_id, product_id) = seed_member_and_product(&pool).await;
    let repo = PgConsumptionRecordRepository::new(pool);
    for unit in Unit::ALL {
        let mut row = consumption_record(member_id, product_id);
        row.amount = ConsumedAmount::Measure(Quantity::new(Decimal::ONE, unit));
        repo.insert(&row)
            .await
            .unwrap_or_else(|e| panic!("the CHECK rejected amount unit `{}`: {e}", unit.code()));
        let loaded = repo.get(row.id).await.unwrap().unwrap();
        assert_eq!(
            loaded.amount,
            ConsumedAmount::Measure(Quantity::new(Decimal::ONE, unit))
        );
    }
}

#[sqlx::test]
async fn every_nutrition_quality_round_trips(pool: PgPool) {
    let (member_id, product_id) = seed_member_and_product(&pool).await;
    let repo = PgConsumptionRecordRepository::new(pool);
    for quality in NutritionQuality::ALL {
        let mut row = consumption_record(member_id, product_id);
        row.quality = quality;
        repo.insert(&row).await.unwrap();
        let loaded = repo.get(row.id).await.unwrap().unwrap();
        assert_eq!(loaded.quality, quality);
    }
}

#[sqlx::test]
async fn updating_a_consumption_record_with_a_stale_revision_reports_the_actual(pool: PgPool) {
    let (member_id, product_id) = seed_member_and_product(&pool).await;
    let repo = PgConsumptionRecordRepository::new(pool);
    let original = consumption_record(member_id, product_id);
    repo.insert(&original).await.unwrap();

    let mut next = original.clone();
    next.revision = original.revision.next();
    repo.update(&next, original.revision).await.unwrap();

    let mut stale = original.clone();
    stale.consumed_on = date!(2026 - 08 - 23);
    stale.revision = Revision::new(2);
    let outcome = repo.update(&stale, Revision::INITIAL).await.unwrap();

    assert_eq!(
        outcome,
        UpdateOutcome::RevisionMismatch {
            actual: Revision::new(2)
        }
    );
}

#[sqlx::test]
async fn updating_a_missing_consumption_record_is_not_found(pool: PgPool) {
    let (member_id, product_id) = seed_member_and_product(&pool).await;
    let repo = PgConsumptionRecordRepository::new(pool);
    let outcome = repo
        .update(
            &consumption_record(member_id, product_id),
            Revision::INITIAL,
        )
        .await
        .unwrap();
    assert_eq!(outcome, UpdateOutcome::NotFound);
}

#[sqlx::test]
async fn deleting_a_consumption_record_removes_it(pool: PgPool) {
    let (member_id, product_id) = seed_member_and_product(&pool).await;
    let repo = PgConsumptionRecordRepository::new(pool);
    let original = consumption_record(member_id, product_id);
    repo.insert(&original).await.unwrap();

    assert!(repo.delete(original.id).await.unwrap());
    assert!(repo.get(original.id).await.unwrap().is_none());
}

#[sqlx::test]
async fn deleting_a_missing_consumption_record_reports_false(pool: PgPool) {
    let repo = PgConsumptionRecordRepository::new(pool);
    assert!(!repo.delete(ConsumptionRecordId::new()).await.unwrap());
}

#[sqlx::test]
async fn listing_filters_by_member_and_date_range(pool: PgPool) {
    let (member_id, product_id) = seed_member_and_product(&pool).await;
    let other_members = PgHouseholdMemberRepository::new(pool.clone());
    let other_member = member("Ann");
    other_members.insert(&other_member).await.unwrap();

    let repo = PgConsumptionRecordRepository::new(pool);

    let mut on_day = consumption_record(member_id, product_id);
    on_day.consumed_on = date!(2026 - 08 - 22);
    repo.insert(&on_day).await.unwrap();

    let mut other_day = consumption_record(member_id, product_id);
    other_day.consumed_on = date!(2026 - 08 - 21);
    repo.insert(&other_day).await.unwrap();

    let mut other_member_entry = consumption_record(other_member.id, product_id);
    other_member_entry.consumed_on = date!(2026 - 08 - 22);
    repo.insert(&other_member_entry).await.unwrap();

    let query = ConsumptionQuery {
        member_id: Some(member_id),
        from: Some(date!(2026 - 08 - 22)),
        to: Some(date!(2026 - 08 - 22)),
        page: PageRequest::default(),
        sort: Default::default(),
    };
    let page = repo.list(&query).await.unwrap();

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, on_day.id);
}

async fn seed_meal_plan_dependencies(pool: &PgPool) -> (HouseholdMemberId, ProductId, UserId) {
    let users = PgUserRepository::new(pool.clone());
    let actor = user("planner", vec![Role::Admin]);
    users.insert(&actor).await.unwrap();
    let (member_id, product_id) = seed_member_and_product(pool).await;
    (member_id, product_id, actor.id)
}

fn meal_plan_entry(
    member_id: HouseholdMemberId,
    product_id: ProductId,
    actor_id: UserId,
) -> MealPlanEntry {
    let now = OffsetDateTime::now_utc();
    let now = now
        .replace_nanosecond(now.nanosecond() / 1_000 * 1_000)
        .unwrap();
    MealPlanEntry {
        id: MealPlanEntryId::new(),
        scope: MealPlanScope::Member,
        member_id: Some(member_id),
        planned_on: date!(2026 - 08 - 25),
        planned_time: Some(time::macros::time!(18:30)),
        slot: MealSlot::Dinner,
        status: MealPlanStatus::Planned,
        components: vec![MealPlanComponent {
            id: MealPlanComponentId::new(),
            item: MealItemRef::product(product_id),
            amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(150, 0), Unit::Millilitre)),
            position: 0,
            snapshot: None,
            status: MealPlanStatus::Planned,
            resolved_by: None,
            resolved_at: None,
            revision: Revision::INITIAL,
            display_order: Uuid::now_v7(),
        }],
        participants: Vec::<MealParticipant>::new(),
        created_by: actor_id,
        updated_by: actor_id,
        resolved_by: None,
        resolved_at: None,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
    }
}

#[sqlx::test]
async fn round_trips_a_planned_meal_with_components(pool: PgPool) {
    let (member_id, product_id, actor_id) = seed_meal_plan_dependencies(&pool).await;
    let repo = PgMealPlanRepository::new(pool);
    let original = meal_plan_entry(member_id, product_id, actor_id);

    repo.insert(&original).await.unwrap();
    let loaded = repo.get(original.id).await.unwrap().unwrap();

    assert_eq!(loaded, original);
    let listed = repo
        .list(&MealPlanQuery {
            member_id,
            from: date!(2026 - 08 - 24),
            to: date!(2026 - 08 - 30),
            include_participating: false,
        })
        .await
        .unwrap();
    assert_eq!(listed, vec![original]);
}

#[sqlx::test]
async fn resolving_a_meal_freezes_components_and_links_consumption(pool: PgPool) {
    let (member_id, product_id, actor_id) = seed_meal_plan_dependencies(&pool).await;
    let plans = PgMealPlanRepository::new(pool.clone());
    let consumption = PgConsumptionRecordRepository::new(pool);
    let original = meal_plan_entry(member_id, product_id, actor_id);
    plans.insert(&original).await.unwrap();

    let mut resolved = original.clone();
    resolved.status = MealPlanStatus::Eaten;
    resolved.resolved_by = Some(actor_id);
    resolved.resolved_at = Some(resolved.updated_at);
    resolved.revision = resolved.revision.next();
    resolved.components[0].status = MealPlanStatus::Eaten;
    resolved.components[0].resolved_by = Some(actor_id);
    resolved.components[0].resolved_at = Some(resolved.updated_at);
    resolved.components[0].revision = resolved.components[0].revision.next();
    resolved.components[0].snapshot = Some(MealPlanComponentSnapshot {
        item_name: "Whole Milk".to_owned(),
        nutrition: NutritionFacts {
            basis: Some(Quantity::new(Decimal::new(150, 0), Unit::Millilitre)),
            energy_kcal: Some(Decimal::new(96, 0)),
            ..Default::default()
        },
        quality: NutritionQuality::Partial,
    });
    let mut record = consumption_record(member_id, product_id);
    record.meal_plan_entry_id = Some(original.id);
    record.meal_plan_component_id = Some(original.components[0].id);
    record.recorded_by = Some(actor_id);

    let outcome = plans
        .resolve(&resolved, original.revision, &[record.clone()])
        .await
        .unwrap();
    assert_eq!(outcome, UpdateOutcome::Updated);
    assert_eq!(plans.get(original.id).await.unwrap().unwrap(), resolved);

    let loaded_record = consumption.get(record.id).await.unwrap().unwrap();
    assert_eq!(loaded_record.meal_plan_entry_id, Some(original.id));
    assert_eq!(
        loaded_record.meal_plan_component_id,
        Some(original.components[0].id)
    );
}

#[sqlx::test]
async fn resolving_and_reopening_one_component_preserves_its_sibling(pool: PgPool) {
    let (member_id, product_id, actor_id) = seed_meal_plan_dependencies(&pool).await;
    let plans = PgMealPlanRepository::new(pool.clone());
    let consumption = PgConsumptionRecordRepository::new(pool);
    let mut original = meal_plan_entry(member_id, product_id, actor_id);
    let mut sibling = original.components[0].clone();
    sibling.id = MealPlanComponentId::new();
    sibling.position = 1;
    original.components.push(sibling.clone());
    plans.insert(&original).await.unwrap();

    let component_id = original.components[0].id;
    let snapshot = MealPlanComponentSnapshot {
        item_name: "Whole Milk".to_owned(),
        nutrition: NutritionFacts::default(),
        quality: NutritionQuality::Unknown,
    };
    let now = original.updated_at;
    let eaten_update = MealPlanComponentUpdate {
        id: component_id,
        status: MealPlanStatus::Eaten,
        snapshot: SnapshotOp::Set(&snapshot),
        resolved_by: Some(actor_id),
        resolved_at: Some(now),
        revision: original.components[0].revision.next(),
        entry_status: MealPlanStatus::PartiallyResolved,
        entry_resolved_by: Some(actor_id),
        entry_resolved_at: Some(now),
        actor_id,
        now,
    };
    let mut record = consumption_record(member_id, product_id);
    record.meal_plan_entry_id = Some(original.id);
    record.meal_plan_component_id = Some(component_id);

    let outcome = plans
        .resolve_component(
            original.id,
            &eaten_update,
            &[],
            original.components[0].revision,
            Some(&record),
        )
        .await
        .unwrap();
    assert_eq!(outcome, UpdateOutcome::Updated);

    let partially_resolved = plans.get(original.id).await.unwrap().unwrap();
    assert_eq!(partially_resolved.status, MealPlanStatus::PartiallyResolved);
    assert_eq!(
        partially_resolved.components[0].status,
        MealPlanStatus::Eaten
    );
    assert!(partially_resolved.components[0].snapshot.is_some());
    assert_eq!(partially_resolved.components[1], sibling);
    assert!(consumption.get(record.id).await.unwrap().is_some());

    let reopen_update = MealPlanComponentUpdate {
        id: component_id,
        status: MealPlanStatus::Planned,
        snapshot: SnapshotOp::Clear,
        resolved_by: None,
        resolved_at: None,
        revision: partially_resolved.components[0].revision.next(),
        entry_status: MealPlanStatus::Planned,
        entry_resolved_by: None,
        entry_resolved_at: None,
        actor_id,
        now,
    };
    let outcome = plans
        .reopen_component(
            original.id,
            &reopen_update,
            &[],
            partially_resolved.components[0].revision,
        )
        .await
        .unwrap();
    assert_eq!(outcome, UpdateOutcome::Updated);

    let reopened = plans.get(original.id).await.unwrap().unwrap();
    assert_eq!(reopened.status, MealPlanStatus::Planned);
    assert_eq!(reopened.components[0].status, MealPlanStatus::Planned);
    assert_eq!(reopened.components[0].snapshot, None);
    assert_eq!(reopened.components[1], sibling);
    assert!(consumption.get(record.id).await.unwrap().is_some());
}

#[sqlx::test]
async fn updating_a_meal_plan_entry_replaces_its_components(pool: PgPool) {
    let (member_id, product_id, actor_id) = seed_meal_plan_dependencies(&pool).await;
    let other_product = product("Other product");
    PgProductRepository::new(pool.clone())
        .insert(&other_product)
        .await
        .unwrap();
    let repo = PgMealPlanRepository::new(pool);
    let original = meal_plan_entry(member_id, product_id, actor_id);
    repo.insert(&original).await.unwrap();

    let mut updated = original.clone();
    updated.components = vec![
        MealPlanComponent {
            id: MealPlanComponentId::new(),
            item: MealItemRef::product(other_product.id),
            amount: ConsumedAmount::Servings(Decimal::new(2, 0)),
            position: 0,
            snapshot: None,
            status: MealPlanStatus::Planned,
            resolved_by: None,
            resolved_at: None,
            revision: Revision::INITIAL,
            display_order: Uuid::now_v7(),
        },
        MealPlanComponent {
            id: MealPlanComponentId::new(),
            item: MealItemRef::product(product_id),
            amount: ConsumedAmount::Servings(Decimal::new(1, 0)),
            position: 1,
            snapshot: None,
            status: MealPlanStatus::Planned,
            resolved_by: None,
            resolved_at: None,
            revision: Revision::INITIAL,
            display_order: Uuid::now_v7(),
        },
    ];
    updated.revision = updated.revision.next();

    let outcome = repo.update(&updated, original.revision).await.unwrap();
    assert_eq!(outcome, UpdateOutcome::Updated);

    let loaded = repo.get(original.id).await.unwrap().unwrap();
    assert_eq!(loaded.components.len(), 2);
    assert!(
        loaded
            .components
            .iter()
            .all(|component| component.id != original.components[0].id)
    );
    assert_eq!(loaded.components[0].position, 0);
    assert_eq!(loaded.components[1].position, 1);
}

#[sqlx::test]
async fn updating_a_meal_plan_entry_with_a_stale_revision_is_refused(pool: PgPool) {
    let (member_id, product_id, actor_id) = seed_meal_plan_dependencies(&pool).await;
    let repo = PgMealPlanRepository::new(pool);
    let original = meal_plan_entry(member_id, product_id, actor_id);
    repo.insert(&original).await.unwrap();

    let mut stale = original.clone();
    stale.revision = stale.revision.next();

    let outcome = repo.update(&stale, Revision::INITIAL.next()).await.unwrap();
    assert_eq!(
        outcome,
        UpdateOutcome::RevisionMismatch {
            actual: original.revision
        }
    );
    assert_eq!(repo.get(original.id).await.unwrap().unwrap(), original);
}

#[sqlx::test]
async fn resolving_with_a_stale_revision_leaves_the_entry_untouched(pool: PgPool) {
    let (member_id, product_id, actor_id) = seed_meal_plan_dependencies(&pool).await;
    let plans = PgMealPlanRepository::new(pool.clone());
    let consumption = PgConsumptionRecordRepository::new(pool);
    let original = meal_plan_entry(member_id, product_id, actor_id);
    plans.insert(&original).await.unwrap();

    let mut resolved = original.clone();
    resolved.status = MealPlanStatus::Eaten;
    resolved.resolved_by = Some(actor_id);
    resolved.resolved_at = Some(resolved.updated_at);
    resolved.revision = resolved.revision.next();
    let mut record = consumption_record(member_id, product_id);
    record.meal_plan_entry_id = Some(original.id);
    record.meal_plan_component_id = Some(original.components[0].id);

    let outcome = plans
        .resolve(&resolved, Revision::INITIAL.next(), &[record.clone()])
        .await
        .unwrap();
    assert_eq!(
        outcome,
        UpdateOutcome::RevisionMismatch {
            actual: original.revision
        }
    );
    assert_eq!(plans.get(original.id).await.unwrap().unwrap(), original);
    assert!(consumption.get(record.id).await.unwrap().is_none());
}

#[sqlx::test]
async fn deleting_a_meal_plan_entry_cascades_to_its_components(pool: PgPool) {
    let (member_id, product_id, actor_id) = seed_meal_plan_dependencies(&pool).await;
    let repo = PgMealPlanRepository::new(pool.clone());
    let original = meal_plan_entry(member_id, product_id, actor_id);
    repo.insert(&original).await.unwrap();

    let outcome = repo.delete(original.id, original.revision).await.unwrap();
    assert_eq!(outcome, UpdateOutcome::Updated);
    assert!(repo.get(original.id).await.unwrap().is_none());

    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM meal_plan_component")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, 0);
}

#[sqlx::test]
async fn deleting_an_unknown_meal_plan_entry_reports_not_found(pool: PgPool) {
    let repo = PgMealPlanRepository::new(pool);
    let outcome = repo
        .delete(MealPlanEntryId::new(), Revision::INITIAL)
        .await
        .unwrap();
    assert_eq!(outcome, UpdateOutcome::NotFound);
}

#[sqlx::test]
async fn reopening_a_meal_clears_the_snapshot(pool: PgPool) {
    let (member_id, product_id, actor_id) = seed_meal_plan_dependencies(&pool).await;
    let plans = PgMealPlanRepository::new(pool.clone());
    let consumption = PgConsumptionRecordRepository::new(pool);
    let original = meal_plan_entry(member_id, product_id, actor_id);
    plans.insert(&original).await.unwrap();

    let mut resolved = original.clone();
    resolved.status = MealPlanStatus::Eaten;
    resolved.resolved_by = Some(actor_id);
    resolved.resolved_at = Some(resolved.updated_at);
    resolved.revision = resolved.revision.next();
    resolved.components[0].status = MealPlanStatus::Eaten;
    resolved.components[0].resolved_by = Some(actor_id);
    resolved.components[0].resolved_at = Some(resolved.updated_at);
    resolved.components[0].revision = resolved.components[0].revision.next();
    resolved.components[0].snapshot = Some(MealPlanComponentSnapshot {
        item_name: "Whole Milk".to_owned(),
        nutrition: NutritionFacts {
            basis: Some(Quantity::new(Decimal::new(150, 0), Unit::Millilitre)),
            energy_kcal: Some(Decimal::new(96, 0)),
            ..Default::default()
        },
        quality: NutritionQuality::Partial,
    });
    let mut record = consumption_record(member_id, product_id);
    record.meal_plan_entry_id = Some(original.id);
    record.meal_plan_component_id = Some(original.components[0].id);
    plans
        .resolve(&resolved, original.revision, &[record.clone()])
        .await
        .unwrap();

    let mut reopened = resolved.clone();
    reopened.status = MealPlanStatus::Planned;
    reopened.resolved_by = None;
    reopened.resolved_at = None;
    reopened.components[0].snapshot = None;
    reopened.components[0].status = MealPlanStatus::Planned;
    reopened.components[0].resolved_by = None;
    reopened.components[0].resolved_at = None;
    reopened.components[0].revision = reopened.components[0].revision.next();
    reopened.revision = reopened.revision.next();

    let outcome = plans.reopen(&reopened, resolved.revision).await.unwrap();
    assert_eq!(outcome, UpdateOutcome::Updated);
    assert_eq!(plans.get(original.id).await.unwrap().unwrap(), reopened);
    assert!(consumption.get(record.id).await.unwrap().is_some());
}

#[sqlx::test]
async fn a_component_cannot_be_confirmed_by_two_consumption_records(pool: PgPool) {
    let (member_id, product_id, actor_id) = seed_meal_plan_dependencies(&pool).await;
    let plans = PgMealPlanRepository::new(pool.clone());
    let original = meal_plan_entry(member_id, product_id, actor_id);
    plans.insert(&original).await.unwrap();

    let mut resolved = original.clone();
    resolved.status = MealPlanStatus::Eaten;
    resolved.resolved_by = Some(actor_id);
    resolved.resolved_at = Some(resolved.updated_at);
    resolved.revision = resolved.revision.next();
    resolved.components[0].status = MealPlanStatus::Eaten;
    resolved.components[0].resolved_by = Some(actor_id);
    resolved.components[0].resolved_at = Some(resolved.updated_at);
    resolved.components[0].revision = resolved.components[0].revision.next();
    resolved.components[0].snapshot = Some(MealPlanComponentSnapshot {
        item_name: "Whole Milk".to_owned(),
        nutrition: NutritionFacts::default(),
        quality: NutritionQuality::Unknown,
    });
    let mut first = consumption_record(member_id, product_id);
    first.meal_plan_entry_id = Some(original.id);
    first.meal_plan_component_id = Some(original.components[0].id);
    let mut second = consumption_record(member_id, product_id);
    second.meal_plan_entry_id = Some(original.id);
    second.meal_plan_component_id = Some(original.components[0].id);

    let error = plans
        .resolve(&resolved, original.revision, &[first, second])
        .await
        .unwrap_err();
    assert!(matches!(error, CoreError::Duplicate { .. }), "{error:?}");
}

fn target(
    member_id: HouseholdMemberId,
    effective: time::Date,
    goals: NutritionGoals,
) -> NutritionTarget {
    let now = OffsetDateTime::now_utc();
    NutritionTarget {
        id: NutritionTargetId::new(),
        member_id,
        effective_from: effective,
        goals,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
    }
}

#[sqlx::test]
async fn round_trips_and_orders_nutrition_targets(pool: PgPool) {
    let members = PgHouseholdMemberRepository::new(pool.clone());
    let repo = PgNutritionTargetRepository::new(pool);
    let joe = member("Joe");
    members.insert(&joe).await.unwrap();

    let later = target(
        joe.id,
        date!(2026 - 06 - 01),
        NutritionGoals {
            energy_kcal: Some(Decimal::new(1800, 0)),
            protein_g: Some(Decimal::new(120, 0)),
            ..Default::default()
        },
    );
    let earlier = target(
        joe.id,
        date!(2026 - 01 - 01),
        NutritionGoals {
            energy_kcal: Some(Decimal::new(2200, 0)),
            ..Default::default()
        },
    );
    repo.insert(&later).await.unwrap();
    repo.insert(&earlier).await.unwrap();

    let listed = repo.list_for_member(joe.id).await.unwrap();
    let dates: Vec<_> = listed.iter().map(|t| t.effective_from).collect();
    assert_eq!(dates, vec![date!(2026 - 01 - 01), date!(2026 - 06 - 01)]);

    let loaded = repo.get(later.id).await.unwrap().expect("should exist");
    assert_eq!(loaded.goals.energy_kcal, Some(Decimal::new(1800, 0)));
    assert_eq!(loaded.goals.protein_g, Some(Decimal::new(120, 0)));
    assert_eq!(loaded.goals.fat_g, None, "unknown must stay unknown");
}

#[sqlx::test]
async fn a_duplicate_effective_date_is_a_conflict(pool: PgPool) {
    let members = PgHouseholdMemberRepository::new(pool.clone());
    let repo = PgNutritionTargetRepository::new(pool);
    let joe = member("Joe");
    members.insert(&joe).await.unwrap();

    let first = target(
        joe.id,
        date!(2026 - 08 - 25),
        NutritionGoals {
            energy_kcal: Some(Decimal::new(2000, 0)),
            ..Default::default()
        },
    );
    repo.insert(&first).await.unwrap();
    let clash = target(
        joe.id,
        date!(2026 - 08 - 25),
        NutritionGoals {
            energy_kcal: Some(Decimal::new(1800, 0)),
            ..Default::default()
        },
    );
    let error = repo.insert(&clash).await.unwrap_err();
    assert!(matches!(error, CoreError::Duplicate { .. }));
}

#[sqlx::test]
async fn an_empty_target_is_rejected_by_the_schema(pool: PgPool) {
    let members = PgHouseholdMemberRepository::new(pool.clone());
    let repo = PgNutritionTargetRepository::new(pool);
    let joe = member("Joe");
    members.insert(&joe).await.unwrap();

    let empty = target(joe.id, date!(2026 - 08 - 25), NutritionGoals::default());
    let error = repo.insert(&empty).await.unwrap_err();
    assert!(matches!(error, CoreError::Validation(_)));
}

#[sqlx::test]
async fn a_negative_goal_is_rejected_by_the_schema(pool: PgPool) {
    let members = PgHouseholdMemberRepository::new(pool.clone());
    let repo = PgNutritionTargetRepository::new(pool);
    let joe = member("Joe");
    members.insert(&joe).await.unwrap();

    let negative = target(
        joe.id,
        date!(2026 - 08 - 25),
        NutritionGoals {
            protein_g: Some(Decimal::new(-1, 0)),
            ..Default::default()
        },
    );
    let error = repo.insert(&negative).await.unwrap_err();
    assert!(matches!(error, CoreError::Validation(_)));
}

#[sqlx::test]
async fn nutrition_target_updates_report_revision_outcomes(pool: PgPool) {
    let members = PgHouseholdMemberRepository::new(pool.clone());
    let repo = PgNutritionTargetRepository::new(pool);
    let joe = member("Joe");
    members.insert(&joe).await.unwrap();

    let original = target(
        joe.id,
        date!(2026 - 08 - 25),
        NutritionGoals {
            energy_kcal: Some(Decimal::new(2000, 0)),
            ..Default::default()
        },
    );
    repo.insert(&original).await.unwrap();

    let mut updated = original.clone();
    updated.goals.energy_kcal = Some(Decimal::new(1900, 0));
    updated.revision = original.revision.next();
    assert_eq!(
        repo.update(&updated, original.revision).await.unwrap(),
        UpdateOutcome::Updated
    );

    let stale = repo.update(&updated, original.revision).await.unwrap();
    assert_eq!(
        stale,
        UpdateOutcome::RevisionMismatch {
            actual: updated.revision,
        }
    );

    let missing = target(
        joe.id,
        date!(2026 - 09 - 01),
        NutritionGoals {
            energy_kcal: Some(Decimal::new(2000, 0)),
            ..Default::default()
        },
    );
    assert_eq!(
        repo.update(&missing, Revision::INITIAL).await.unwrap(),
        UpdateOutcome::NotFound
    );
}

#[sqlx::test]
async fn nutrition_targets_can_be_deleted(pool: PgPool) {
    let members = PgHouseholdMemberRepository::new(pool.clone());
    let repo = PgNutritionTargetRepository::new(pool);
    let joe = member("Joe");
    members.insert(&joe).await.unwrap();

    let original = target(
        joe.id,
        date!(2026 - 08 - 25),
        NutritionGoals {
            energy_kcal: Some(Decimal::new(2000, 0)),
            ..Default::default()
        },
    );
    repo.insert(&original).await.unwrap();

    let stale = repo
        .delete(original.id, original.revision.next())
        .await
        .unwrap();
    assert_eq!(
        stale,
        UpdateOutcome::RevisionMismatch {
            actual: original.revision
        }
    );

    assert_eq!(
        repo.delete(original.id, original.revision).await.unwrap(),
        UpdateOutcome::Updated
    );
    assert!(repo.get(original.id).await.unwrap().is_none());
}

#[sqlx::test]
async fn household_settings_are_seeded_with_defaults(pool: PgPool) {
    let repo = PgHouseholdSettingsRepository::new(pool);
    let settings = repo.get().await.unwrap();
    assert_eq!(settings.meal_times.breakfast, time!(08:00));
    assert_eq!(settings.meal_times.lunch, time!(12:30));
    assert_eq!(settings.meal_times.dinner, time!(18:00));
    assert_eq!(settings.revision, Revision::INITIAL);
}

#[sqlx::test]
async fn household_settings_updates_report_revision_outcomes(pool: PgPool) {
    let repo = PgHouseholdSettingsRepository::new(pool);
    let original = repo.get().await.unwrap();

    let mut updated = original;
    updated.meal_times.lunch = time!(13:15);
    updated.revision = original.revision.next();
    assert_eq!(
        repo.update(&updated, original.revision).await.unwrap(),
        UpdateOutcome::Updated
    );

    let stored = repo.get().await.unwrap();
    assert_eq!(stored.meal_times.lunch, time!(13:15));
    assert_eq!(stored.meal_times.breakfast, time!(08:00));

    let stale = repo.update(&updated, original.revision).await.unwrap();
    assert_eq!(
        stale,
        UpdateOutcome::RevisionMismatch {
            actual: updated.revision,
        }
    );
}

async fn seed_recipe_dependencies(pool: &PgPool) -> (UserId, ProductId, ProductId) {
    let users = PgUserRepository::new(pool.clone());
    let owner = user("cook", vec![Role::Admin]);
    users.insert(&owner).await.unwrap();
    let products = PgProductRepository::new(pool.clone());
    let first = product("Recipe product one");
    let second = product("Recipe product two");
    products.insert(&first).await.unwrap();
    products.insert(&second).await.unwrap();
    (owner.id, first.id, second.id)
}

fn recipe_component(product_id: ProductId, position: i32) -> RecipeComponent {
    RecipeComponent {
        id: RecipeComponentId::new(),
        requirement: RecipeRequirement::Product { product_id },
        source_text: None,
        amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(100, 0), Unit::Gram)),
        position,
    }
}

fn recipe_ingredient_component(ingredient_id: IngredientId, position: i32) -> RecipeComponent {
    RecipeComponent {
        id: RecipeComponentId::new(),
        requirement: RecipeRequirement::Ingredient { ingredient_id },
        source_text: None,
        amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(100, 0), Unit::Gram)),
        position,
    }
}

fn recipe_unresolved_component(text: &str, position: i32) -> RecipeComponent {
    RecipeComponent {
        id: RecipeComponentId::new(),
        requirement: RecipeRequirement::Unresolved {
            text: text.to_owned(),
        },
        source_text: None,
        amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(100, 0), Unit::Gram)),
        position,
    }
}

fn recipe(owner: UserId, components: Vec<RecipeComponent>) -> Recipe {
    let now = OffsetDateTime::now_utc();
    Recipe {
        id: RecipeId::new(),
        name: "Test Recipe".to_owned(),
        description: None,
        servings: 2,
        preparation_minutes: None,
        cooking_minutes: None,
        notes: None,
        components,
        instructions: vec![],
        meal_categories: vec![],
        country_categories: vec![],
        tags: vec![],
        photo_version: None,
        owner_id: owner,
        visibility: RecipeVisibility::Private,
        created_by: owner,
        updated_by: owner,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    }
}

#[sqlx::test]
async fn round_trips_a_recipe_with_ordered_components(pool: PgPool) {
    let (owner, first, second) = seed_recipe_dependencies(&pool).await;
    let repo = PgRecipeRepository::new(pool);
    let mut original = recipe(
        owner,
        vec![recipe_component(first, 0), recipe_component(second, 1)],
    );
    original.description = Some("A complete recipe".to_owned());
    original.preparation_minutes = Some(10);
    original.cooking_minutes = Some(20);
    original.notes = Some("Serve warm".to_owned());
    original.instructions = vec![
        RecipeInstruction {
            id: RecipeInstructionId::new(),
            text: "First".to_owned(),
            position: 0,
        },
        RecipeInstruction {
            id: RecipeInstructionId::new(),
            text: "Second".to_owned(),
            position: 1,
        },
    ];
    original.meal_categories = vec![MealCategory::Dinner];
    original.country_categories = vec!["GB".to_owned()];
    original.tags = vec!["Family".to_owned()];
    repo.insert(&original).await.unwrap();

    let loaded = repo.get(original.id).await.unwrap().unwrap();
    assert_eq!(loaded.name, "Test Recipe");
    assert_eq!(loaded.owner_id, owner);
    assert_eq!(loaded.visibility, RecipeVisibility::Private);
    assert_eq!(loaded.components.len(), 2);
    assert_eq!(loaded.components[0].position, 0);
    assert_eq!(
        loaded.components[0].requirement,
        RecipeRequirement::Product { product_id: first }
    );
    assert_eq!(
        loaded.components[1].requirement,
        RecipeRequirement::Product { product_id: second }
    );
    assert_eq!(loaded.description, original.description);
    assert_eq!(loaded.instructions, original.instructions);
    assert_eq!(loaded.meal_categories, vec![MealCategory::Dinner]);
    assert_eq!(loaded.country_categories, vec!["GB"]);
    assert_eq!(loaded.tags, vec!["Family"]);
}

#[sqlx::test]
async fn replaces_and_deletes_recipe_photo_derivatives(pool: PgPool) {
    let (owner, first, _second) = seed_recipe_dependencies(&pool).await;
    let repo = PgRecipeRepository::new(pool);
    let original = recipe(owner, vec![recipe_component(first, 0)]);
    repo.insert(&original).await.unwrap();

    let mut with_photo = original.clone();
    with_photo.photo_version = Some(1);
    with_photo.revision = Revision::new(2);
    let photo = RecipePhoto {
        recipe_id: original.id,
        version: 1,
        derivatives: RecipePhotoDerivatives {
            hero_jpeg: vec![1, 2, 3],
            card_jpeg: vec![4, 5],
            hero_width: 100,
            hero_height: 50,
            card_width: 50,
            card_height: 25,
        },
        updated_at: OffsetDateTime::now_utc(),
    };
    assert_eq!(
        repo.update_photo(&with_photo, original.revision, Some(&photo))
            .await
            .unwrap(),
        UpdateOutcome::Updated
    );
    assert_eq!(
        repo.get_photo(original.id)
            .await
            .unwrap()
            .unwrap()
            .derivatives
            .card_jpeg,
        vec![4, 5]
    );

    let mut without_photo = with_photo.clone();
    without_photo.photo_version = None;
    without_photo.revision = Revision::new(3);
    assert_eq!(
        repo.update_photo(&without_photo, with_photo.revision, None)
            .await
            .unwrap(),
        UpdateOutcome::Updated
    );
    assert!(repo.get_photo(original.id).await.unwrap().is_none());
}

#[sqlx::test]
async fn updating_a_recipe_reorders_and_preserves_component_ids(pool: PgPool) {
    let (owner, first, second) = seed_recipe_dependencies(&pool).await;
    let repo = PgRecipeRepository::new(pool);
    let original = recipe(
        owner,
        vec![recipe_component(first, 0), recipe_component(second, 1)],
    );
    repo.insert(&original).await.unwrap();

    let mut updated = original.clone();
    updated.components = vec![
        RecipeComponent {
            position: 0,
            ..original.components[1].clone()
        },
        RecipeComponent {
            position: 1,
            ..original.components[0].clone()
        },
    ];
    updated.revision = updated.revision.next();

    let outcome = repo.update(&updated, original.revision).await.unwrap();
    assert_eq!(outcome, UpdateOutcome::Updated);

    let loaded = repo.get(original.id).await.unwrap().unwrap();
    assert_eq!(loaded.components[0].id, original.components[1].id);
    assert_eq!(loaded.components[1].id, original.components[0].id);
}

#[sqlx::test]
async fn a_stale_recipe_update_is_rejected_and_keeps_components(pool: PgPool) {
    let (owner, first, second) = seed_recipe_dependencies(&pool).await;
    let repo = PgRecipeRepository::new(pool);
    let original = recipe(owner, vec![recipe_component(first, 0)]);
    repo.insert(&original).await.unwrap();

    let mut updated = original.clone();
    updated.components = vec![recipe_component(second, 0)];
    updated.revision = updated.revision.next();

    let outcome = repo.update(&updated, Revision::new(99)).await.unwrap();
    assert!(matches!(outcome, UpdateOutcome::RevisionMismatch { .. }));

    // The rollback must leave the original single component untouched.
    let loaded = repo.get(original.id).await.unwrap().unwrap();
    assert_eq!(loaded.components.len(), 1);
    assert_eq!(
        loaded.components[0].requirement,
        RecipeRequirement::Product { product_id: first }
    );
}

#[sqlx::test]
async fn a_recipe_round_trips_every_requirement_kind(pool: PgPool) {
    let (owner, product_id, _second) = seed_recipe_dependencies(&pool).await;
    let ingredients = PgIngredientRepository::new(pool.clone());
    let oats = ingredient("Rolled Oats");
    ingredients.insert(&oats).await.unwrap();

    let repo = PgRecipeRepository::new(pool.clone());
    let mut dish = recipe(
        owner,
        vec![
            recipe_component(product_id, 0),
            recipe_ingredient_component(oats.id, 1),
            recipe_unresolved_component("Jasmin Rice", 2),
        ],
    );
    dish.components[1].source_text = Some("imported: Rolld Oats".to_owned());
    repo.insert(&dish).await.unwrap();

    let loaded = repo.get(dish.id).await.unwrap().unwrap();
    assert_eq!(
        loaded.components[0].requirement,
        RecipeRequirement::Product { product_id }
    );
    assert_eq!(
        loaded.components[1].requirement,
        RecipeRequirement::Ingredient {
            ingredient_id: oats.id
        }
    );
    assert_eq!(
        loaded.components[2].requirement,
        RecipeRequirement::Unresolved {
            text: "Jasmin Rice".to_owned()
        }
    );
    assert_eq!(
        loaded.components[1].source_text.as_deref(),
        Some("imported: Rolld Oats")
    );
    assert_eq!(loaded.components[2].source_text, None);

    let summaries = repo
        .list(&RecipeQuery {
            owner_id: owner,
            include_archived: false,
            search: None,
            sort: SortDirection::Ascending,
            page: PageRequest::default(),
        })
        .await
        .unwrap();
    assert_eq!(summaries.items[0].unresolved_count, 1);
}

#[sqlx::test]
async fn a_recipe_component_needs_exactly_one_requirement(pool: PgPool) {
    let (owner, product_id, _second) = seed_recipe_dependencies(&pool).await;
    let repo = PgRecipeRepository::new(pool.clone());
    let dish = recipe(owner, vec![recipe_component(product_id, 0)]);
    repo.insert(&dish).await.unwrap();

    let none_set = sqlx::query(
        "INSERT INTO recipe_component (id, recipe_id, position, amount_kind, amount_value, amount_unit) \
         VALUES ($1, $2, 1, 'measure', 100, 'g')",
    )
    .bind(Uuid::now_v7())
    .bind(dish.id.as_uuid())
    .execute(&pool)
    .await;
    assert!(none_set.is_err());

    let two_set = sqlx::query(
        "INSERT INTO recipe_component (id, recipe_id, position, ingredient_id, unresolved_text, amount_kind, amount_value, amount_unit) \
         VALUES ($1, $2, 1, $3, 'text', 'measure', 100, 'g')",
    )
    .bind(Uuid::now_v7())
    .bind(dish.id.as_uuid())
    .bind(product_id.as_uuid())
    .execute(&pool)
    .await;
    assert!(two_set.is_err());
}

#[sqlx::test]
async fn a_recipe_component_referencing_a_missing_ingredient_names_the_ingredient(pool: PgPool) {
    let (owner, _first, _second) = seed_recipe_dependencies(&pool).await;
    let repo = PgRecipeRepository::new(pool.clone());
    let dish = recipe(
        owner,
        vec![recipe_ingredient_component(IngredientId::new(), 0)],
    );

    let error = repo.insert(&dish).await.unwrap_err();
    assert!(
        matches!(
            error,
            CoreError::NotFound {
                resource: "ingredient",
                ..
            }
        ),
        "{error:?}"
    );
}

#[sqlx::test]
async fn products_are_listed_by_the_ingredient_they_fulfil(pool: PgPool) {
    let ingredients = PgIngredientRepository::new(pool.clone());
    let products = PgProductRepository::new(pool.clone());
    let oats = ingredient("Rolled Oats");
    let rice = ingredient("Basmati Rice");
    ingredients.insert(&oats).await.unwrap();
    ingredients.insert(&rice).await.unwrap();

    let mut branded = product("Sainsbury Oats");
    branded.mapped_ingredient_id = Some(oats.id);
    let mut own_brand = product("Tesco Oats");
    own_brand.mapped_ingredient_id = Some(oats.id);
    let mut archived = product("Discontinued Oats");
    archived.mapped_ingredient_id = Some(oats.id);
    archived.archived_at = Some(OffsetDateTime::now_utc());
    for item in [&branded, &own_brand, &archived] {
        products.insert(item).await.unwrap();
    }

    let grouped = products
        .list_by_ingredient(&[oats.id, rice.id])
        .await
        .unwrap();
    let oats_products = grouped.get(&oats.id).unwrap();
    assert_eq!(oats_products.len(), 2);
    assert_eq!(oats_products[0].name, "Sainsbury Oats");
    assert_eq!(oats_products[1].name, "Tesco Oats");
    assert!(grouped.get(&rice.id).is_none_or(Vec::is_empty));
}

#[sqlx::test]
async fn lists_recipes_scoped_to_owner_and_excludes_archived(pool: PgPool) {
    let (owner, first, _second) = seed_recipe_dependencies(&pool).await;
    let stranger = user("stranger", vec![Role::Admin]);
    PgUserRepository::new(pool.clone())
        .insert(&stranger)
        .await
        .unwrap();
    let repo = PgRecipeRepository::new(pool);

    repo.insert(&recipe(owner, vec![recipe_component(first, 0)]))
        .await
        .unwrap();
    let mut archived = recipe(owner, vec![recipe_component(first, 0)]);
    archived.archived_at = Some(OffsetDateTime::now_utc());
    repo.insert(&archived).await.unwrap();
    repo.insert(&recipe(stranger.id, vec![recipe_component(first, 0)]))
        .await
        .unwrap();

    let query = RecipeQuery {
        owner_id: owner,
        search: None,
        include_archived: false,
        page: PageRequest::default(),
        sort: SortDirection::Ascending,
    };
    let page = repo.list(&query).await.unwrap();
    assert_eq!(
        page.total, 1,
        "only the owner's non-archived recipe should list"
    );

    let with_archived = RecipeQuery {
        include_archived: true,
        ..query
    };
    let page = repo.list(&with_archived).await.unwrap();
    assert_eq!(
        page.total, 2,
        "including archived shows both of the owner's recipes"
    );
}

#[sqlx::test]
async fn round_trips_a_meal_plan_entry_with_a_recipe_component(pool: PgPool) {
    let (member_id, product_id, actor_id) = seed_meal_plan_dependencies(&pool).await;
    let recipes = PgRecipeRepository::new(pool.clone());
    let dish = recipe(actor_id, vec![recipe_component(product_id, 0)]);
    recipes.insert(&dish).await.unwrap();

    let repo = PgMealPlanRepository::new(pool.clone());
    let mut entry = meal_plan_entry(member_id, product_id, actor_id);
    entry.components = vec![MealPlanComponent {
        id: MealPlanComponentId::new(),
        item: MealItemRef::recipe(dish.id),
        amount: ConsumedAmount::Servings(Decimal::new(2, 0)),
        position: 0,
        snapshot: None,
        status: MealPlanStatus::Planned,
        resolved_by: None,
        resolved_at: None,
        revision: Revision::INITIAL,
        display_order: Uuid::now_v7(),
    }];
    repo.insert(&entry).await.unwrap();

    let loaded = repo.get(entry.id).await.unwrap().unwrap();
    assert_eq!(loaded.components.len(), 1);
    assert_eq!(loaded.components[0].item, MealItemRef::recipe(dish.id));
    assert_eq!(
        loaded.components[0].amount,
        ConsumedAmount::Servings(Decimal::new(2, 0))
    );
}

#[sqlx::test]
async fn a_recipe_component_referencing_a_missing_recipe_is_rejected(pool: PgPool) {
    let (member_id, product_id, actor_id) = seed_meal_plan_dependencies(&pool).await;
    let repo = PgMealPlanRepository::new(pool.clone());
    let mut entry = meal_plan_entry(member_id, product_id, actor_id);
    entry.components = vec![MealPlanComponent {
        id: MealPlanComponentId::new(),
        item: MealItemRef::recipe(RecipeId::new()),
        amount: ConsumedAmount::Servings(Decimal::ONE),
        position: 0,
        snapshot: None,
        status: MealPlanStatus::Planned,
        resolved_by: None,
        resolved_at: None,
        revision: Revision::INITIAL,
        display_order: Uuid::now_v7(),
    }];

    let error = repo.insert(&entry).await.unwrap_err();
    assert!(
        matches!(
            error,
            CoreError::NotFound {
                resource: "recipe",
                ..
            }
        ),
        "{error:?}"
    );
}

#[sqlx::test]
async fn logs_and_reloads_a_recipe_consumption_record(pool: PgPool) {
    let (member_id, product_id, actor_id) = seed_meal_plan_dependencies(&pool).await;
    let recipes = PgRecipeRepository::new(pool.clone());
    let dish = recipe(actor_id, vec![recipe_component(product_id, 0)]);
    recipes.insert(&dish).await.unwrap();

    let repo = PgConsumptionRecordRepository::new(pool.clone());
    let mut record = consumption_record(member_id, product_id);
    record.item = MealItemRef::recipe(dish.id);
    record.amount = ConsumedAmount::Servings(Decimal::ONE);
    repo.insert(&record).await.unwrap();

    let loaded = repo.get(record.id).await.unwrap().unwrap();
    assert_eq!(loaded.item, MealItemRef::recipe(dish.id));
}

fn added_event(actor: UserId, subject: HouseholdMemberId) -> NewStockEvent {
    NewStockEvent {
        kind: StockEventKind::Added,
        quantity_delta: None,
        actor_user_id: Some(actor),
        subject_member_id: Some(subject),
        note: None,
    }
}

#[sqlx::test]
async fn round_trips_a_stock_item_and_writes_an_event(pool: PgPool) {
    let users = PgUserRepository::new(pool.clone());
    let actor = user("stockkeeper", vec![Role::Admin]);
    users.insert(&actor).await.unwrap();
    let (member_id, product_id) = seed_member_and_product(&pool).await;

    let repo = PgStockRepository::new(pool.clone());
    let item = mmp_core::domain::StockItem {
        id: StockItemId::new(),
        product_id,
        level: StockLevel::Exact {
            quantity: Quantity::new(Decimal::new(400, 0), Unit::Gram),
        },
        storage_location: StorageLocation::Chilled,
        source_date: None,
        usability_deadline: None,
        note: Some("back left".to_owned()),
        revision: Revision::INITIAL,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        archived_at: None,
    };
    repo.insert(&item, &added_event(actor.id, member_id))
        .await
        .unwrap();

    let loaded = repo.get(item.id).await.unwrap().expect("should exist");
    assert_eq!(
        loaded.level,
        StockLevel::Exact {
            quantity: Quantity::new(Decimal::new(400, 0), Unit::Gram)
        }
    );
    assert_eq!(loaded.storage_location, StorageLocation::Chilled);

    let events = repo.list_events(item.id).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, StockEventKind::Added);
    assert_eq!(events[0].actor_user_id, Some(actor.id));
    assert_eq!(events[0].subject_member_id, Some(member_id));
}

#[sqlx::test]
async fn several_stock_rows_may_share_a_product(pool: PgPool) {
    let users = PgUserRepository::new(pool.clone());
    let actor = user("stockkeeper", vec![Role::Admin]);
    users.insert(&actor).await.unwrap();
    let (member_id, product_id) = seed_member_and_product(&pool).await;
    let repo = PgStockRepository::new(pool.clone());

    for grams in [400, 650] {
        let now = OffsetDateTime::now_utc();
        let item = mmp_core::domain::StockItem {
            id: StockItemId::new(),
            product_id,
            level: StockLevel::Exact {
                quantity: Quantity::new(Decimal::new(grams, 0), Unit::Gram),
            },
            storage_location: StorageLocation::Frozen,
            source_date: None,
            usability_deadline: None,
            note: None,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
            archived_at: None,
        };
        repo.insert(&item, &added_event(actor.id, member_id))
            .await
            .unwrap();
    }

    let rows = repo.list_for_products(&[product_id]).await.unwrap();
    assert_eq!(rows.len(), 2);
    let page = repo
        .list(&StockQuery {
            product_id: Some(product_id),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(page.total, 2);
}

#[sqlx::test]
async fn a_stale_stock_update_is_refused(pool: PgPool) {
    let users = PgUserRepository::new(pool.clone());
    let actor = user("stockkeeper", vec![Role::Admin]);
    users.insert(&actor).await.unwrap();
    let (member_id, product_id) = seed_member_and_product(&pool).await;
    let repo = PgStockRepository::new(pool.clone());

    let now = OffsetDateTime::now_utc();
    let mut item = mmp_core::domain::StockItem {
        id: StockItemId::new(),
        product_id,
        level: StockLevel::Exact {
            quantity: Quantity::new(Decimal::new(400, 0), Unit::Gram),
        },
        storage_location: StorageLocation::Chilled,
        source_date: None,
        usability_deadline: None,
        note: None,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    };
    repo.insert(&item, &added_event(actor.id, member_id))
        .await
        .unwrap();

    item.level = StockLevel::Exact {
        quantity: Quantity::new(Decimal::new(150, 0), Unit::Gram),
    };
    item.revision = Revision::INITIAL.next();
    let stale = Revision::new(99);
    let outcome = repo
        .update(&item, stale, &added_event(actor.id, member_id))
        .await
        .unwrap();
    assert!(matches!(outcome, UpdateOutcome::RevisionMismatch { .. }));

    let events = repo.list_events(item.id).await.unwrap();
    assert_eq!(events.len(), 1, "a refused update must not write an event");
}
