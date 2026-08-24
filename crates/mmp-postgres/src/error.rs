use mmp_core::{CoreError, RepositoryError, ValidationErrors};

const UNIQUE_VIOLATION: &str = "23505";
const FOREIGN_KEY_VIOLATION: &str = "23503";
const CHECK_VIOLATION: &str = "23514";

pub fn map_db_error(error: sqlx::Error, context: &str) -> CoreError {
    let Some(db) = error.as_database_error() else {
        return CoreError::Repository(RepositoryError::with_source(
            format!("{context} failed"),
            error,
        ));
    };

    let code = db.code().unwrap_or_default().to_string();
    let constraint = db.constraint().unwrap_or_default().to_string();

    match code.as_str() {
        UNIQUE_VIOLATION => match unique_violation(&constraint) {
            Some((resource, field)) => CoreError::Duplicate {
                resource,
                field,
                value: String::new(),
            },
            None => CoreError::Repository(RepositoryError::with_source(
                format!("{context} violated the unique constraint `{constraint}`"),
                error,
            )),
        },
        FOREIGN_KEY_VIOLATION => {
            CoreError::not_found(foreign_key_target(&constraint), "referenced here")
        }
        CHECK_VIOLATION => {
            let mut errors = ValidationErrors::new();
            errors.push(constraint_field(&constraint), "Not valid");
            CoreError::Validation(errors)
        }
        _ => CoreError::Repository(RepositoryError::with_source(
            format!("{context} failed"),
            error,
        )),
    }
}

const TABLES: [(&str, &str); 6] = [
    ("household_member_", "household member"),
    ("member_access_grant_", "access grant"),
    ("ingredient_", "ingredient"),
    ("app_user_", "user"),
    ("product_", "product"),
    ("consumption_record_", "consumption record"),
];

const UNIQUE_CONSTRAINTS: [(&str, &str, &str); 11] = [
    ("ingredient_name_unique", "ingredient", "name"),
    ("ingredient_seed_key_unique", "ingredient", "seed_key"),
    ("ingredient_pkey", "ingredient", "id"),
    ("product_barcode_unique", "product", "barcode"),
    ("product_seed_key_unique", "product", "seed_key"),
    ("product_pkey", "product", "id"),
    ("app_user_username_unique", "user", "username"),
    ("app_user_auth_subject_unique", "user", "auth_subject"),
    ("app_user_pkey", "user", "id"),
    (
        "household_member_display_name_unique",
        "household member",
        "name",
    ),
    ("consumption_record_pkey", "consumption record", "id"),
];

fn unique_violation(constraint: &str) -> Option<(&'static str, &'static str)> {
    if let Some((_, resource, field)) = UNIQUE_CONSTRAINTS.iter().find(|(c, _, _)| *c == constraint)
    {
        return Some((resource, field));
    }

    match constraint {
        "household_member_linked_user_id_key" => Some(("household member", "account")),
        "household_member_pkey" => Some(("household member", "id")),
        "member_access_grant_pkey" => Some(("access grant", "id")),
        "user_role_pkey" => Some(("user", "role")),
        _ => None,
    }
}

fn foreign_key_target(constraint: &str) -> &'static str {
    if constraint.contains("ingredient_id") {
        return "ingredient";
    }
    if constraint.contains("product_id") {
        return "product";
    }
    if constraint.contains("user_id") || constraint.contains("_by_fkey") {
        return "user";
    }
    if constraint.contains("member_id") {
        return "household member";
    }
    "ingredient"
}

fn constraint_field(constraint: &str) -> String {
    TABLES
        .iter()
        .find_map(|(prefix, _)| constraint.strip_prefix(prefix))
        .unwrap_or(constraint)
        .to_owned()
}

pub fn repository_error(context: &str, error: sqlx::Error) -> CoreError {
    CoreError::Repository(RepositoryError::with_source(
        format!("{context} failed"),
        error,
    ))
}

#[cfg(test)]
mod tests {
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
    fn a_non_database_error_becomes_a_repository_error() {
        let mapped = map_db_error(sqlx::Error::PoolClosed, "listing ingredients");
        assert!(matches!(mapped, CoreError::Repository(_)));
    }
}
