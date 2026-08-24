use std::sync::Arc;

use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

use crate::openapi::ApiDoc;
use crate::routes;
use crate::state::AppState;

pub fn build(state: AppState) -> (Router, utoipa::openapi::OpenApi) {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(routes::meta::router())
        .merge(routes::auth::router())
        .merge(routes::ingredients::router())
        .merge(routes::products::router())
        .merge(routes::members::router())
        .merge(routes::users::router())
        .merge(routes::diary::router())
        .split_for_parts();

    let router = router
        .merge(Scalar::with_url("/docs", api.clone()))
        .route(
            "/openapi.json",
            axum::routing::get({
                let api = api.clone();
                move || async move { axum::Json(api) }
            }),
        )
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    (router, api)
}

pub fn with_web_client(router: Router, web_dist: Option<&str>) -> Router {
    let Some(dist) = web_dist else {
        return router;
    };

    let index = std::path::Path::new(dist).join("index.html");
    router.fallback_service(
        tower_http::services::ServeDir::new(dist)
            .fallback(tower_http::services::ServeFile::new(index)),
    )
}

pub fn stub_state() -> AppState {
    use mmp_core::ports::SystemClock;

    let household = Arc::new(mmp_core::services::HouseholdService::new(
        Arc::new(NoopMembers),
        Arc::new(NoopUsers),
        Arc::new(NoopGrants),
        Arc::new(SystemClock),
    ));

    AppState::new(
        mmp_core::services::CatalogueService::new(
            Arc::new(NoopIngredients),
            Arc::new(NoopProducts),
            Arc::new(SystemClock),
        ),
        household.clone(),
        mmp_core::services::DiaryService::new(
            Arc::new(NoopConsumptionRecords),
            Arc::new(NoopProducts),
            Arc::new(SystemClock),
        ),
        Arc::new(crate::auth::DevBasicAuthProvider::new(household, "")),
    )
}

struct NoopIngredients;
struct NoopProducts;
struct NoopMembers;
struct NoopUsers;
struct NoopGrants;
struct NoopConsumptionRecords;

#[async_trait::async_trait]
impl mmp_core::ports::IngredientRepository for NoopIngredients {
    async fn get(
        &self,
        _: mmp_core::domain::IngredientId,
    ) -> mmp_core::Result<Option<mmp_core::domain::Ingredient>> {
        Ok(None)
    }
    async fn find_by_name(
        &self,
        _: &str,
    ) -> mmp_core::Result<Option<mmp_core::domain::Ingredient>> {
        Ok(None)
    }
    async fn find_by_seed_key(
        &self,
        _: &str,
    ) -> mmp_core::Result<Option<mmp_core::domain::Ingredient>> {
        Ok(None)
    }
    async fn list(
        &self,
        q: &mmp_core::ports::IngredientQuery,
    ) -> mmp_core::Result<mmp_core::ports::Paginated<mmp_core::domain::Ingredient>> {
        Ok(mmp_core::ports::Paginated::new(vec![], 0, q.page))
    }
    async fn insert(&self, _: &mmp_core::domain::Ingredient) -> mmp_core::Result<()> {
        Ok(())
    }
    async fn update(
        &self,
        _: &mmp_core::domain::Ingredient,
        _: mmp_core::domain::Revision,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
}

#[async_trait::async_trait]
impl mmp_core::ports::ProductRepository for NoopProducts {
    async fn count_by_ingredient(
        &self,
        _: &[mmp_core::domain::IngredientId],
    ) -> mmp_core::Result<std::collections::HashMap<mmp_core::domain::IngredientId, i64>> {
        Ok(Default::default())
    }
    async fn get(
        &self,
        _: mmp_core::domain::ProductId,
    ) -> mmp_core::Result<Option<mmp_core::domain::Product>> {
        Ok(None)
    }
    async fn find_by_barcode(
        &self,
        _: &str,
    ) -> mmp_core::Result<Option<mmp_core::domain::Product>> {
        Ok(None)
    }
    async fn find_by_seed_key(
        &self,
        _: &str,
    ) -> mmp_core::Result<Option<mmp_core::domain::Product>> {
        Ok(None)
    }
    async fn list(
        &self,
        q: &mmp_core::ports::ProductQuery,
    ) -> mmp_core::Result<mmp_core::ports::Paginated<mmp_core::domain::Product>> {
        Ok(mmp_core::ports::Paginated::new(vec![], 0, q.page))
    }
    async fn insert(&self, _: &mmp_core::domain::Product) -> mmp_core::Result<()> {
        Ok(())
    }
    async fn update(
        &self,
        _: &mmp_core::domain::Product,
        _: mmp_core::domain::Revision,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
}

#[async_trait::async_trait]
impl mmp_core::ports::HouseholdMemberRepository for NoopMembers {
    async fn get(
        &self,
        _: mmp_core::domain::HouseholdMemberId,
    ) -> mmp_core::Result<Option<mmp_core::domain::HouseholdMember>> {
        Ok(None)
    }
    async fn find_by_display_name(
        &self,
        _: &str,
    ) -> mmp_core::Result<Option<mmp_core::domain::HouseholdMember>> {
        Ok(None)
    }
    async fn find_by_linked_user(
        &self,
        _: mmp_core::domain::UserId,
    ) -> mmp_core::Result<Option<mmp_core::domain::HouseholdMember>> {
        Ok(None)
    }
    async fn list(
        &self,
        q: &mmp_core::ports::MemberQuery,
    ) -> mmp_core::Result<mmp_core::ports::Paginated<mmp_core::domain::HouseholdMember>> {
        Ok(mmp_core::ports::Paginated::new(vec![], 0, q.page))
    }
    async fn insert(&self, _: &mmp_core::domain::HouseholdMember) -> mmp_core::Result<()> {
        Ok(())
    }
    async fn update(
        &self,
        _: &mmp_core::domain::HouseholdMember,
        _: mmp_core::domain::Revision,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
}

#[async_trait::async_trait]
impl mmp_core::ports::UserRepository for NoopUsers {
    async fn get(
        &self,
        _: mmp_core::domain::UserId,
    ) -> mmp_core::Result<Option<mmp_core::domain::User>> {
        Ok(None)
    }
    async fn find_by_username(&self, _: &str) -> mmp_core::Result<Option<mmp_core::domain::User>> {
        Ok(None)
    }
    async fn list(
        &self,
        q: &mmp_core::ports::UserQuery,
    ) -> mmp_core::Result<mmp_core::ports::Paginated<mmp_core::domain::User>> {
        Ok(mmp_core::ports::Paginated::new(vec![], 0, q.page))
    }
    async fn count_with_role(&self, _: mmp_core::domain::Role, _: bool) -> mmp_core::Result<i64> {
        Ok(0)
    }
    async fn insert(&self, _: &mmp_core::domain::User) -> mmp_core::Result<()> {
        Ok(())
    }
    async fn update(
        &self,
        _: &mmp_core::domain::User,
        _: mmp_core::domain::Revision,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
}

#[async_trait::async_trait]
impl mmp_core::ports::ConsumptionRecordRepository for NoopConsumptionRecords {
    async fn get(
        &self,
        _: mmp_core::domain::ConsumptionRecordId,
    ) -> mmp_core::Result<Option<mmp_core::domain::ConsumptionRecord>> {
        Ok(None)
    }
    async fn list(
        &self,
        q: &mmp_core::ports::ConsumptionQuery,
    ) -> mmp_core::Result<mmp_core::ports::Paginated<mmp_core::domain::ConsumptionRecord>> {
        Ok(mmp_core::ports::Paginated::new(vec![], 0, q.page))
    }
    async fn insert(&self, _: &mmp_core::domain::ConsumptionRecord) -> mmp_core::Result<()> {
        Ok(())
    }
    async fn update(
        &self,
        _: &mmp_core::domain::ConsumptionRecord,
        _: mmp_core::domain::Revision,
    ) -> mmp_core::Result<mmp_core::ports::UpdateOutcome> {
        Ok(mmp_core::ports::UpdateOutcome::NotFound)
    }
    async fn delete(&self, _: mmp_core::domain::ConsumptionRecordId) -> mmp_core::Result<bool> {
        Ok(false)
    }
}

#[async_trait::async_trait]
impl mmp_core::ports::AccessGrantRepository for NoopGrants {
    async fn list_for_member(
        &self,
        _: mmp_core::domain::HouseholdMemberId,
    ) -> mmp_core::Result<Vec<mmp_core::domain::MemberAccessGrant>> {
        Ok(vec![])
    }
    async fn exists(
        &self,
        _: mmp_core::domain::UserId,
        _: mmp_core::domain::HouseholdMemberId,
        _: mmp_core::domain::AccessScope,
    ) -> mmp_core::Result<bool> {
        Ok(false)
    }
    async fn upsert(&self, _: &mmp_core::domain::MemberAccessGrant) -> mmp_core::Result<()> {
        Ok(())
    }
    async fn revoke(
        &self,
        _: mmp_core::domain::UserId,
        _: mmp_core::domain::HouseholdMemberId,
        _: mmp_core::domain::AccessScope,
    ) -> mmp_core::Result<bool> {
        Ok(false)
    }
}
