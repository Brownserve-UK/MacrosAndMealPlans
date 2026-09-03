use mmp_core::{CoreError, RepositoryError};

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
        CHECK_VIOLATION => CoreError::Repository(RepositoryError::with_source(
            format!("{context} violated the check constraint `{constraint}`"),
            error,
        )),
        _ => CoreError::Repository(RepositoryError::with_source(
            format!("{context} failed"),
            error,
        )),
    }
}

const UNIQUE_CONSTRAINTS: [(&str, &str, &str); 28] = [
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
    ("meal_plan_entry_pkey", "meal plan entry", "id"),
    (
        "meal_plan_entry_member_day_slot_unique",
        "meal plan entry",
        "slot",
    ),
    (
        "meal_plan_entry_member_day_snack_time_unique",
        "meal plan entry",
        "time",
    ),
    ("meal_plan_component_pkey", "meal plan component", "id"),
    (
        "meal_plan_opt_out_entry_member_unique",
        "meal plan opt-out",
        "member",
    ),
    (
        "consumption_record_meal_plan_component_member_unique",
        "consumption record",
        "meal_plan_component_id",
    ),
    (
        "meal_plan_participant_member_occurrence_unique",
        "meal plan participant",
        "slot",
    ),
    (
        "meal_plan_participant_member_snack_time_unique",
        "meal plan participant",
        "time",
    ),
    (
        "meal_plan_participant_entry_member_unique",
        "meal plan participant",
        "member",
    ),
    (
        "meal_plan_participant_allocation_unique",
        "participant allocation",
        "component",
    ),
    ("nutrition_target_pkey", "nutrition target", "id"),
    (
        "nutrition_target_member_effective_from_unique",
        "nutrition target",
        "effective_from",
    ),
    (
        "stock_effect_active_source_item_unique",
        "stock effect",
        "source",
    ),
    (
        "shopping_opportunity_generated_for_unique",
        "shopping opportunity",
        "date",
    ),
    ("shopping_opportunity_pkey", "shopping opportunity", "id"),
    ("shopping_cadence_pkey", "shopping cadence", "id"),
    ("purchase_pkey", "purchase", "id"),
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
        "meal_plan_component_entry_id_position_key" => Some(("meal plan component", "position")),
        "recipe_tag_case_insensitive" => Some(("recipe", "tag")),
        "user_role_pkey" => Some(("user", "role")),
        _ => None,
    }
}

fn foreign_key_target(constraint: &str) -> &'static str {
    if constraint.contains("recipe_id") {
        return "recipe";
    }
    if constraint.contains("ingredient_id") {
        return "ingredient";
    }
    if constraint.contains("product_id") {
        return "product";
    }
    if constraint.contains("recipe_component_id") {
        return "recipe component";
    }
    if constraint.contains("meal_plan_component_id") {
        return "meal plan component";
    }
    if constraint.contains("user_id") || constraint.contains("_by_fkey") {
        return "user";
    }
    if constraint.contains("member_id") {
        return "household member";
    }
    "referenced record"
}

pub fn repository_error(context: &str, error: sqlx::Error) -> CoreError {
    CoreError::Repository(RepositoryError::with_source(
        format!("{context} failed"),
        error,
    ))
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
