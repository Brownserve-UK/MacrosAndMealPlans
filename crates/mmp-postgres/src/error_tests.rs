use super::*;

#[test]
fn constraint_names_reduce_to_a_field() {
    assert_eq!(
        constraint_field("ingredient_name_not_blank"),
        "name_not_blank"
    );
    assert_eq!(constraint_field("product_barcode_valid"), "barcode_valid");
    assert_eq!(
        constraint_field("household_member_display_name_not_blank"),
        "display_name_not_blank"
    );
    assert_eq!(
        constraint_field("app_user_username_not_blank"),
        "username_not_blank"
    );
    assert_eq!(constraint_field("something_else"), "something_else");
}

#[test]
fn every_unique_index_in_the_schema_maps_to_a_duplicate() {
    let migration = include_str!("../migrations/0001_init.sql");

    for line in migration.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("CREATE UNIQUE INDEX ") else {
            continue;
        };
        let name = rest.split_whitespace().next().unwrap();
        assert!(
            unique_violation(name).is_some(),
            "`{name}` has no mapping, so a race would surface as a 500 rather than a 409"
        );
    }
}

#[test]
fn the_household_unique_constraints_are_mapped() {
    for (constraint, resource, field) in [
        ("app_user_username_unique", "user", "username"),
        (
            "household_member_display_name_unique",
            "household member",
            "name",
        ),
        (
            "household_member_linked_user_id_key",
            "household member",
            "account",
        ),
    ] {
        assert_eq!(
            unique_violation(constraint),
            Some((resource, field)),
            "{constraint}"
        );
    }
}

#[test]
fn an_unknown_unique_constraint_is_not_guessed_at() {
    assert_eq!(unique_violation("some_future_table_thing_unique"), None);
}

#[test]
fn foreign_keys_name_the_thing_that_is_missing() {
    assert_eq!(
        foreign_key_target("product_mapped_ingredient_id_fkey"),
        "ingredient"
    );
    assert_eq!(
        foreign_key_target("household_member_linked_user_id_fkey"),
        "user"
    );
    assert_eq!(foreign_key_target("ingredient_created_by_fkey"), "user");
    assert_eq!(
        foreign_key_target("member_access_grant_subject_member_id_fkey"),
        "household member"
    );
    assert_eq!(
        foreign_key_target("consumption_record_product_id_fkey"),
        "product"
    );
}

#[test]
fn a_missing_recipe_reads_as_a_missing_recipe_not_a_missing_ingredient() {
    assert_eq!(
        foreign_key_target("meal_plan_component_recipe_id_fkey"),
        "recipe"
    );
    assert_eq!(
        foreign_key_target("consumption_record_recipe_id_fkey"),
        "recipe"
    );
    assert_eq!(
        foreign_key_target("recipe_component_ingredient_id_fkey"),
        "ingredient"
    );
    assert_eq!(
        foreign_key_target("recipe_component_product_id_fkey"),
        "product"
    );
}

#[test]
fn an_unmapped_foreign_key_does_not_pretend_to_know_the_target() {
    assert_eq!(
        foreign_key_target("some_future_table_other_id_fkey"),
        "referenced record"
    );
}

#[test]
fn recipe_child_constraints_reduce_to_a_field() {
    assert_eq!(
        constraint_field("recipe_component_amount_kind_valid"),
        "amount_kind_valid"
    );
    assert_eq!(
        constraint_field("recipe_instruction_not_blank"),
        "not_blank"
    );
    assert_eq!(
        constraint_field("recipe_servings_positive"),
        "servings_positive"
    );
}

#[test]
fn a_non_database_error_becomes_a_repository_error() {
    let mapped = map_db_error(sqlx::Error::PoolClosed, "listing ingredients");
    assert!(matches!(mapped, CoreError::Repository(_)));
}
