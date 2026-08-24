#![cfg(feature = "db-tests")]

use mmp_core::CoreError;
use mmp_core::domain::{
    AccessScope, CatalogueOrigin, ConsumedAmount, ConsumptionRecord, ConsumptionRecordId,
    HouseholdMember, HouseholdMemberId, Ingredient, IngredientId, MemberAccessGrant,
    NutritionFacts, NutritionQuality, Product, ProductId, Provenance, Quantity, Revision, Role,
    Unit, User, UserId,
};
use mmp_core::ports::{
    AccessGrantRepository, ConsumptionQuery, ConsumptionRecordRepository,
    HouseholdMemberRepository, IngredientQuery, IngredientRepository, MemberQuery, PageRequest,
    ProductQuery, ProductRepository, UpdateOutcome, UserRepository,
};
use mmp_postgres::{
    PgAccessGrantRepository, PgConsumptionRecordRepository, PgHouseholdMemberRepository,
    PgIngredientRepository, PgProductRepository, PgUserRepository,
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::OffsetDateTime;
use time::macros::date;

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
        product_id,
        recorded_by: None,
        amount: ConsumedAmount::Measure(Quantity::new(Decimal::new(150, 0), Unit::Millilitre)),
        consumed_on: date!(2026 - 08 - 22),
        consumed_at: now,
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
    assert_eq!(loaded.nutrition.energy_kcal, Some(Decimal::new(96, 0)));
    assert_eq!(loaded.quality, NutritionQuality::Partial);
    assert_eq!(loaded.consumed_on, date!(2026 - 08 - 22));
    assert_eq!(loaded.revision, Revision::INITIAL);
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
