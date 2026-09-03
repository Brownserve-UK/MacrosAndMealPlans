use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Macros & Meal Plans",
        description = "Food catalogue API for a self-hosted Macros & Meal Plans installation.",
        license(name = "GPL-3.0-or-later")
    ),
    tags(
        (name = "meta", description = "Server and protocol metadata"),
        (name = "auth", description = "Authentication"),
        (name = "ingredients", description = "Generic food concepts used by recipes"),
        (name = "products", description = "Specific purchasable items"),
        (name = "household", description = "The people in this installation's household"),
        (name = "accounts", description = "Sign-in accounts and their roles"),
        (name = "diary", description = "What was actually eaten, and when"),
        (name = "meal-plan", description = "What the signed-in household member intends to eat"),
        (name = "needs-review", description = "Items that need attention"),
        (name = "nutrition-targets", description = "Per-member calorie, macro and dietary targets"),
        (name = "recipes", description = "Reusable multi-component recipes with derived nutrition"),
        (name = "stock", description = "Physical household stock and its availability against plans"),
        (name = "shopping", description = "What needs buying, when the household shops, and what they bought"),
    ),
    components(schemas(
        crate::dto::SortDirectionDto,
        crate::dto::IngredientSortDto,
        crate::dto::AmountKindDto
    )),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "basic",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Basic)
                    .description(Some(
                        "Development authentication only. Replaced by OpenID Connect before \
                         public release.",
                    ))
                    .build(),
            ),
        );
    }
}

impl ApiDoc {
    pub fn openapi_with_routes() -> String {
        let (_, api) = crate::app::build(crate::app::stub_state());
        serde_json::to_string_pretty(&api).expect("the OpenAPI document should serialise")
    }
}
