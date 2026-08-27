use axum::Json;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH, VARY};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use mmp_core::domain::{RecipeId, UserId};
use mmp_core::ports::{PageRequest, RecipeQuery};
use mmp_core::{CoreError, ValidationErrors};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::{Permission, Principal};
use crate::dto::{
    CreateRecipeRequest, RecipeDto, RecipeListQuery, RecipeNutritionDto,
    RecipeNutritionPreviewRequest, RecipePage, UpdateRecipeRequest,
};
use crate::error::ApiResult;
use crate::http::{Created, IfMatch, Tagged};
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    let upload = OpenApiRouter::new()
        .routes(routes!(put_photo))
        .route_layer(DefaultBodyLimit::max(20 * 1024 * 1024));
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(get_one, update))
        .routes(routes!(archive))
        .routes(routes!(unarchive))
        .routes(routes!(nutrition))
        .routes(routes!(nutrition_preview))
        .routes(routes!(get_photo, delete_photo))
        .merge(upload)
}

fn to_query(owner: UserId, query: RecipeListQuery) -> RecipeQuery {
    RecipeQuery {
        owner_id: owner,
        search: query.q.filter(|q| !q.trim().is_empty()),
        include_archived: query.include_archived.unwrap_or(false),
        page: PageRequest::new(
            query.page.unwrap_or(1),
            query.per_page.unwrap_or(PageRequest::DEFAULT_PER_PAGE),
        ),
        sort: query.sort.map(Into::into).unwrap_or_default(),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/recipes",
    params(RecipeListQuery),
    operation_id = "listRecipes",
    responses(
        (status = 200, description = "A page of the signed-in user's recipes", body = RecipePage),
        (status = 401, description = "Authentication required", body = crate::error::Problem),
    ),
    tag = "recipes",
    security(("basic" = []))
)]
async fn list(
    State(state): State<AppState>,
    principal: Principal,
    Query(query): Query<RecipeListQuery>,
) -> ApiResult<Json<RecipePage>> {
    principal.require(Permission::CatalogueRead)?;
    let page = state
        .recipes
        .list_recipes(&to_query(principal.user_id, query))
        .await?;
    Ok(Json(page.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/recipes",
    request_body = CreateRecipeRequest,
    operation_id = "createRecipe",
    responses(
        (status = 201, description = "Created", body = RecipeDto),
        (status = 422, description = "Validation failed", body = crate::error::Problem),
    ),
    tag = "recipes",
    security(("basic" = []))
)]
async fn create(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<CreateRecipeRequest>,
) -> ApiResult<Created<RecipeDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let created = state
        .recipes
        .create_recipe(body.into_domain(principal.user_id))
        .await?;
    let products = state.recipes.products_for(&created).await?;
    Ok(Created(
        created.revision,
        RecipeDto::from_domain(created, &products),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/recipes/{id}",
    operation_id = "getRecipe",
    params(("id" = Uuid, Path, description = "Recipe id")),
    responses(
        (status = 200, description = "The recipe", body = RecipeDto,
         headers(("ETag" = String, description = "The revision to send back as If-Match"))),
        (status = 404, description = "Not found", body = crate::error::Problem),
    ),
    tag = "recipes",
    security(("basic" = []))
)]
async fn get_one(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Tagged<RecipeDto>> {
    principal.require(Permission::CatalogueRead)?;
    let recipe = state
        .recipes
        .get_recipe(RecipeId::from(id), principal.user_id)
        .await?;
    let products = state.recipes.products_for(&recipe).await?;
    Ok(Tagged(
        recipe.revision,
        RecipeDto::from_domain(recipe, &products),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/recipes/{id}",
    operation_id = "updateRecipe",
    params(
        ("id" = Uuid, Path, description = "Recipe id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body = UpdateRecipeRequest,
    responses(
        (status = 200, description = "Updated", body = RecipeDto),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 428, description = "If-Match is required", body = crate::error::Problem),
    ),
    tag = "recipes",
    security(("basic" = []))
)]
async fn update(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    Json(body): Json<UpdateRecipeRequest>,
) -> ApiResult<Tagged<RecipeDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let updated = state
        .recipes
        .update_recipe(RecipeId::from(id), revision, body.into(), principal.user_id)
        .await?;
    let products = state.recipes.products_for(&updated).await?;
    Ok(Tagged(
        updated.revision,
        RecipeDto::from_domain(updated, &products),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/recipes/{id}/archive",
    operation_id = "archiveRecipe",
    params(
        ("id" = Uuid, Path, description = "Recipe id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Archived", body = RecipeDto)),
    tag = "recipes",
    security(("basic" = []))
)]
async fn archive(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<RecipeDto>> {
    set_archived(state, principal, id, revision, true).await
}

#[utoipa::path(
    post,
    path = "/api/v1/recipes/{id}/unarchive",
    operation_id = "unarchiveRecipe",
    params(
        ("id" = Uuid, Path, description = "Recipe id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses((status = 200, description = "Restored", body = RecipeDto)),
    tag = "recipes",
    security(("basic" = []))
)]
async fn unarchive(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<RecipeDto>> {
    set_archived(state, principal, id, revision, false).await
}

async fn set_archived(
    state: AppState,
    principal: Principal,
    id: Uuid,
    revision: mmp_core::domain::Revision,
    archived: bool,
) -> ApiResult<Tagged<RecipeDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let updated = state
        .recipes
        .set_recipe_archived(RecipeId::from(id), revision, archived, principal.user_id)
        .await?;
    let products = state.recipes.products_for(&updated).await?;
    Ok(Tagged(
        updated.revision,
        RecipeDto::from_domain(updated, &products),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/recipes/{id}/nutrition",
    operation_id = "getRecipeNutrition",
    params(("id" = Uuid, Path, description = "Recipe id")),
    responses(
        (status = 200, description = "Derived per-serving nutrition", body = RecipeNutritionDto),
        (status = 404, description = "Not found", body = crate::error::Problem),
    ),
    tag = "recipes",
    security(("basic" = []))
)]
async fn nutrition(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RecipeNutritionDto>> {
    principal.require(Permission::CatalogueRead)?;
    let nutrition = state
        .recipes
        .nutrition_for(RecipeId::from(id), principal.user_id)
        .await?;
    Ok(Json(nutrition.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/recipes/nutrition-preview",
    request_body = RecipeNutritionPreviewRequest,
    operation_id = "previewRecipeNutrition",
    responses(
        (status = 200, description = "Derived per-serving nutrition for an unsaved draft", body = RecipeNutritionDto),
    ),
    tag = "recipes",
    security(("basic" = []))
)]
async fn nutrition_preview(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<RecipeNutritionPreviewRequest>,
) -> ApiResult<Json<RecipeNutritionDto>> {
    principal.require(Permission::CatalogueRead)?;
    let components: Vec<mmp_core::domain::NewRecipeComponent> =
        body.components.into_iter().map(Into::into).collect();
    let nutrition = state
        .recipes
        .nutrition_preview(body.servings, &components)
        .await?;
    Ok(Json(nutrition.into()))
}

#[utoipa::path(
    put,
    path = "/api/v1/recipes/{id}/photo",
    operation_id = "putRecipePhoto",
    params(
        ("id" = Uuid, Path, description = "Recipe id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    request_body(content(
        (Vec<u8> = "image/jpeg"),
        (Vec<u8> = "image/png"),
        (Vec<u8> = "image/webp")
    )),
    responses(
        (status = 200, description = "Photo replaced", body = RecipeDto),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
        (status = 413, description = "Image exceeds 20 MB"),
        (status = 422, description = "Image is not supported", body = crate::error::Problem),
    ),
    tag = "recipes",
    security(("basic" = []))
)]
async fn put_photo(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Tagged<RecipeDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next());
    if !matches!(
        content_type,
        Some("image/jpeg" | "image/png" | "image/webp")
    ) {
        return Err(photo_validation("Choose a JPEG, PNG, or WebP image."));
    }
    let current = state
        .recipes
        .get_recipe(RecipeId::from(id), principal.user_id)
        .await?;
    if current.revision != revision {
        return Err(CoreError::RevisionMismatch {
            resource: "recipe",
            id: id.to_string(),
            expected: revision,
            actual: current.revision,
        }
        .into());
    }
    let derivatives = tokio::task::spawn_blocking(move || crate::photo::process(&body))
        .await
        .map_err(|_| {
            crate::error::ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "image-processing-failed",
                "Image processing failed",
                "The image could not be processed.",
            )
        })?
        .map_err(|message| photo_validation(&message))?;
    let updated = state
        .recipes
        .replace_photo(RecipeId::from(id), revision, derivatives, principal.user_id)
        .await?;
    let products = state.recipes.products_for(&updated).await?;
    Ok(Tagged(
        updated.revision,
        RecipeDto::from_domain(updated, &products),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/recipes/{id}/photo/{size}",
    operation_id = "getRecipePhoto",
    params(
        ("id" = Uuid, Path, description = "Recipe id"),
        ("size" = String, Path, description = "card or hero"),
    ),
    responses(
        (status = 200, description = "Processed JPEG", content_type = "image/jpeg", body = Vec<u8>),
        (status = 304, description = "Not modified"),
        (status = 404, description = "Not found", body = crate::error::Problem),
    ),
    tag = "recipes",
    security(("basic" = []))
)]
async fn get_photo(
    State(state): State<AppState>,
    principal: Principal,
    Path((id, size)): Path<(Uuid, String)>,
    request_headers: HeaderMap,
) -> ApiResult<Response> {
    principal.require(Permission::CatalogueRead)?;
    if size != "card" && size != "hero" {
        return Err(CoreError::not_found("recipe photo", id).into());
    }
    let photo = state
        .recipes
        .get_photo(RecipeId::from(id), principal.user_id)
        .await?;
    let etag = format!("\"photo-{}-{size}\"", photo.version);
    let mut response_headers = HeaderMap::new();
    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("image/jpeg"));
    response_headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=31536000, immutable"),
    );
    response_headers.insert(VARY, HeaderValue::from_static("Authorization"));
    if let Ok(value) = HeaderValue::from_str(&etag) {
        response_headers.insert(ETAG, value);
    }
    if request_headers
        .get(IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        == Some(etag.as_str())
    {
        return Ok((StatusCode::NOT_MODIFIED, response_headers).into_response());
    }
    let bytes = if size == "card" {
        photo.derivatives.card_jpeg
    } else {
        photo.derivatives.hero_jpeg
    };
    Ok((StatusCode::OK, response_headers, bytes).into_response())
}

#[utoipa::path(
    delete,
    path = "/api/v1/recipes/{id}/photo",
    operation_id = "deleteRecipePhoto",
    params(
        ("id" = Uuid, Path, description = "Recipe id"),
        ("If-Match" = String, Header, description = "The revision you loaded"),
    ),
    responses(
        (status = 200, description = "Photo removed", body = RecipeDto),
        (status = 409, description = "Someone else changed it first", body = crate::error::Problem),
    ),
    tag = "recipes",
    security(("basic" = []))
)]
async fn delete_photo(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<Uuid>,
    IfMatch(revision): IfMatch,
) -> ApiResult<Tagged<RecipeDto>> {
    principal.require(Permission::CatalogueWrite)?;
    let updated = state
        .recipes
        .delete_photo(RecipeId::from(id), revision, principal.user_id)
        .await?;
    let products = state.recipes.products_for(&updated).await?;
    Ok(Tagged(
        updated.revision,
        RecipeDto::from_domain(updated, &products),
    ))
}

fn photo_validation(message: &str) -> crate::error::ApiError {
    let mut errors = ValidationErrors::new();
    errors.push("photo", message);
    CoreError::Validation(errors).into()
}
