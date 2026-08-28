use std::io::Cursor;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use http_body_util::BodyExt;
use image::{DynamicImage, ImageFormat, RgbImage};
use mmp_core::ports::FixedClock;
use mmp_core::services::{
    CatalogueService, DiaryService, HouseholdService, HouseholdSettingsService, MealPlanService,
    NutritionTargetService, RecipeService,
};
use mmp_core::testing::{
    InMemoryAccessGrantRepository, InMemoryConsumptionRecordRepository,
    InMemoryHouseholdMemberRepository, InMemoryHouseholdSettingsRepository,
    InMemoryIngredientRepository, InMemoryMealPlanRepository, InMemoryNutritionTargetRepository,
    InMemoryProductRepository, InMemoryRecipeRepository, InMemoryUserRepository,
};
use mmp_server::AppState;
use mmp_server::auth::DevBasicAuthProvider;
use serde_json::{Value, json};
use time::macros::datetime;
use tower::ServiceExt;

const USER: &str = "admin";
const PASSWORD: &str = "changeme";

async fn app() -> Router {
    let clock = Arc::new(FixedClock::new(datetime!(2026-08-26 12:00 UTC)));
    let household = Arc::new(HouseholdService::new(
        Arc::new(InMemoryHouseholdMemberRepository::new()),
        Arc::new(InMemoryUserRepository::new()),
        Arc::new(InMemoryAccessGrantRepository::new()),
        clock.clone(),
    ));

    mmp_server::bootstrap::ensure_bootstrap_user(&household, USER)
        .await
        .expect("the bootstrap admin should be created");

    let products = InMemoryProductRepository::new();
    let consumption = InMemoryConsumptionRecordRepository::new();
    let targets = InMemoryNutritionTargetRepository::new();
    let recipes_repo = Arc::new(InMemoryRecipeRepository::new());
    let recipes = RecipeService::new(
        recipes_repo.clone(),
        Arc::new(products.clone()),
        clock.clone(),
    );
    let state = AppState::new(
        CatalogueService::new(
            Arc::new(InMemoryIngredientRepository::new()),
            Arc::new(products.clone()),
            clock.clone(),
        ),
        household.clone(),
        HouseholdSettingsService::new(
            Arc::new(InMemoryHouseholdSettingsRepository::new()),
            clock.clone(),
        ),
        DiaryService::new(
            Arc::new(consumption.clone()),
            Arc::new(products.clone()),
            recipes_repo.clone(),
            clock.clone(),
        ),
        MealPlanService::new(
            Arc::new(InMemoryMealPlanRepository::new(consumption.clone())),
            Arc::new(products),
            recipes_repo,
            Arc::new(consumption),
            Arc::new(targets.clone()),
            clock.clone(),
        ),
        NutritionTargetService::new(Arc::new(targets), clock),
        recipes,
        Arc::new(DevBasicAuthProvider::new(household, PASSWORD)),
    );
    mmp_server::app::build(state).0
}

fn credential() -> String {
    credential_for(USER)
}

fn credential_for(username: &str) -> String {
    format!(
        "Basic {}",
        STANDARD.encode(format!("{username}:{PASSWORD}"))
    )
}

struct Call {
    method: &'static str,
    path: String,
    body: Option<Value>,
    if_match: Option<String>,
    auth: bool,
    as_user: Option<String>,
}

impl Call {
    fn new(method: &'static str, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            body: None,
            if_match: None,
            auth: true,
            as_user: None,
        }
    }

    fn signed_in_as(mut self, username: &str) -> Self {
        self.as_user = Some(username.to_owned());
        self
    }

    fn body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    fn if_match(mut self, revision: impl std::fmt::Display) -> Self {
        self.if_match = Some(format!("\"{revision}\""));
        self
    }

    fn anonymous(mut self) -> Self {
        self.auth = false;
        self
    }
}

async fn send(app: &Router, call: Call) -> (StatusCode, Value, axum::http::HeaderMap) {
    let mut builder = Request::builder().method(call.method).uri(&call.path);
    if call.auth {
        let value = match &call.as_user {
            Some(username) => credential_for(username),
            None => credential(),
        };
        builder = builder.header(header::AUTHORIZATION, value);
    }
    if let Some(value) = &call.if_match {
        builder = builder.header(header::IF_MATCH, value);
    }
    let request = match call.body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value, headers)
}

async fn send_bytes(
    app: &Router,
    method: &'static str,
    path: impl Into<String>,
    content_type: &'static str,
    body: Vec<u8>,
    revision: Option<&str>,
) -> (StatusCode, Vec<u8>, axum::http::HeaderMap) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path.into())
        .header(header::AUTHORIZATION, credential())
        .header(header::CONTENT_TYPE, content_type);
    if let Some(revision) = revision {
        builder = builder.header(header::IF_MATCH, format!("\"{revision}\""));
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    (status, bytes, headers)
}

fn etag(headers: &axum::http::HeaderMap) -> String {
    headers
        .get(header::ETAG)
        .expect("an ETag should be present")
        .to_str()
        .unwrap()
        .trim_matches('"')
        .to_owned()
}

async fn create_ingredient(app: &Router, name: &str) -> Value {
    let (status, body, _) = send(
        app,
        Call::new("POST", "/api/v1/ingredients").body(json!({"name": name, "default_unit": "g"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body
}

#[tokio::test]
async fn rejects_anonymous_requests() {
    let app = app().await;
    let (status, body, headers) =
        send(&app, Call::new("GET", "/api/v1/ingredients").anonymous()).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["status"], 401);
    assert!(
        !headers.contains_key(header::WWW_AUTHENTICATE),
        "the API must not trigger the browser's own Basic auth prompt"
    );
}

#[tokio::test]
async fn errors_use_problem_json() {
    let app = app().await;
    let (_, _, headers) = send(&app, Call::new("GET", "/api/v1/ingredients").anonymous()).await;
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
}

#[tokio::test]
async fn meta_reports_protocol_compatibility() {
    let app = app().await;
    let (status, body, _) = send(&app, Call::new("GET", "/api/v1/meta").anonymous()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["protocol_version"], 1);
    assert_eq!(body["supported_client_protocol_versions"]["min"], 1);
    assert!(body["server_version"].is_string());
}

#[tokio::test]
async fn session_endpoint_reports_the_principal() {
    let app = app().await;
    let (status, body, _) = send(&app, Call::new("POST", "/api/v1/auth/session")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["username"], "admin");
    assert_eq!(body["roles"][0], "admin");
    assert!(
        body["permissions"]
            .as_array()
            .unwrap()
            .contains(&json!("catalogue:write"))
    );
}

#[tokio::test]
async fn creates_an_ingredient_with_an_etag() {
    let app = app().await;
    let (status, body, headers) = send(
        &app,
        Call::new("POST", "/api/v1/ingredients")
            .body(json!({"name": "Whole Milk", "default_unit": "ml"})),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["name"], "Whole Milk");
    assert_eq!(body["default_unit"], "ml");
    assert_eq!(body["revision"], 1);
    assert_eq!(body["provenance"]["origin"], "local");
    assert_eq!(etag(&headers), "1");
}

#[tokio::test]
async fn an_ingredient_carries_no_nutrition() {
    let app = app().await;
    let body = create_ingredient(&app, "Coriander").await;
    assert!(
        body.get("nutrition").is_none(),
        "nutrition lives on products, not ingredients: {body}"
    );
}

#[tokio::test]
async fn product_nutrition_values_are_json_numbers() {
    let app = app().await;
    let (status, body, _) = send(
        &app,
        Call::new("POST", "/api/v1/products").body(json!({
            "name": "Tesco Whole Milk 1L",
            "nutrition": {"basis": {"amount": 100.0, "unit": "ml"}, "energy_kcal": 64.5, "fat_g": 3.6}
        })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["nutrition"]["energy_kcal"], json!(64.5));
    assert!(
        body["nutrition"]["protein_g"].is_null(),
        "unknown stays null, never zero"
    );
}

#[tokio::test]
async fn updating_without_if_match_is_precondition_required() {
    let app = app().await;
    let created = create_ingredient(&app, "Whole Milk").await;
    let id = created["id"].as_str().unwrap();

    let (status, body, _) = send(
        &app,
        Call::new("PATCH", format!("/api/v1/ingredients/{id}")).body(json!({"name": "Milk"})),
    )
    .await;

    assert_eq!(status, StatusCode::PRECONDITION_REQUIRED);
    assert!(body["detail"].as_str().unwrap().contains("If-Match"));
}

#[tokio::test]
async fn updating_with_a_stale_revision_conflicts() {
    let app = app().await;
    let created = create_ingredient(&app, "Whole Milk").await;
    let id = created["id"].as_str().unwrap();

    let (status, _, _) = send(
        &app,
        Call::new("PATCH", format!("/api/v1/ingredients/{id}"))
            .if_match(1)
            .body(json!({"name": "Semi Skimmed Milk"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body, _) = send(
        &app,
        Call::new("PATCH", format!("/api/v1/ingredients/{id}"))
            .if_match(1)
            .body(json!({"name": "Skimmed Milk"})),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["expected_revision"], 1);
    assert_eq!(body["actual_revision"], 2);
}

#[tokio::test]
async fn a_wildcard_if_match_is_refused() {
    let app = app().await;
    let created = create_ingredient(&app, "Whole Milk").await;
    let id = created["id"].as_str().unwrap();

    let request = Request::builder()
        .method("PATCH")
        .uri(format!("/api/v1/ingredients/{id}"))
        .header(header::AUTHORIZATION, credential())
        .header(header::IF_MATCH, "*")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"name":"Milk"}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_successful_update_returns_the_new_etag() {
    let app = app().await;
    let created = create_ingredient(&app, "Whole Milk").await;
    let id = created["id"].as_str().unwrap();

    let (status, body, headers) = send(
        &app,
        Call::new("PATCH", format!("/api/v1/ingredients/{id}"))
            .if_match(1)
            .body(json!({"default_unit": "l"})),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["revision"], 2);
    assert_eq!(body["default_unit"], "l");
    assert_eq!(etag(&headers), "2");
}

#[tokio::test]
async fn validation_failures_list_the_offending_fields() {
    let app = app().await;
    let (status, body, _) = send(
        &app,
        Call::new("POST", "/api/v1/ingredients").body(json!({"name": "   ", "default_unit": "g"})),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "name");
    assert_eq!(body["errors"][0]["message"], "Required");
}

#[tokio::test]
async fn a_duplicate_name_conflicts() {
    let app = app().await;
    create_ingredient(&app, "Whole Milk").await;
    let (status, body, _) = send(
        &app,
        Call::new("POST", "/api/v1/ingredients")
            .body(json!({"name": "whole milk", "default_unit": "ml"})),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["title"], "Already exists");
}

#[tokio::test]
async fn a_missing_ingredient_is_not_found() {
    let app = app().await;
    let id = uuid::Uuid::now_v7();
    let (status, _, _) = send(&app, Call::new("GET", format!("/api/v1/ingredients/{id}"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn archiving_removes_an_ingredient_from_the_default_listing() {
    let app = app().await;
    let created = create_ingredient(&app, "Whole Milk").await;
    let id = created["id"].as_str().unwrap();

    let (status, _, _) = send(
        &app,
        Call::new("POST", format!("/api/v1/ingredients/{id}/archive")).if_match(1),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, body, _) = send(&app, Call::new("GET", "/api/v1/ingredients")).await;
    assert_eq!(body["total"], 0);

    let (_, body, _) = send(
        &app,
        Call::new("GET", "/api/v1/ingredients?include_archived=true"),
    )
    .await;
    assert_eq!(body["total"], 1);
}

#[tokio::test]
async fn lists_are_paginated() {
    let app = app().await;
    for n in 0..7 {
        create_ingredient(&app, &format!("Ingredient {n}")).await;
    }

    let (status, body, _) = send(
        &app,
        Call::new("GET", "/api/v1/ingredients?page=2&per_page=3"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 7);
    assert_eq!(body["page"], 2);
    assert_eq!(body["total_pages"], 3);
    assert_eq!(body["items"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn searches_by_name() {
    let app = app().await;
    create_ingredient(&app, "Chicken Breast").await;
    create_ingredient(&app, "Whole Milk").await;

    let (_, body, _) = send(&app, Call::new("GET", "/api/v1/ingredients?q=chick")).await;
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["name"], "Chicken Breast");
}

#[tokio::test]
async fn maps_a_product_to_an_ingredient_and_lists_it_back() {
    let app = app().await;
    let milk = create_ingredient(&app, "Whole Milk").await;
    let milk_id = milk["id"].as_str().unwrap();

    let (status, product, _) = send(
        &app,
        Call::new("POST", "/api/v1/products").body(json!({
            "name": "Tesco Whole Milk 1L",
            "brand": "Tesco",
            "barcode": "5000119012345",
            "package_quantity": {"amount": 1.0, "unit": "l"}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{product}");
    let product_id = product["id"].as_str().unwrap();

    let (status, mapped, _) = send(
        &app,
        Call::new("PUT", format!("/api/v1/products/{product_id}/ingredient"))
            .if_match(1)
            .body(json!({"ingredient_id": milk_id})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{mapped}");
    assert_eq!(mapped["mapped_ingredient_id"], json!(milk_id));

    let (status, listed, _) = send(
        &app,
        Call::new("GET", format!("/api/v1/ingredients/{milk_id}/products")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed["total"], 1);
    assert_eq!(listed["items"][0]["name"], "Tesco Whole Milk 1L");
}

#[tokio::test]
async fn clearing_a_product_field_differs_from_omitting_it() {
    let app = app().await;
    let (_, product, _) = send(
        &app,
        Call::new("POST", "/api/v1/products").body(json!({
            "name": "Tesco Whole Milk 1L", "brand": "Tesco", "retailer": "Tesco"
        })),
    )
    .await;
    let id = product["id"].as_str().unwrap();

    let (status, updated, _) = send(
        &app,
        Call::new("PATCH", format!("/api/v1/products/{id}"))
            .if_match(1)
            .body(json!({"brand": null})),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{updated}");
    assert!(updated.get("brand").is_none(), "brand should be cleared");
    assert_eq!(updated["retailer"], "Tesco", "retailer was not mentioned");
}

#[tokio::test]
async fn a_malformed_barcode_is_rejected() {
    let app = app().await;
    let (status, body, _) = send(
        &app,
        Call::new("POST", "/api/v1/products")
            .body(json!({"name": "Odd Product", "barcode": "12AB34"})),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "barcode");
}

#[tokio::test]
async fn units_are_published_with_their_conversion_capability() {
    let app = app().await;
    let (status, body, _) = send(&app, Call::new("GET", "/api/v1/units").anonymous()).await;
    assert_eq!(status, StatusCode::OK);

    let units = body.as_array().unwrap();
    let gram = units.iter().find(|u| u["code"] == "g").unwrap();
    assert_eq!(gram["dimension"], "mass");
    assert_eq!(gram["convertible"], true);

    let clove = units.iter().find(|u| u["code"] == "clove").unwrap();
    assert_eq!(clove["dimension"], "count");
    assert_eq!(clove["convertible"], false);
}

#[tokio::test]
async fn the_openapi_document_describes_the_routes() {
    let app = app().await;
    let (status, body, _) = send(&app, Call::new("GET", "/openapi.json").anonymous()).await;
    assert_eq!(status, StatusCode::OK);

    let paths = &body["paths"];
    assert!(paths.get("/api/v1/ingredients").is_some());
    assert!(paths.get("/api/v1/ingredients/{id}").is_some());
    assert!(paths.get("/api/v1/products/{id}/ingredient").is_some());
    assert!(paths.get("/api/v1/meal-plan/{week_start}").is_some());
    assert!(
        paths
            .get("/api/v1/meal-plan-entries/{id}/components/{component_id}/eaten")
            .is_some()
    );
    assert!(paths.get("/api/v1/meal-plan/members").is_none());
    assert!(
        paths
            .get("/api/v1/meal-plan/{member_id}/{week_start}")
            .is_none()
    );
    assert!(body["components"]["securitySchemes"]["basic"].is_object());
}

#[tokio::test]
async fn the_openapi_document_has_no_dangling_references() {
    let app = app().await;
    let (_, body, _) = send(&app, Call::new("GET", "/openapi.json").anonymous()).await;

    let defined: std::collections::HashSet<String> = body["components"]["schemas"]
        .as_object()
        .expect("components.schemas")
        .keys()
        .cloned()
        .collect();

    let raw = serde_json::to_string(&body).unwrap();
    let mut missing: Vec<String> = Vec::new();
    for part in raw.split("\"#/components/schemas/").skip(1) {
        let name = part.split('"').next().unwrap_or_default().to_owned();
        if !defined.contains(&name) && !missing.contains(&name) {
            missing.push(name);
        }
    }

    assert!(
        missing.is_empty(),
        "these schemas are referenced but never defined, which breaks client codegen: {missing:?}"
    );
}

#[tokio::test]
async fn every_operation_has_a_unique_id() {
    let app = app().await;
    let (_, body, _) = send(&app, Call::new("GET", "/openapi.json").anonymous()).await;

    let mut ids: Vec<String> = Vec::new();
    for (path, methods) in body["paths"].as_object().expect("paths") {
        for (method, operation) in methods.as_object().expect("methods") {
            let id = operation["operationId"]
                .as_str()
                .unwrap_or_else(|| panic!("{method} {path} has no operationId"));
            ids.push(id.to_owned());
        }
    }

    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();

    assert_eq!(
        ids.len(),
        unique.len(),
        "duplicate operationIds are invalid OpenAPI and collapse distinct routes together \
         in every generated client"
    );
}

async fn create_member(app: &Router, name: &str) -> Value {
    let (status, body, _) = send(
        app,
        Call::new("POST", "/api/v1/members").body(json!({"display_name": name})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body
}

async fn create_user(app: &Router, username: &str, roles: &[&str]) -> Value {
    let (status, body, _) = send(
        app,
        Call::new("POST", "/api/v1/users").body(json!({"username": username, "roles": roles})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body
}

#[tokio::test]
async fn the_bootstrap_admin_can_sign_in_and_has_a_member() {
    let app = app().await;
    let (status, body, _) = send(&app, Call::new("GET", "/api/v1/auth/me")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["username"], "admin");
    assert!(
        body["member_id"].is_string(),
        "the bootstrap admin should be linked to a member: {body}"
    );
    assert_eq!(body["roles"][0], "admin");
}

#[tokio::test]
async fn an_unknown_account_is_rejected() {
    let app = app().await;
    let (status, _, _) = send(
        &app,
        Call::new("GET", "/api/v1/members").signed_in_as("nobody"),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_member_needs_no_account() {
    let app = app().await;
    let member = create_member(&app, "Joe").await;

    assert_eq!(member["display_name"], "Joe");
    assert_eq!(member["has_account"], false);
    assert!(member["linked_user_id"].is_null());
}

#[tokio::test]
async fn duplicate_member_names_conflict() {
    let app = app().await;
    create_member(&app, "Joe").await;

    let (status, _, _) = send(
        &app,
        Call::new("POST", "/api/v1/members").body(json!({"display_name": "joe"})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn a_blank_member_name_is_unprocessable() {
    let app = app().await;
    let (status, body, _) = send(
        &app,
        Call::new("POST", "/api/v1/members").body(json!({"display_name": "  "})),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "display_name");
}

#[tokio::test]
async fn updating_a_member_needs_if_match() {
    let app = app().await;
    let member = create_member(&app, "Joe").await;

    let (status, _, _) = send(
        &app,
        Call::new(
            "PATCH",
            format!("/api/v1/members/{}", member["id"].as_str().unwrap()),
        )
        .body(json!({"display_name": "Joseph"})),
    )
    .await;
    assert_eq!(status, StatusCode::PRECONDITION_REQUIRED);
}

#[tokio::test]
async fn a_stale_member_revision_conflicts() {
    let app = app().await;
    let member = create_member(&app, "Joe").await;
    let id = member["id"].as_str().unwrap();

    let (status, _, _) = send(
        &app,
        Call::new("PATCH", format!("/api/v1/members/{id}"))
            .if_match(99)
            .body(json!({"display_name": "Joseph"})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn a_member_round_trips_through_archive() {
    let app = app().await;
    let member = create_member(&app, "Joe").await;
    let id = member["id"].as_str().unwrap();

    let (status, archived, _) = send(
        &app,
        Call::new("POST", format!("/api/v1/members/{id}/archive"))
            .if_match(member["revision"].as_i64().unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(archived["archived_at"].is_string());

    let (status, restored, _) = send(
        &app,
        Call::new("POST", format!("/api/v1/members/{id}/unarchive"))
            .if_match(archived["revision"].as_i64().unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(restored["archived_at"].is_null());
}

#[tokio::test]
async fn linking_an_account_keeps_the_same_member() {
    let app = app().await;
    let member = create_member(&app, "Joe").await;
    let user = create_user(&app, "joe", &["basic_user"]).await;
    let id = member["id"].as_str().unwrap();

    let (status, linked, _) = send(
        &app,
        Call::new("PUT", format!("/api/v1/members/{id}/account"))
            .if_match(member["revision"].as_i64().unwrap())
            .body(json!({"user_id": user["id"]})),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        linked["id"], member["id"],
        "USR-013: the member is not replaced"
    );
    assert_eq!(linked["created_at"], member["created_at"]);
    assert_eq!(linked["has_account"], true);
}

#[tokio::test]
async fn one_account_cannot_serve_two_members() {
    let app = app().await;
    let first = create_member(&app, "Joe").await;
    let second = create_member(&app, "Jo").await;
    let user = create_user(&app, "joe", &["basic_user"]).await;

    send(
        &app,
        Call::new(
            "PUT",
            format!("/api/v1/members/{}/account", first["id"].as_str().unwrap()),
        )
        .if_match(first["revision"].as_i64().unwrap())
        .body(json!({"user_id": user["id"]})),
    )
    .await;

    let (status, _, _) = send(
        &app,
        Call::new(
            "PUT",
            format!("/api/v1/members/{}/account", second["id"].as_str().unwrap()),
        )
        .if_match(second["revision"].as_i64().unwrap())
        .body(json!({"user_id": user["id"]})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn unlinking_an_account_leaves_the_member() {
    let app = app().await;
    let member = create_member(&app, "Joe").await;
    let user = create_user(&app, "joe", &["basic_user"]).await;
    let id = member["id"].as_str().unwrap();

    let (_, linked, _) = send(
        &app,
        Call::new("PUT", format!("/api/v1/members/{id}/account"))
            .if_match(member["revision"].as_i64().unwrap())
            .body(json!({"user_id": user["id"]})),
    )
    .await;

    let (status, unlinked, _) = send(
        &app,
        Call::new("DELETE", format!("/api/v1/members/{id}/account"))
            .if_match(linked["revision"].as_i64().unwrap()),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(unlinked["id"], member["id"]);
    assert_eq!(unlinked["has_account"], false);
}

#[tokio::test]
async fn a_new_account_signs_in_with_the_dev_password() {
    let app = app().await;
    create_user(&app, "joe", &["basic_user"]).await;

    let (status, body, _) = send(
        &app,
        Call::new("GET", "/api/v1/auth/me").signed_in_as("joe"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["username"], "joe");
    assert_eq!(body["roles"][0], "basic_user");
}

#[tokio::test]
async fn a_basic_user_cannot_reach_the_accounts_api() {
    let app = app().await;
    create_user(&app, "joe", &["basic_user"]).await;

    let (members, _, _) = send(
        &app,
        Call::new("GET", "/api/v1/members").signed_in_as("joe"),
    )
    .await;
    let (users, body, _) = send(&app, Call::new("GET", "/api/v1/users").signed_in_as("joe")).await;

    assert_eq!(
        members,
        StatusCode::OK,
        "SEC-004: coordination stays visible"
    );
    assert_eq!(
        users,
        StatusCode::FORBIDDEN,
        "SEC-004: accounts are admin only"
    );
    assert_eq!(body["status"], 403);
}

#[tokio::test]
async fn a_basic_user_cannot_manage_members() {
    let app = app().await;
    create_user(&app, "joe", &["basic_user"]).await;

    let (status, _, _) = send(
        &app,
        Call::new("POST", "/api/v1/members")
            .signed_in_as("joe")
            .body(json!({"display_name": "Ann"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_basic_user_can_rename_their_own_linked_member() {
    let app = app().await;
    let member = create_member(&app, "Joe").await;
    let user = create_user(&app, "joe", &["basic_user"]).await;
    send(
        &app,
        Call::new(
            "PUT",
            format!("/api/v1/members/{}/account", member["id"].as_str().unwrap()),
        )
        .if_match(member["revision"].as_i64().unwrap())
        .body(json!({"user_id": user["id"]})),
    )
    .await;

    let (status, updated, _) = send(
        &app,
        Call::new(
            "PATCH",
            format!("/api/v1/members/{}", member["id"].as_str().unwrap()),
        )
        .signed_in_as("joe")
        .if_match(member["revision"].as_i64().unwrap() + 1)
        .body(json!({"display_name": "Joseph"})),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["display_name"], "Joseph");
}

#[tokio::test]
async fn a_basic_user_cannot_rename_someone_elses_member() {
    let app = app().await;
    let joe = create_member(&app, "Joe").await;
    let ann = create_member(&app, "Ann").await;
    let user = create_user(&app, "joe", &["basic_user"]).await;
    send(
        &app,
        Call::new(
            "PUT",
            format!("/api/v1/members/{}/account", joe["id"].as_str().unwrap()),
        )
        .if_match(joe["revision"].as_i64().unwrap())
        .body(json!({"user_id": user["id"]})),
    )
    .await;

    let (status, _, _) = send(
        &app,
        Call::new(
            "PATCH",
            format!("/api/v1/members/{}", ann["id"].as_str().unwrap()),
        )
        .signed_in_as("joe")
        .if_match(ann["revision"].as_i64().unwrap())
        .body(json!({"display_name": "Anna"})),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_household_manager_manages_members_but_not_accounts() {
    let app = app().await;
    create_user(&app, "manager", &["household_manager"]).await;

    let (created, _, _) = send(
        &app,
        Call::new("POST", "/api/v1/members")
            .signed_in_as("manager")
            .body(json!({"display_name": "Ann"})),
    )
    .await;
    let (accounts, _, _) = send(
        &app,
        Call::new("GET", "/api/v1/users").signed_in_as("manager"),
    )
    .await;

    assert_eq!(created, StatusCode::CREATED);
    assert_eq!(accounts, StatusCode::FORBIDDEN, "USR-005 and DEC-005");
}

#[tokio::test]
async fn an_archived_account_can_no_longer_sign_in() {
    let app = app().await;
    let user = create_user(&app, "joe", &["basic_user"]).await;

    let (status, _, _) = send(
        &app,
        Call::new(
            "POST",
            format!("/api/v1/users/{}/archive", user["id"].as_str().unwrap()),
        )
        .if_match(user["revision"].as_i64().unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _, _) = send(
        &app,
        Call::new("GET", "/api/v1/members").signed_in_as("joe"),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn the_last_admin_cannot_be_demoted() {
    let app = app().await;
    let (_, me, _) = send(&app, Call::new("GET", "/api/v1/auth/me")).await;
    let id = me["user_id"].as_str().unwrap();
    let (_, admin, _) = send(&app, Call::new("GET", format!("/api/v1/users/{id}"))).await;

    let (status, body, _) = send(
        &app,
        Call::new("PUT", format!("/api/v1/users/{id}/roles"))
            .if_match(admin["revision"].as_i64().unwrap())
            .body(json!({"roles": ["basic_user"]})),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "roles");
}

#[tokio::test]
async fn setting_roles_replaces_them_wholesale() {
    let app = app().await;
    let user = create_user(&app, "joe", &["basic_user"]).await;
    let id = user["id"].as_str().unwrap();

    let (status, updated, _) = send(
        &app,
        Call::new("PUT", format!("/api/v1/users/{id}/roles"))
            .if_match(user["revision"].as_i64().unwrap())
            .body(json!({"roles": ["household_manager"]})),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["roles"], json!(["household_manager"]));
    assert!(
        updated["permissions"]
            .as_array()
            .unwrap()
            .contains(&json!("household:write"))
    );
}

#[tokio::test]
async fn a_duplicate_username_conflicts() {
    let app = app().await;
    create_user(&app, "joe", &["basic_user"]).await;

    let (status, _, _) = send(
        &app,
        Call::new("POST", "/api/v1/users")
            .body(json!({"username": "JOE", "roles": ["basic_user"]})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn a_username_with_spaces_is_unprocessable() {
    let app = app().await;
    let (status, body, _) = send(
        &app,
        Call::new("POST", "/api/v1/users")
            .body(json!({"username": "joe bloggs", "roles": ["basic_user"]})),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "username");
}

#[tokio::test]
async fn an_account_without_a_role_is_unprocessable() {
    let app = app().await;
    let (status, _, _) = send(
        &app,
        Call::new("POST", "/api/v1/users").body(json!({"username": "joe", "roles": []})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn access_grants_round_trip() {
    let app = app().await;
    let member = create_member(&app, "Joe").await;
    let user = create_user(&app, "nutritionist", &["nutritionist"]).await;
    let member_id = member["id"].as_str().unwrap();
    let user_id = user["id"].as_str().unwrap();

    let (status, _, _) = send(
        &app,
        Call::new("PUT", format!("/api/v1/members/{member_id}/access"))
            .body(json!({"user_id": user_id, "scope": "health_data"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, grants, _) = send(
        &app,
        Call::new("GET", format!("/api/v1/members/{member_id}/access")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(grants.as_array().unwrap().len(), 1);
    assert_eq!(grants[0]["scope"], "health_data");

    let (status, _, _) = send(
        &app,
        Call::new(
            "DELETE",
            format!("/api/v1/members/{member_id}/access/{user_id}/health_data"),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, grants, _) = send(
        &app,
        Call::new("GET", format!("/api/v1/members/{member_id}/access")),
    )
    .await;
    assert!(grants.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn a_household_manager_cannot_read_access_grants() {
    let app = app().await;
    let member = create_member(&app, "Joe").await;
    create_user(&app, "manager", &["household_manager"]).await;

    let (status, _, _) = send(
        &app,
        Call::new(
            "GET",
            format!("/api/v1/members/{}/access", member["id"].as_str().unwrap()),
        )
        .signed_in_as("manager"),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "USR-006");
}

#[tokio::test]
async fn members_can_be_filtered_by_account() {
    let app = app().await;
    let member = create_member(&app, "Joe").await;
    let user = create_user(&app, "joe", &["basic_user"]).await;

    send(
        &app,
        Call::new(
            "PUT",
            format!("/api/v1/members/{}/account", member["id"].as_str().unwrap()),
        )
        .if_match(member["revision"].as_i64().unwrap())
        .body(json!({"user_id": user["id"]})),
    )
    .await;

    let (_, with, _) = send(&app, Call::new("GET", "/api/v1/members?with_account=true")).await;
    let (_, without, _) = send(&app, Call::new("GET", "/api/v1/members?with_account=false")).await;

    let named: Vec<&str> = with["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["display_name"].as_str().unwrap())
        .collect();
    assert_eq!(
        named,
        vec!["admin", "Joe"],
        "the bootstrap admin has one too"
    );
    assert!(without["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn a_missing_member_is_not_found() {
    let app = app().await;
    let (status, _, _) = send(
        &app,
        Call::new("GET", format!("/api/v1/members/{}", uuid::Uuid::now_v7())),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

async fn create_milk_product(app: &Router) -> Value {
    let (status, body, _) = send(
        app,
        Call::new("POST", "/api/v1/products").body(json!({
            "name": "Tesco Whole Milk 1L",
            "package_quantity": {"amount": 650.0, "unit": "g"},
            "nutrition": {"basis": {"amount": 100.0, "unit": "g"}, "energy_kcal": 64.0}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body
}

fn measured_amount(grams: f64) -> serde_json::Value {
    json!({"kind": "measure", "value": grams, "unit": "g"})
}

#[tokio::test]
async fn creates_a_consumption_record_with_scaled_nutrition() {
    let app = app().await;
    let me = send(&app, Call::new("GET", "/api/v1/auth/me")).await.1;
    let product = create_milk_product(&app).await;

    let (status, body, headers) = send(
        &app,
        Call::new("POST", "/api/v1/consumption").body(json!({
            "member_id": me["member_id"],
            "product_id": product["id"],
            "slot": "breakfast",
            "amount": measured_amount(150.0),
            "consumed_on": "2026-08-22",
        })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["revision"], 1);
    assert_eq!(body["slot"], "breakfast");
    assert_eq!(body["consumed_at"], Value::Null);
    assert_eq!(body["nutrition"]["energy_kcal"], json!(96.0));
    assert_eq!(etag(&headers), "1");
}

#[tokio::test]
async fn a_basic_user_can_log_against_their_own_linked_member() {
    let app = app().await;
    let member = create_member(&app, "Joe").await;
    let user = create_user(&app, "joe", &["basic_user"]).await;
    send(
        &app,
        Call::new(
            "PUT",
            format!("/api/v1/members/{}/account", member["id"].as_str().unwrap()),
        )
        .if_match(member["revision"].as_i64().unwrap())
        .body(json!({"user_id": user["id"]})),
    )
    .await;
    let product = create_milk_product(&app).await;

    let (status, body, _) = send(
        &app,
        Call::new("POST", "/api/v1/consumption")
            .signed_in_as("joe")
            .body(json!({
                "member_id": member["id"],
                "product_id": product["id"],
                "slot": "breakfast",
                "amount": measured_amount(150.0),
                "consumed_on": "2026-08-22",
            })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
}

#[tokio::test]
async fn a_basic_user_cannot_log_against_someone_elses_member() {
    let app = app().await;
    let member = create_member(&app, "Ann").await;
    create_user(&app, "joe", &["basic_user"]).await;
    let product = create_milk_product(&app).await;

    let (status, _, _) = send(
        &app,
        Call::new("POST", "/api/v1/consumption")
            .signed_in_as("joe")
            .body(json!({
                "member_id": member["id"],
                "product_id": product["id"],
                "slot": "breakfast",
                "amount": measured_amount(150.0),
                "consumed_on": "2026-08-22",
            })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "USR-006");
}

#[tokio::test]
async fn a_household_manager_cannot_log_on_behalf_of_a_member_without_a_grant() {
    let app = app().await;
    let member = create_member(&app, "Ann").await;
    create_user(&app, "manager", &["household_manager"]).await;
    let product = create_milk_product(&app).await;

    let (status, _, _) = send(
        &app,
        Call::new("POST", "/api/v1/consumption")
            .signed_in_as("manager")
            .body(json!({
                "member_id": member["id"],
                "product_id": product["id"],
                "slot": "breakfast",
                "amount": measured_amount(150.0),
                "consumed_on": "2026-08-22",
            })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "USR-005: managing the household is not health access"
    );
}

#[tokio::test]
async fn an_explicit_health_data_grant_allows_logging() {
    let app = app().await;
    let member = create_member(&app, "Ann").await;
    let user = create_user(&app, "nutritionist", &["nutritionist"]).await;
    send(
        &app,
        Call::new(
            "PUT",
            format!("/api/v1/members/{}/access", member["id"].as_str().unwrap()),
        )
        .body(json!({"user_id": user["id"], "scope": "health_data"})),
    )
    .await;
    let product = create_milk_product(&app).await;

    let (status, body, _) = send(
        &app,
        Call::new("POST", "/api/v1/consumption")
            .signed_in_as("nutritionist")
            .body(json!({
                "member_id": member["id"],
                "product_id": product["id"],
                "slot": "breakfast",
                "amount": measured_amount(150.0),
                "consumed_on": "2026-08-22",
            })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
}

#[tokio::test]
async fn logging_against_an_archived_product_is_rejected() {
    let app = app().await;
    let me = send(&app, Call::new("GET", "/api/v1/auth/me")).await.1;
    let product = create_milk_product(&app).await;
    send(
        &app,
        Call::new(
            "POST",
            format!(
                "/api/v1/products/{}/archive",
                product["id"].as_str().unwrap()
            ),
        )
        .if_match(product["revision"].as_i64().unwrap()),
    )
    .await;

    let (status, body, _) = send(
        &app,
        Call::new("POST", "/api/v1/consumption").body(json!({
            "member_id": me["member_id"],
            "product_id": product["id"],
            "slot": "breakfast",
            "amount": measured_amount(150.0),
            "consumed_on": "2026-08-22",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
}

#[tokio::test]
async fn an_amount_the_product_cannot_resolve_is_a_validation_error() {
    let app = app().await;
    let me = send(&app, Call::new("GET", "/api/v1/auth/me")).await.1;
    let product = create_milk_product(&app).await;

    let (status, body, _) = send(
        &app,
        Call::new("POST", "/api/v1/consumption").body(json!({
            "member_id": me["member_id"],
            "product_id": product["id"],
            "slot": "breakfast",
            "amount": {"kind": "servings", "value": 1.0},
            "consumed_on": "2026-08-22",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["errors"][0]["field"], "amount");
}

#[tokio::test]
async fn updating_a_consumption_record_needs_if_match() {
    let app = app().await;
    let me = send(&app, Call::new("GET", "/api/v1/auth/me")).await.1;
    let product = create_milk_product(&app).await;
    let (_, created, _) = send(
        &app,
        Call::new("POST", "/api/v1/consumption").body(json!({
            "member_id": me["member_id"],
            "product_id": product["id"],
            "slot": "breakfast",
            "amount": measured_amount(150.0),
            "consumed_on": "2026-08-22",
        })),
    )
    .await;

    let (status, _, _) = send(
        &app,
        Call::new(
            "PATCH",
            format!("/api/v1/consumption/{}", created["id"].as_str().unwrap()),
        )
        .body(json!({"amount": measured_amount(300.0)})),
    )
    .await;
    assert_eq!(status, StatusCode::PRECONDITION_REQUIRED);
}

#[tokio::test]
async fn amending_the_amount_rescales_the_nutrition() {
    let app = app().await;
    let me = send(&app, Call::new("GET", "/api/v1/auth/me")).await.1;
    let product = create_milk_product(&app).await;
    let (_, created, _) = send(
        &app,
        Call::new("POST", "/api/v1/consumption").body(json!({
            "member_id": me["member_id"],
            "product_id": product["id"],
            "slot": "breakfast",
            "amount": measured_amount(150.0),
            "consumed_on": "2026-08-22",
        })),
    )
    .await;
    let id = created["id"].as_str().unwrap();

    let (status, updated, _) = send(
        &app,
        Call::new("PATCH", format!("/api/v1/consumption/{id}"))
            .if_match(1)
            .body(json!({"amount": measured_amount(300.0), "slot": "lunch"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["slot"], "lunch");
    assert_eq!(updated["nutrition"]["energy_kcal"], json!(192.0));
}

#[tokio::test]
async fn a_stale_consumption_revision_conflicts() {
    let app = app().await;
    let me = send(&app, Call::new("GET", "/api/v1/auth/me")).await.1;
    let product = create_milk_product(&app).await;
    let (_, created, _) = send(
        &app,
        Call::new("POST", "/api/v1/consumption").body(json!({
            "member_id": me["member_id"],
            "product_id": product["id"],
            "slot": "breakfast",
            "amount": measured_amount(150.0),
            "consumed_on": "2026-08-22",
        })),
    )
    .await;
    let id = created["id"].as_str().unwrap();

    let (status, _, _) = send(
        &app,
        Call::new("PATCH", format!("/api/v1/consumption/{id}"))
            .if_match(99)
            .body(json!({"amount": measured_amount(300.0)})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn deleting_a_consumption_record_removes_it() {
    let app = app().await;
    let me = send(&app, Call::new("GET", "/api/v1/auth/me")).await.1;
    let product = create_milk_product(&app).await;
    let (_, created, _) = send(
        &app,
        Call::new("POST", "/api/v1/consumption").body(json!({
            "member_id": me["member_id"],
            "product_id": product["id"],
            "slot": "breakfast",
            "amount": measured_amount(150.0),
            "consumed_on": "2026-08-22",
        })),
    )
    .await;
    let id = created["id"].as_str().unwrap();

    let (status, _, _) = send(
        &app,
        Call::new("DELETE", format!("/api/v1/consumption/{id}")).if_match(1),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _, _) = send(&app, Call::new("GET", format!("/api/v1/consumption/{id}"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_diary_day_endpoint_returns_entries_and_totals() {
    let app = app().await;
    let me = send(&app, Call::new("GET", "/api/v1/auth/me")).await.1;
    let member_id = me["member_id"].as_str().unwrap();
    let product = create_milk_product(&app).await;

    for _ in 0..2 {
        send(
            &app,
            Call::new("POST", "/api/v1/consumption").body(json!({
                "member_id": member_id,
                "product_id": product["id"],
                "slot": "breakfast",
                "amount": measured_amount(150.0),
                "consumed_on": "2026-08-22",
            })),
        )
        .await;
    }

    let (status, day, _) = send(
        &app,
        Call::new("GET", format!("/api/v1/diary/{member_id}/2026-08-22")),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{day}");
    assert_eq!(day["entries"].as_array().unwrap().len(), 2);
    assert_eq!(day["entries"][0]["product_name"], "Tesco Whole Milk 1L");
    assert_eq!(day["totals"]["entry_count"], 2);
    assert_eq!(day["totals"]["nutrition"]["energy_kcal"], json!(192.0));
}

#[tokio::test]
async fn the_product_nutrition_preview_scales_the_amount() {
    let app = app().await;
    let product = create_milk_product(&app).await;
    let id = product["id"].as_str().unwrap();

    let (status, body, _) = send(
        &app,
        Call::new(
            "GET",
            format!("/api/v1/products/{id}/nutrition?kind=measure&value=150&unit=g"),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["nutrition"]["energy_kcal"], json!(96.0));
    assert_eq!(body["quality"], "partial");
}

#[tokio::test]
async fn a_meal_plan_entry_round_trips_through_the_week() {
    let app = app().await;
    let me = send(&app, Call::new("GET", "/api/v1/auth/me")).await.1;
    let member_id = me["member_id"].as_str().unwrap();
    let product = create_milk_product(&app).await;

    let (status, entry, headers) = send(
        &app,
        Call::new("POST", "/api/v1/meal-plan-entries").body(json!({
            "planned_on": "2026-08-25",
            "planned_time": "18:30",
            "slot": "dinner",
            "components": [
                {"product_id": product["id"], "amount": measured_amount(150.0)},
                {"product_id": product["id"], "amount": measured_amount(50.0)}
            ]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{entry}");
    assert_eq!(etag(&headers), "1");
    assert_eq!(entry["member_id"], member_id);
    assert_eq!(entry["status"], "planned");
    assert_eq!(entry["components"].as_array().unwrap().len(), 2);
    assert_eq!(entry["planned"]["nutrition"]["energy_kcal"], json!(128.0));

    let (status, week, _) = send(&app, Call::new("GET", "/api/v1/meal-plan/2026-08-24")).await;
    assert_eq!(status, StatusCode::OK, "{week}");
    assert_eq!(week["days"].as_array().unwrap().len(), 7);
    assert_eq!(week["days"][1]["entries"][0]["id"], entry["id"]);
    assert_eq!(
        week["remaining_planned"]["nutrition"]["energy_kcal"],
        json!(128.0)
    );
    assert_eq!(week["projected"]["nutrition"]["energy_kcal"], json!(128.0));
}

#[tokio::test]
async fn household_meal_times_are_published_with_defaults_and_an_etag() {
    let app = app().await;
    let (status, body, headers) =
        send(&app, Call::new("GET", "/api/v1/household/meal-times")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["breakfast"], "08:00");
    assert_eq!(body["lunch"], "12:30");
    assert_eq!(body["dinner"], "18:00");
    assert_eq!(etag(&headers), "1");
}

#[tokio::test]
async fn a_household_manager_can_change_a_meal_time() {
    let app = app().await;
    let (status, body, _) = send(
        &app,
        Call::new("PUT", "/api/v1/household/meal-times")
            .if_match(1)
            .body(json!({ "lunch": "13:15" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["lunch"], "13:15");
    assert_eq!(body["breakfast"], "08:00");
    assert_eq!(body["revision"], json!(2));

    let (_, reread, _) = send(&app, Call::new("GET", "/api/v1/household/meal-times")).await;
    assert_eq!(reread["lunch"], "13:15");
}

#[tokio::test]
async fn a_stale_meal_times_revision_conflicts() {
    let app = app().await;
    let (status, _, _) = send(
        &app,
        Call::new("PUT", "/api/v1/household/meal-times")
            .if_match(9)
            .body(json!({ "dinner": "19:00" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn a_basic_user_cannot_change_meal_times() {
    let app = app().await;
    create_user(&app, "sam", &["basic_user"]).await;
    let (status, _, _) = send(
        &app,
        Call::new("PUT", "/api/v1/household/meal-times")
            .signed_in_as("sam")
            .if_match(1)
            .body(json!({ "dinner": "19:00" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn meal_times_default_for_timed_slots_but_not_snacks() {
    let app = app().await;
    let product = create_milk_product(&app).await;

    let (status, dinner, _) = send(
        &app,
        Call::new("POST", "/api/v1/meal-plan-entries").body(json!({
            "planned_on": "2026-08-27",
            "slot": "dinner",
            "components": [{"product_id": product["id"], "amount": measured_amount(150.0)}]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{dinner}");
    assert_eq!(dinner["planned_time"], "18:00");

    let (_, snack, _) = send(
        &app,
        Call::new("POST", "/api/v1/meal-plan-entries").body(json!({
            "planned_on": "2026-08-27",
            "planned_time": "20:45",
            "slot": "snacks",
            "components": [{"product_id": product["id"], "amount": measured_amount(20.0)}]
        })),
    )
    .await;
    assert_eq!(snack["planned_time"], Value::Null);
}

#[tokio::test]
async fn an_explicit_planned_time_is_kept_over_the_default() {
    let app = app().await;
    let product = create_milk_product(&app).await;
    let (_, entry, _) = send(
        &app,
        Call::new("POST", "/api/v1/meal-plan-entries").body(json!({
            "planned_on": "2026-08-27",
            "planned_time": "20:45",
            "slot": "dinner",
            "components": [{"product_id": product["id"], "amount": measured_amount(150.0)}]
        })),
    )
    .await;
    assert_eq!(entry["planned_time"], "20:45");
}

#[tokio::test]
async fn confirming_one_component_leaves_the_rest_of_the_meal_pending() {
    let app = app().await;
    let product = create_milk_product(&app).await;
    let entry = send(
        &app,
        Call::new("POST", "/api/v1/meal-plan-entries").body(json!({
            "planned_on": "2026-08-25",
            "planned_time": "08:00",
            "slot": "breakfast",
            "components": [
                {"product_id": product["id"], "amount": measured_amount(80.0)},
                {"product_id": product["id"], "amount": measured_amount(250.0)},
                {"product_id": product["id"], "amount": measured_amount(100.0)}
            ]
        })),
    )
    .await
    .1;
    let entry_id = entry["id"].as_str().unwrap();
    let component_id = entry["components"][2]["id"].as_str().unwrap();

    let (status, updated, _) = send(
        &app,
        Call::new(
            "POST",
            format!("/api/v1/meal-plan-entries/{entry_id}/components/{component_id}/eaten"),
        )
        .if_match(1)
        .body(json!({
            "consumed_on": "2026-08-25",
            "consumed_at": "2026-08-25T08:00:00Z",
            "amount": measured_amount(100.0)
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["status"], "partially_resolved");
    assert_eq!(updated["components"][0]["status"], "planned");
    assert_eq!(updated["components"][1]["status"], "planned");
    assert_eq!(updated["components"][2]["status"], "eaten");
    assert!(updated["components"][0]["consumption_record"].is_null());
    assert!(updated["components"][1]["consumption_record"].is_null());
    assert!(updated["components"][2]["consumption_record"].is_object());
}

#[tokio::test]
async fn the_week_slots_projection_flattens_planned_and_logged_food_together() {
    let app = app().await;
    let member_id = send(&app, Call::new("GET", "/api/v1/auth/me")).await.1["member_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let product = create_milk_product(&app).await;

    let (status, entry, _) = send(
        &app,
        Call::new("POST", "/api/v1/meal-plan-entries").body(json!({
            "planned_on": "2026-08-25",
            "planned_time": "08:00",
            "slot": "breakfast",
            "components": [{"product_id": product["id"], "amount": measured_amount(150.0)}]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{entry}");

    let (status, logged, _) = send(
        &app,
        Call::new("POST", "/api/v1/consumption").body(json!({
            "member_id": member_id,
            "product_id": product["id"],
            "slot": "breakfast",
            "amount": measured_amount(50.0),
            "consumed_on": "2026-08-25",
            "consumed_at": "2026-08-25T08:15:00Z",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{logged}");

    let (status, week, _) = send(&app, Call::new("GET", "/api/v1/meal-plan/2026-08-24")).await;
    assert_eq!(status, StatusCode::OK, "{week}");
    let breakfast = &week["days"][1]["slots"][0];
    assert_eq!(breakfast["slot"], "breakfast");
    let items = breakfast["items"].as_array().unwrap();
    assert_eq!(items.len(), 2, "{items:?}");
    assert!(
        items
            .iter()
            .any(|item| item["kind"] == "planned" && item["status"] == "planned")
    );
    let logged_item = items.iter().find(|item| item["kind"] == "logged").unwrap();
    assert_eq!(logged_item["consumed_at"], "2026-08-25T08:15:00Z");
    assert!(logged_item["at"].is_null());
    assert!(
        items
            .iter()
            .any(|item| item["kind"] == "logged" && item["status"] == "eaten")
    );
}

#[tokio::test]
async fn confirming_a_planned_meal_creates_locked_diary_records() {
    let app = app().await;
    let me = send(&app, Call::new("GET", "/api/v1/auth/me")).await.1;
    let member_id = me["member_id"].as_str().unwrap();
    let product = create_milk_product(&app).await;
    let entry = send(
        &app,
        Call::new("POST", "/api/v1/meal-plan-entries").body(json!({
            "planned_on": "2026-08-25",
            "slot": "lunch",
            "components": [{"product_id": product["id"], "amount": measured_amount(150.0)}]
        })),
    )
    .await
    .1;
    let entry_id = entry["id"].as_str().unwrap();
    let component_id = entry["components"][0]["id"].as_str().unwrap();

    let (status, eaten, _) = send(
        &app,
        Call::new(
            "POST",
            format!("/api/v1/meal-plan-entries/{entry_id}/eaten"),
        )
        .if_match(1)
        .body(json!({
            "consumed_on": "2026-08-26",
            "consumed_at": "2026-08-26T19:15:00Z",
            "components": [{"component_id": component_id, "amount": measured_amount(200.0)}]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{eaten}");
    assert_eq!(eaten["status"], "eaten");
    assert_eq!(eaten["actual"]["nutrition"]["energy_kcal"], json!(128.0));
    let record = &eaten["components"][0]["consumption_record"];
    assert_eq!(record["meal_plan_entry_id"], entry["id"]);
    assert_eq!(
        record["meal_plan_component_id"],
        entry["components"][0]["id"]
    );

    let (status, diary, _) = send(
        &app,
        Call::new("GET", format!("/api/v1/diary/{member_id}/2026-08-26")),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{diary}");
    assert_eq!(diary["entries"].as_array().unwrap().len(), 1);

    let record_id = record["id"].as_str().unwrap();
    let (status, _, _) = send(
        &app,
        Call::new("DELETE", format!("/api/v1/consumption/{record_id}"))
            .if_match(record["revision"].as_i64().unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn reopening_a_confirmed_meal_removes_the_diary_entries_and_allows_reconfirmation() {
    let app = app().await;
    let me = send(&app, Call::new("GET", "/api/v1/auth/me")).await.1;
    let member_id = me["member_id"].as_str().unwrap();
    let product = create_milk_product(&app).await;
    let entry = send(
        &app,
        Call::new("POST", "/api/v1/meal-plan-entries").body(json!({
            "planned_on": "2026-08-25",
            "slot": "lunch",
            "components": [{"product_id": product["id"], "amount": measured_amount(150.0)}]
        })),
    )
    .await
    .1;
    let entry_id = entry["id"].as_str().unwrap();
    let component_id = entry["components"][0]["id"].as_str().unwrap();

    send(
        &app,
        Call::new(
            "POST",
            format!("/api/v1/meal-plan-entries/{entry_id}/eaten"),
        )
        .if_match(1)
        .body(json!({
            "consumed_on": "2026-08-26",
            "consumed_at": "2026-08-26T19:15:00Z",
            "components": [{"component_id": component_id, "amount": measured_amount(200.0)}]
        })),
    )
    .await;

    let (status, reopened, _) = send(
        &app,
        Call::new(
            "POST",
            format!("/api/v1/meal-plan-entries/{entry_id}/reopen"),
        )
        .if_match(2),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{reopened}");
    assert_eq!(reopened["status"], "planned");

    let (status, diary, _) = send(
        &app,
        Call::new("GET", format!("/api/v1/diary/{member_id}/2026-08-26")),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{diary}");
    assert_eq!(diary["entries"].as_array().unwrap().len(), 0);

    let component_id = reopened["components"][0]["id"].as_str().unwrap();
    let (status, reconfirmed, _) = send(
        &app,
        Call::new(
            "POST",
            format!("/api/v1/meal-plan-entries/{entry_id}/eaten"),
        )
        .if_match(3)
        .body(json!({
            "consumed_on": "2026-08-26",
            "consumed_at": "2026-08-26T19:15:00Z",
            "components": [{"component_id": component_id, "amount": measured_amount(150.0)}]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{reconfirmed}");
    assert_eq!(reconfirmed["status"], "eaten");
}

#[tokio::test]
async fn a_planned_meal_cannot_be_reopened() {
    let app = app().await;
    let product = create_milk_product(&app).await;
    let entry = send(
        &app,
        Call::new("POST", "/api/v1/meal-plan-entries").body(json!({
            "planned_on": "2026-08-25",
            "slot": "lunch",
            "components": [{"product_id": product["id"], "amount": measured_amount(150.0)}]
        })),
    )
    .await
    .1;
    let entry_id = entry["id"].as_str().unwrap();

    let (status, body, _) = send(
        &app,
        Call::new(
            "POST",
            format!("/api/v1/meal-plan-entries/{entry_id}/reopen"),
        )
        .if_match(1),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
}

#[tokio::test]
async fn another_members_meal_cannot_be_reopened() {
    let app = app().await;
    let product = create_milk_product(&app).await;
    let entry = send(
        &app,
        Call::new("POST", "/api/v1/meal-plan-entries").body(json!({
            "planned_on": "2026-08-25",
            "slot": "lunch",
            "components": [{"product_id": product["id"], "amount": measured_amount(150.0)}]
        })),
    )
    .await
    .1;
    let entry_id = entry["id"].as_str().unwrap();
    let component_id = entry["components"][0]["id"].as_str().unwrap();
    send(
        &app,
        Call::new(
            "POST",
            format!("/api/v1/meal-plan-entries/{entry_id}/eaten"),
        )
        .if_match(1)
        .body(json!({
            "consumed_on": "2026-08-26",
            "consumed_at": "2026-08-26T19:15:00Z",
            "components": [{"component_id": component_id, "amount": measured_amount(150.0)}]
        })),
    )
    .await;

    let member = create_member(&app, "Joe").await;
    let user = create_user(&app, "joe", &["basic_user"]).await;
    send(
        &app,
        Call::new(
            "PUT",
            format!("/api/v1/members/{}/account", member["id"].as_str().unwrap()),
        )
        .if_match(member["revision"].as_i64().unwrap())
        .body(json!({"user_id": user["id"]})),
    )
    .await;

    let (status, body, _) = send(
        &app,
        Call::new(
            "POST",
            format!("/api/v1/meal-plan-entries/{entry_id}/reopen"),
        )
        .if_match(2)
        .signed_in_as("joe"),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
}

#[tokio::test]
async fn a_meal_plan_week_must_start_on_monday() {
    let app = app().await;

    let (status, body, _) = send(&app, Call::new("GET", "/api/v1/meal-plan/2026-08-25")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

#[tokio::test]
async fn an_unlinked_user_cannot_use_a_personal_meal_plan() {
    let app = app().await;
    create_user(&app, "joe", &["basic_user"]).await;
    let product = create_milk_product(&app).await;

    let (status, body, _) = send(
        &app,
        Call::new("POST", "/api/v1/meal-plan-entries")
            .signed_in_as("joe")
            .body(json!({
                "planned_on": "2026-08-25",
                "slot": "breakfast",
                "components": [{"product_id": product["id"], "amount": measured_amount(100.0)}]
            })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(
        body["type"],
        "https://macrosandmealplans.dev/problems/member-link-required"
    );
}

#[tokio::test]
async fn a_personal_meal_plan_cannot_be_opened_by_another_member() {
    let app = app().await;
    let product = create_milk_product(&app).await;
    let entry = send(
        &app,
        Call::new("POST", "/api/v1/meal-plan-entries").body(json!({
            "planned_on": "2026-08-25",
            "slot": "breakfast",
            "components": [{"product_id": product["id"], "amount": measured_amount(100.0)}]
        })),
    )
    .await
    .1;

    let member = create_member(&app, "Joe").await;
    let user = create_user(&app, "joe", &["basic_user"]).await;
    send(
        &app,
        Call::new(
            "PUT",
            format!("/api/v1/members/{}/account", member["id"].as_str().unwrap()),
        )
        .if_match(member["revision"].as_i64().unwrap())
        .body(json!({"user_id": user["id"]})),
    )
    .await;

    let (status, body, _) = send(
        &app,
        Call::new(
            "GET",
            format!(
                "/api/v1/meal-plan-entries/{}",
                entry["id"].as_str().unwrap()
            ),
        )
        .signed_in_as("joe"),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
}

#[tokio::test]
async fn the_trusted_admin_can_open_another_members_entry() {
    let app = app().await;
    let member = create_member(&app, "Joe").await;
    let user = create_user(&app, "joe", &["basic_user"]).await;
    send(
        &app,
        Call::new(
            "PUT",
            format!("/api/v1/members/{}/account", member["id"].as_str().unwrap()),
        )
        .if_match(member["revision"].as_i64().unwrap())
        .body(json!({"user_id": user["id"]})),
    )
    .await;

    let product = create_milk_product(&app).await;
    let entry = send(
        &app,
        Call::new("POST", "/api/v1/meal-plan-entries")
            .signed_in_as("joe")
            .body(json!({
                "planned_on": "2026-08-25",
                "slot": "breakfast",
                "components": [{"product_id": product["id"], "amount": measured_amount(100.0)}]
            })),
    )
    .await
    .1;

    let (status, body, _) = send(
        &app,
        Call::new(
            "GET",
            format!(
                "/api/v1/meal-plan-entries/{}",
                entry["id"].as_str().unwrap()
            ),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

async fn my_member_id(app: &Router) -> String {
    let (status, body, _) = send(app, Call::new("GET", "/api/v1/auth/me")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["member_id"]
        .as_str()
        .expect("the admin should be linked to a member")
        .to_owned()
}

#[tokio::test]
async fn nutrition_targets_round_trip() {
    let app = app().await;
    let member = my_member_id(&app).await;

    let (status, created, headers) = send(
        &app,
        Call::new(
            "POST",
            format!("/api/v1/members/{member}/nutrition-targets"),
        )
        .body(json!({
            "effective_from": "2026-08-25",
            "energy_kcal": 2000.0,
            "protein_g": 120.0,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["energy_kcal"], 2000.0);
    assert_eq!(created["protein_g"], 120.0);
    assert!(created["fat_g"].is_null());
    assert_eq!(etag(&headers), "1");

    let id = created["id"].as_str().unwrap();
    let (status, updated, _) = send(
        &app,
        Call::new("PATCH", format!("/api/v1/nutrition-targets/{id}"))
            .if_match(1)
            .body(json!({"energy_kcal": 1800.0, "protein_g": null})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["energy_kcal"], 1800.0);
    assert!(updated["protein_g"].is_null(), "null clears the field");
    assert_eq!(updated["revision"], 2);

    let (status, listed, _) = send(
        &app,
        Call::new("GET", format!("/api/v1/members/{member}/nutrition-targets")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed.as_array().unwrap().len(), 1);

    let (status, _, _) = send(
        &app,
        Call::new("DELETE", format!("/api/v1/nutrition-targets/{id}")).if_match(2),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn updating_a_target_without_if_match_is_precondition_required() {
    let app = app().await;
    let member = my_member_id(&app).await;
    let (_, created, _) = send(
        &app,
        Call::new(
            "POST",
            format!("/api/v1/members/{member}/nutrition-targets"),
        )
        .body(json!({"effective_from": "2026-08-25", "energy_kcal": 2000.0})),
    )
    .await;
    let id = created["id"].as_str().unwrap();

    let (status, body, _) = send(
        &app,
        Call::new("PATCH", format!("/api/v1/nutrition-targets/{id}"))
            .body(json!({"energy_kcal": 1900.0})),
    )
    .await;
    assert_eq!(status, StatusCode::PRECONDITION_REQUIRED, "{body}");
}

#[tokio::test]
async fn a_stale_target_revision_conflicts() {
    let app = app().await;
    let member = my_member_id(&app).await;
    let (_, created, _) = send(
        &app,
        Call::new(
            "POST",
            format!("/api/v1/members/{member}/nutrition-targets"),
        )
        .body(json!({"effective_from": "2026-08-25", "energy_kcal": 2000.0})),
    )
    .await;
    let id = created["id"].as_str().unwrap();

    let (status, body, _) = send(
        &app,
        Call::new("PATCH", format!("/api/v1/nutrition-targets/{id}"))
            .if_match(99)
            .body(json!({"energy_kcal": 1900.0})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["actual_revision"], 1);
}

#[tokio::test]
async fn a_duplicate_effective_date_conflicts() {
    let app = app().await;
    let member = my_member_id(&app).await;
    let make = || {
        Call::new(
            "POST",
            format!("/api/v1/members/{member}/nutrition-targets"),
        )
        .body(json!({"effective_from": "2026-08-25", "energy_kcal": 2000.0}))
    };
    let (status, _, _) = send(&app, make()).await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, body, _) = send(&app, make()).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
}

#[tokio::test]
async fn an_empty_target_is_unprocessable() {
    let app = app().await;
    let member = my_member_id(&app).await;
    let (status, body, _) = send(
        &app,
        Call::new(
            "POST",
            format!("/api/v1/members/{member}/nutrition-targets"),
        )
        .body(json!({"effective_from": "2026-08-25"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
}

#[tokio::test]
async fn another_member_cannot_read_your_targets() {
    let app = app().await;
    let member = my_member_id(&app).await;
    create_user(&app, "joe", &["basic_user"]).await;

    let (status, body, _) = send(
        &app,
        Call::new("GET", format!("/api/v1/members/{member}/nutrition-targets")).signed_in_as("joe"),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
}

#[tokio::test]
async fn the_meal_plan_week_carries_a_resolved_target() {
    let app = app().await;
    let member = my_member_id(&app).await;
    send(
        &app,
        Call::new(
            "POST",
            format!("/api/v1/members/{member}/nutrition-targets"),
        )
        .body(json!({"effective_from": "2026-08-01", "energy_kcal": 2000.0})),
    )
    .await;

    let (status, week, _) = send(&app, Call::new("GET", "/api/v1/meal-plan/2026-08-24")).await;
    assert_eq!(status, StatusCode::OK, "{week}");
    assert_eq!(week["target"]["energy_kcal"], 14000.0);
    assert_eq!(week["days"][0]["target"]["energy_kcal"], 2000.0);
    assert!(
        week["insufficient_target_coverage"].as_array().is_none()
            || week["insufficient_target_coverage"]
                .as_array()
                .unwrap()
                .is_empty()
    );
}

#[tokio::test]
async fn a_midweek_target_reports_insufficient_weekly_coverage() {
    let app = app().await;
    let member = my_member_id(&app).await;
    send(
        &app,
        Call::new(
            "POST",
            format!("/api/v1/members/{member}/nutrition-targets"),
        )
        .body(json!({"effective_from": "2026-08-26", "energy_kcal": 2000.0})),
    )
    .await;

    let (status, week, _) = send(&app, Call::new("GET", "/api/v1/meal-plan/2026-08-24")).await;
    assert_eq!(status, StatusCode::OK, "{week}");
    assert!(
        week["target"].is_null(),
        "a partial week has no weekly target"
    );
    assert_eq!(week["insufficient_target_coverage"], json!(["energy_kcal"]));
    assert!(week["days"][0]["target"].is_null());
    assert_eq!(week["days"][2]["target"]["energy_kcal"], 2000.0);
}

#[tokio::test]
async fn creates_a_recipe_and_derives_its_nutrition() {
    let app = app().await;
    let product = create_milk_product(&app).await;
    let product_id = product["id"].as_str().unwrap();

    let (status, recipe, _) = send(
        &app,
        Call::new("POST", "/api/v1/recipes").body(json!({
            "name": "Warm Milk",
            "description": "A quiet drink",
            "servings": 2,
            "preparation_minutes": 5,
            "cooking_minutes": 10,
            "instructions": [{"text": "Warm gently"}],
            "meal_categories": ["snack"],
            "country_categories": ["GB"],
            "tags": ["Quick", "quick"],
            "components": [
                {"product_id": product_id, "amount": measured_amount(100.0)}
            ]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{recipe}");
    assert_eq!(recipe["visibility"], "private");
    let recipe_id = recipe["id"].as_str().unwrap().to_owned();

    let (status, fetched, headers) = send(
        &app,
        Call::new("GET", format!("/api/v1/recipes/{recipe_id}")),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{fetched}");
    assert_eq!(fetched["components"][0]["product_id"], product_id);
    assert_eq!(
        fetched["components"][0]["product_name"],
        "Tesco Whole Milk 1L"
    );
    assert_eq!(fetched["instructions"][0]["text"], "Warm gently");
    assert_eq!(fetched["tags"], json!(["Quick"]));
    assert!(!etag(&headers).is_empty());

    // 100g of a 64 kcal/100g product across 2 servings => 32 kcal per serving.
    let (status, nutrition, _) = send(
        &app,
        Call::new("GET", format!("/api/v1/recipes/{recipe_id}/nutrition")),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{nutrition}");
    assert_eq!(nutrition["nutrition"]["energy_kcal"], 32.0);
}

#[tokio::test]
async fn uploads_caches_and_deletes_a_recipe_photo() {
    let app = app().await;
    let product = create_milk_product(&app).await;
    let (status, recipe, headers) = send(
        &app,
        Call::new("POST", "/api/v1/recipes").body(json!({
            "name": "Photo recipe",
            "servings": 1,
            "components": [{"product_id": product["id"], "amount": measured_amount(100.0)}]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{recipe}");
    let id = recipe["id"].as_str().unwrap();
    let revision = etag(&headers);
    let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(800, 400, image::Rgb([90, 40, 20])));
    let mut png = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .unwrap();

    let (status, _, _) = send_bytes(
        &app,
        "PUT",
        format!("/api/v1/recipes/{id}/photo"),
        "image/gif",
        png.clone(),
        Some(&revision),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, _, _) = send_bytes(
        &app,
        "PUT",
        format!("/api/v1/recipes/{id}/photo"),
        "image/png",
        b"not an image".to_vec(),
        Some(&revision),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, _, _) = send_bytes(
        &app,
        "PUT",
        format!("/api/v1/recipes/{id}/photo"),
        "image/png",
        png.clone(),
        Some("999"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, _, _) = send_bytes(
        &app,
        "PUT",
        format!("/api/v1/recipes/{id}/photo"),
        "image/png",
        vec![0; 20 * 1024 * 1024 + 1],
        Some(&revision),
    )
    .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);

    let (status, uploaded, headers) = send_bytes(
        &app,
        "PUT",
        format!("/api/v1/recipes/{id}/photo"),
        "image/png",
        png,
        Some(&revision),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&uploaded)
    );
    let uploaded: Value = serde_json::from_slice(&uploaded).unwrap();
    assert_eq!(uploaded["photo_version"], 1);
    let photo_revision = etag(&headers);

    let (status, photo, headers) = send_bytes(
        &app,
        "GET",
        format!("/api/v1/recipes/{id}/photo/card"),
        "application/octet-stream",
        vec![],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(photo.starts_with(&[0xff, 0xd8]));
    assert_eq!(headers.get(header::CONTENT_TYPE).unwrap(), "image/jpeg");
    assert!(
        headers
            .get(header::CACHE_CONTROL)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("private")
    );
    let photo_etag = headers.get(header::ETAG).unwrap().clone();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/recipes/{id}/photo/card"))
                .header(header::AUTHORIZATION, credential())
                .header(header::IF_NONE_MATCH, photo_etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);

    let (status, deleted, _) = send_bytes(
        &app,
        "DELETE",
        format!("/api/v1/recipes/{id}/photo"),
        "application/octet-stream",
        vec![],
        Some(&photo_revision),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&deleted)
    );
    let deleted: Value = serde_json::from_slice(&deleted).unwrap();
    assert!(deleted["photo_version"].is_null());
}

#[tokio::test]
async fn recipes_require_authentication() {
    let app = app().await;
    let (status, _, _) = send(&app, Call::new("GET", "/api/v1/recipes").anonymous()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

async fn create_recipe(app: &Router, name: &str, product_id: &Value) -> Value {
    let (status, body, _) = send(
        app,
        Call::new("POST", "/api/v1/recipes").body(json!({
            "name": name,
            "servings": 1,
            "components": [{"product_id": product_id, "amount": measured_amount(100.0)}]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body
}

#[tokio::test]
async fn updates_a_recipe_and_bumps_its_revision() {
    let app = app().await;
    let product = create_milk_product(&app).await;
    let recipe = create_recipe(&app, "Draft name", &product["id"]).await;
    let id = recipe["id"].as_str().unwrap();

    let (status, _, _) = send(
        &app,
        Call::new("PATCH", format!("/api/v1/recipes/{id}")).body(json!({"name": "Final name"})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::PRECONDITION_REQUIRED,
        "a recipe update without If-Match must be refused"
    );

    let (status, updated, _) = send(
        &app,
        Call::new("PATCH", format!("/api/v1/recipes/{id}"))
            .if_match(recipe["revision"].as_i64().unwrap())
            .body(json!({"name": "Final name", "tags": ["Weeknight"]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["name"], "Final name");
    assert_eq!(updated["tags"], json!(["Weeknight"]));
    assert_eq!(
        updated["revision"],
        recipe["revision"].as_i64().unwrap() + 1
    );
}

#[tokio::test]
async fn archiving_a_recipe_hides_it_from_the_default_list() {
    let app = app().await;
    let product = create_milk_product(&app).await;
    let recipe = create_recipe(&app, "Retired recipe", &product["id"]).await;
    let id = recipe["id"].as_str().unwrap();

    let (status, archived, _) = send(
        &app,
        Call::new("POST", format!("/api/v1/recipes/{id}/archive")).if_match(1),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{archived}");
    assert!(!archived["archived_at"].is_null());

    let (_, listed, _) = send(&app, Call::new("GET", "/api/v1/recipes")).await;
    assert_eq!(listed["total"], 0);

    let (_, listed, _) = send(
        &app,
        Call::new("GET", "/api/v1/recipes?include_archived=true"),
    )
    .await;
    assert_eq!(listed["total"], 1);

    let (status, restored, _) = send(
        &app,
        Call::new("POST", format!("/api/v1/recipes/{id}/unarchive"))
            .if_match(archived["revision"].as_i64().unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{restored}");
    assert!(restored["archived_at"].is_null());

    let (_, listed, _) = send(&app, Call::new("GET", "/api/v1/recipes")).await;
    assert_eq!(listed["total"], 1);
}

#[tokio::test]
async fn lists_recipes_with_search_and_pagination() {
    let app = app().await;
    let product = create_milk_product(&app).await;
    for n in 0..7 {
        create_recipe(&app, &format!("Recipe {n}"), &product["id"]).await;
    }
    create_recipe(&app, "Lemon Drizzle", &product["id"]).await;

    let (status, body, _) = send(&app, Call::new("GET", "/api/v1/recipes?page=2&per_page=3")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["total"], 8);
    assert_eq!(body["page"], 2);
    assert_eq!(body["total_pages"], 3);
    assert_eq!(body["items"].as_array().unwrap().len(), 3);

    let (_, body, _) = send(&app, Call::new("GET", "/api/v1/recipes?q=lemon")).await;
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["name"], "Lemon Drizzle");
}
