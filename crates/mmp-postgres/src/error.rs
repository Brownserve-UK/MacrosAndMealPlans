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

const UNIQUE_CONSTRAINTS: [(&str, &str, &str); 43] = [
    ("ingredient_name_unique", "ingredient", "name"),
    ("ingredient_seed_key_unique", "ingredient", "seed_key"),
    ("ingredient_pkey", "ingredient", "id"),
    ("prepared_meal_name_unique", "prepared meal", "name"),
    ("prepared_meal_seed_key_unique", "prepared meal", "seed_key"),
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
    ("member_body_profile_pkey", "member body profile", "member"),
    (
        "calorie_target_calculation_pkey",
        "calorie target calculation",
        "id",
    ),
    (
        "calorie_target_calculation_target_unique",
        "calorie target calculation",
        "nutrition target",
    ),
    ("weight_record_pkey", "weight record", "id"),
    ("weight_goal_pkey", "weight goal", "id"),
    ("weight_goal_member_unique", "weight goal", "member"),
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
    ("shopping_trip_one_per_shop", "shopping trip", "date"),
    ("shopping_trip_pkey", "shopping trip", "id"),
    ("shopping_trip_row_pkey", "shopping trip row", "id"),
    ("shopping_list_item_pkey", "shopping list item", "id"),
    ("purchase_pkey", "purchase", "id"),
    (
        "shopping_suggestion_dismissal_ingredient",
        "shopping suggestion dismissal",
        "ingredient",
    ),
    (
        "shopping_suggestion_dismissal_product",
        "shopping suggestion dismissal",
        "product",
    ),
    (
        "shopping_suggestion_dismissal_prepared_meal",
        "shopping suggestion dismissal",
        "prepared meal",
    ),
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
        "meal_plan_component_entry_id_position_unique" => Some(("meal plan component", "position")),
        "recipe_component_recipe_id_position_unique" => Some(("recipe component", "position")),
        "recipe_instruction_recipe_id_position_unique" => Some(("recipe instruction", "position")),
        "recipe_meal_category_recipe_id_position_unique" => {
            Some(("recipe meal category", "position"))
        }
        "recipe_country_category_recipe_id_position_unique" => {
            Some(("recipe country category", "position"))
        }
        "recipe_tag_recipe_id_position_unique" => Some(("recipe tag", "position")),
        "meal_template_component_template_id_position_unique" => {
            Some(("meal template component", "position"))
        }
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
    if constraint.contains("nutrition_target_id") {
        return "nutrition target";
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
