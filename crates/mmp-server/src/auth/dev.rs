use std::sync::Arc;

use async_trait::async_trait;
use axum::http::HeaderMap;
use axum::http::header::AUTHORIZATION;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use mmp_core::services::HouseholdService;

use super::{AuthError, AuthProvider, Principal};

// 2026-08-22: Development only, basic auth will be replaced by OpenID Connect before release
pub struct DevBasicAuthProvider {
    household: Arc<HouseholdService>,
    password: String,
}

impl DevBasicAuthProvider {
    pub fn new(household: Arc<HouseholdService>, password: impl Into<String>) -> Self {
        Self {
            household,
            password: password.into(),
        }
    }
}

#[async_trait]
impl AuthProvider for DevBasicAuthProvider {
    async fn authenticate(&self, headers: &HeaderMap) -> Result<Principal, AuthError> {
        let (username, password) = read_basic(headers)?;

        if !constant_time_eq(password.as_bytes(), self.password.as_bytes()) {
            return Err(AuthError::InvalidCredentials);
        }

        let user = self
            .household
            .find_user_by_username(&username)
            .await
            .map_err(|_| AuthError::Unavailable)?
            .ok_or(AuthError::InvalidCredentials)?;

        if user.is_archived() {
            return Err(AuthError::InvalidCredentials);
        }

        let member = self
            .household
            .find_member_by_linked_user(user.id)
            .await
            .map_err(|_| AuthError::Unavailable)?;

        Ok(Principal::from_user(&user, member.map(|m| m.id)))
    }
}

fn read_basic(headers: &HeaderMap) -> Result<(String, String), AuthError> {
    let header = headers
        .get(AUTHORIZATION)
        .ok_or(AuthError::MissingCredentials)?
        .to_str()
        .map_err(|_| AuthError::MalformedCredentials("the header was not valid ASCII"))?;

    let encoded = header
        .strip_prefix("Basic ")
        .ok_or(AuthError::MalformedCredentials("expected a Basic scheme"))?;

    let decoded = STANDARD
        .decode(encoded.trim())
        .map_err(|_| AuthError::MalformedCredentials("the payload was not valid base64"))?;
    let decoded = String::from_utf8(decoded)
        .map_err(|_| AuthError::MalformedCredentials("the payload was not valid UTF-8"))?;

    let (username, password) = decoded
        .split_once(':')
        .ok_or(AuthError::MalformedCredentials("expected `user:password`"))?;

    Ok((username.to_owned(), password.to_owned()))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::Permission;
    use mmp_core::domain::{NewHouseholdMember, NewUser, Role};
    use mmp_core::ports::SystemClock;
    use mmp_core::testing::{
        InMemoryAccessGrantRepository, InMemoryHouseholdMemberRepository, InMemoryUserRepository,
    };

    const PASSWORD: &str = "changeme";

    fn household() -> Arc<HouseholdService> {
        Arc::new(HouseholdService::new(
            Arc::new(InMemoryHouseholdMemberRepository::new()),
            Arc::new(InMemoryUserRepository::new()),
            Arc::new(InMemoryAccessGrantRepository::new()),
            Arc::new(SystemClock),
        ))
    }

    async fn provider_with_admin() -> (DevBasicAuthProvider, Arc<HouseholdService>) {
        let household = household();
        household
            .create_user(NewUser {
                id: None,
                username: "admin".to_owned(),
                display_name: None,
                roles: vec![Role::Admin],
            })
            .await
            .unwrap();
        let provider = DevBasicAuthProvider::new(household.clone(), PASSWORD);
        (provider, household)
    }

    fn header_for(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, value.parse().unwrap());
        headers
    }

    fn basic(user: &str, password: &str) -> String {
        format!("Basic {}", STANDARD.encode(format!("{user}:{password}")))
    }

    #[tokio::test]
    async fn accepts_a_known_account() {
        let (provider, _) = provider_with_admin().await;
        let principal = provider
            .authenticate(&header_for(&basic("admin", PASSWORD)))
            .await
            .unwrap();

        assert_eq!(principal.username, "admin");
        assert!(principal.has(Permission::AccountAdmin));
    }

    #[tokio::test]
    async fn rejects_a_wrong_password() {
        let (provider, _) = provider_with_admin().await;
        let err = provider
            .authenticate(&header_for(&basic("admin", "nope")))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidCredentials));
    }

    #[tokio::test]
    async fn rejects_an_unknown_account() {
        let (provider, _) = provider_with_admin().await;
        let err = provider
            .authenticate(&header_for(&basic("nobody", PASSWORD)))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidCredentials));
    }

    #[tokio::test]
    async fn rejects_an_archived_account() {
        let (provider, household) = provider_with_admin().await;
        household
            .create_user(NewUser {
                id: None,
                username: "root".to_owned(),
                display_name: None,
                roles: vec![Role::Admin],
            })
            .await
            .unwrap();
        let joe = household
            .create_user(NewUser {
                id: None,
                username: "joe".to_owned(),
                display_name: None,
                roles: vec![Role::BasicUser],
            })
            .await
            .unwrap();
        household
            .set_user_archived(joe.id, joe.revision, true)
            .await
            .unwrap();

        let err = provider
            .authenticate(&header_for(&basic("joe", PASSWORD)))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidCredentials));
    }

    #[tokio::test]
    async fn the_principal_carries_the_roles_that_are_stored() {
        let (provider, household) = provider_with_admin().await;
        household
            .create_user(NewUser {
                id: None,
                username: "joe".to_owned(),
                display_name: None,
                roles: vec![Role::BasicUser],
            })
            .await
            .unwrap();

        let principal = provider
            .authenticate(&header_for(&basic("joe", PASSWORD)))
            .await
            .unwrap();

        assert_eq!(principal.roles, vec![Role::BasicUser]);
        assert!(principal.has(Permission::CatalogueWrite));
        assert!(!principal.has(Permission::AccountAdmin));
    }

    #[tokio::test]
    async fn the_principal_resolves_a_linked_member() {
        let (provider, household) = provider_with_admin().await;
        let user = household
            .find_user_by_username("admin")
            .await
            .unwrap()
            .unwrap();
        let member = household
            .create_member(NewHouseholdMember {
                id: None,
                display_name: "Admin".to_owned(),
                linked_user_id: None,
            })
            .await
            .unwrap();
        household
            .link_account(member.id, member.revision, user.id)
            .await
            .unwrap();

        let principal = provider
            .authenticate(&header_for(&basic("admin", PASSWORD)))
            .await
            .unwrap();

        assert_eq!(principal.member_id, Some(member.id));
    }

    #[tokio::test]
    async fn an_account_without_a_member_has_no_member_id() {
        let (provider, _) = provider_with_admin().await;
        let principal = provider
            .authenticate(&header_for(&basic("admin", PASSWORD)))
            .await
            .unwrap();
        assert_eq!(principal.member_id, None);
    }

    #[tokio::test]
    async fn reports_missing_credentials_separately() {
        let (provider, _) = provider_with_admin().await;
        let err = provider.authenticate(&HeaderMap::new()).await.unwrap_err();
        assert!(matches!(err, AuthError::MissingCredentials));
    }

    #[tokio::test]
    async fn rejects_a_non_basic_scheme() {
        let (provider, _) = provider_with_admin().await;
        let err = provider
            .authenticate(&header_for("Bearer sometoken"))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::MalformedCredentials(_)));
    }
}
