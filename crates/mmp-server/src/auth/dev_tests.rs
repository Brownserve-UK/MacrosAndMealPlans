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
