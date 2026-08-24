use std::sync::Arc;

use super::*;
use crate::domain::{Patch, Role};
use crate::ports::SystemClock;
use crate::testing::{
    InMemoryAccessGrantRepository, InMemoryHouseholdMemberRepository, InMemoryUserRepository,
};

struct Fixture {
    service: HouseholdService,
    members: InMemoryHouseholdMemberRepository,
    users: InMemoryUserRepository,
    grants: InMemoryAccessGrantRepository,
}

fn fixture() -> Fixture {
    let members = InMemoryHouseholdMemberRepository::new();
    let users = InMemoryUserRepository::new();
    let grants = InMemoryAccessGrantRepository::new();
    let service = HouseholdService::new(
        Arc::new(members.clone()),
        Arc::new(users.clone()),
        Arc::new(grants.clone()),
        Arc::new(SystemClock),
    );
    Fixture {
        service,
        members,
        users,
        grants,
    }
}

fn new_member(name: &str) -> NewHouseholdMember {
    NewHouseholdMember {
        id: None,
        display_name: name.to_owned(),
        linked_user_id: None,
    }
}

fn new_user(username: &str, roles: Vec<Role>) -> NewUser {
    NewUser {
        id: None,
        username: username.to_owned(),
        display_name: None,
        roles,
    }
}

async fn an_admin(service: &HouseholdService, username: &str) -> User {
    service
        .create_user(new_user(username, vec![Role::Admin]))
        .await
        .unwrap()
}

#[tokio::test]
async fn a_member_can_exist_without_an_account() {
    let f = fixture();
    let member = f.service.create_member(new_member("Joe")).await.unwrap();

    assert!(!member.has_account());
    assert_eq!(member.revision, Revision::INITIAL);
    assert_eq!(f.members.count(), 1);
    assert_eq!(f.users.count(), 0);
}

#[tokio::test]
async fn member_names_are_unique_case_insensitively() {
    let f = fixture();
    f.service.create_member(new_member("Joe")).await.unwrap();

    let err = f
        .service
        .create_member(new_member("joe"))
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Duplicate { .. }));
}

#[tokio::test]
async fn a_blank_member_name_is_rejected() {
    let f = fixture();
    let err = f.service.create_member(new_member("  ")).await.unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn renaming_a_member_advances_the_revision() {
    let f = fixture();
    let member = f.service.create_member(new_member("Joe")).await.unwrap();

    let updated = f
        .service
        .update_member(
            member.id,
            member.revision,
            HouseholdMemberPatch {
                display_name: Some("Joseph".to_owned()),
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.display_name, "Joseph");
    assert_eq!(updated.revision, member.revision.next());
}

#[tokio::test]
async fn a_stale_revision_is_rejected() {
    let f = fixture();
    let member = f.service.create_member(new_member("Joe")).await.unwrap();

    let err = f
        .service
        .update_member(
            member.id,
            Revision::new(99),
            HouseholdMemberPatch {
                display_name: Some("Joseph".to_owned()),
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(err, CoreError::RevisionMismatch { .. }));
}

#[tokio::test]
async fn keeping_a_members_own_name_is_allowed() {
    let f = fixture();
    let member = f.service.create_member(new_member("Joe")).await.unwrap();

    let updated = f
        .service
        .update_member(
            member.id,
            member.revision,
            HouseholdMemberPatch {
                display_name: Some("JOE".to_owned()),
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.display_name, "JOE");
}

#[tokio::test]
async fn archiving_a_member_round_trips() {
    let f = fixture();
    let member = f.service.create_member(new_member("Joe")).await.unwrap();

    let archived = f
        .service
        .set_member_archived(member.id, member.revision, true)
        .await
        .unwrap();
    assert!(archived.is_archived());

    let restored = f
        .service
        .set_member_archived(archived.id, archived.revision, false)
        .await
        .unwrap();
    assert!(!restored.is_archived());
}

#[tokio::test]
async fn linking_an_account_preserves_the_member() {
    let f = fixture();
    let member = f.service.create_member(new_member("Joe")).await.unwrap();
    let user = f
        .service
        .create_user(new_user("joe", vec![Role::BasicUser]))
        .await
        .unwrap();

    let linked = f
        .service
        .link_account(member.id, member.revision, user.id)
        .await
        .unwrap();

    assert_eq!(linked.id, member.id, "USR-013: the member is not replaced");
    assert_eq!(linked.created_at, member.created_at);
    assert_eq!(linked.linked_user_id, Some(user.id));
    assert_eq!(f.members.count(), 1);
}

#[tokio::test]
async fn one_account_cannot_serve_two_members() {
    let f = fixture();
    let user = f
        .service
        .create_user(new_user("joe", vec![Role::BasicUser]))
        .await
        .unwrap();
    let first = f.service.create_member(new_member("Joe")).await.unwrap();
    let second = f.service.create_member(new_member("Jo")).await.unwrap();

    f.service
        .link_account(first.id, first.revision, user.id)
        .await
        .unwrap();

    let err = f
        .service
        .link_account(second.id, second.revision, user.id)
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn an_archived_account_cannot_be_linked() {
    let f = fixture();
    an_admin(&f.service, "root").await;
    let user = f
        .service
        .create_user(new_user("joe", vec![Role::BasicUser]))
        .await
        .unwrap();
    let archived = f
        .service
        .set_user_archived(user.id, user.revision, true)
        .await
        .unwrap();
    let member = f.service.create_member(new_member("Joe")).await.unwrap();

    let err = f
        .service
        .link_account(member.id, member.revision, archived.id)
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn unlinking_leaves_the_member_and_the_account_alone() {
    let f = fixture();
    let member = f.service.create_member(new_member("Joe")).await.unwrap();
    let user = f
        .service
        .create_user(new_user("joe", vec![Role::BasicUser]))
        .await
        .unwrap();
    let linked = f
        .service
        .link_account(member.id, member.revision, user.id)
        .await
        .unwrap();

    let unlinked = f
        .service
        .unlink_account(linked.id, linked.revision)
        .await
        .unwrap();

    assert_eq!(unlinked.id, member.id);
    assert!(!unlinked.has_account());
    assert!(f.service.get_user(user.id).await.is_ok());
}

#[tokio::test]
async fn relinking_the_same_account_is_a_no_op() {
    let f = fixture();
    let member = f.service.create_member(new_member("Joe")).await.unwrap();
    let user = f
        .service
        .create_user(new_user("joe", vec![Role::BasicUser]))
        .await
        .unwrap();
    let linked = f
        .service
        .link_account(member.id, member.revision, user.id)
        .await
        .unwrap();

    let again = f
        .service
        .link_account(linked.id, linked.revision, user.id)
        .await
        .unwrap();
    assert_eq!(again.revision, linked.revision);
}

#[tokio::test]
async fn usernames_are_unique_case_insensitively() {
    let f = fixture();
    f.service
        .create_user(new_user("joe", vec![Role::BasicUser]))
        .await
        .unwrap();

    let err = f
        .service
        .create_user(new_user("JOE", vec![Role::BasicUser]))
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Duplicate { .. }));
}

#[tokio::test]
async fn roles_are_deduplicated_and_ordered() {
    let f = fixture();
    let user = f
        .service
        .create_user(new_user(
            "joe",
            vec![Role::BasicUser, Role::Admin, Role::BasicUser],
        ))
        .await
        .unwrap();

    assert_eq!(user.roles, vec![Role::Admin, Role::BasicUser]);
}

#[tokio::test]
async fn a_user_without_a_role_is_rejected() {
    let f = fixture();
    let err = f
        .service
        .create_user(new_user("joe", vec![]))
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn changing_roles_advances_the_revision() {
    let f = fixture();
    an_admin(&f.service, "root").await;
    let user = f
        .service
        .create_user(new_user("joe", vec![Role::BasicUser]))
        .await
        .unwrap();

    let promoted = f
        .service
        .set_user_roles(user.id, user.revision, vec![Role::HouseholdManager])
        .await
        .unwrap();

    assert_eq!(promoted.roles, vec![Role::HouseholdManager]);
    assert_eq!(promoted.revision, user.revision.next());
}

#[tokio::test]
async fn setting_the_same_roles_is_a_no_op() {
    let f = fixture();
    let user = f
        .service
        .create_user(new_user("joe", vec![Role::BasicUser]))
        .await
        .unwrap();

    let same = f
        .service
        .set_user_roles(user.id, user.revision, vec![Role::BasicUser])
        .await
        .unwrap();
    assert_eq!(same.revision, user.revision);
}

#[tokio::test]
async fn clearing_every_role_is_rejected() {
    let f = fixture();
    let user = f
        .service
        .create_user(new_user("joe", vec![Role::BasicUser]))
        .await
        .unwrap();

    let err = f
        .service
        .set_user_roles(user.id, user.revision, vec![])
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn the_last_admin_cannot_be_demoted() {
    let f = fixture();
    let admin = an_admin(&f.service, "admin").await;

    let err = f
        .service
        .set_user_roles(admin.id, admin.revision, vec![Role::BasicUser])
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn the_last_admin_cannot_be_archived() {
    let f = fixture();
    let admin = an_admin(&f.service, "admin").await;

    let err = f
        .service
        .set_user_archived(admin.id, admin.revision, true)
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn a_second_admin_frees_the_first_to_step_down() {
    let f = fixture();
    let first = an_admin(&f.service, "admin").await;
    an_admin(&f.service, "root").await;

    let demoted = f
        .service
        .set_user_roles(first.id, first.revision, vec![Role::BasicUser])
        .await
        .unwrap();
    assert_eq!(demoted.roles, vec![Role::BasicUser]);
}

#[tokio::test]
async fn an_archived_admin_does_not_count_towards_the_last_admin_guard() {
    let f = fixture();
    let keeper = an_admin(&f.service, "admin").await;
    let spare = an_admin(&f.service, "root").await;

    f.service
        .set_user_archived(spare.id, spare.revision, true)
        .await
        .unwrap();

    let err = f
        .service
        .set_user_archived(keeper.id, keeper.revision, true)
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}

#[tokio::test]
async fn a_display_name_can_be_cleared() {
    let f = fixture();
    let user = f
        .service
        .create_user(NewUser {
            id: None,
            username: "joe".to_owned(),
            display_name: Some("Joe Bloggs".to_owned()),
            roles: vec![Role::BasicUser],
        })
        .await
        .unwrap();

    let cleared = f
        .service
        .update_user(
            user.id,
            user.revision,
            UserPatch {
                display_name: Patch::Clear,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(cleared.display_name, None);
}

#[tokio::test]
async fn archiving_a_user_leaves_its_member_intact() {
    let f = fixture();
    an_admin(&f.service, "root").await;
    let member = f.service.create_member(new_member("Joe")).await.unwrap();
    let user = f
        .service
        .create_user(new_user("joe", vec![Role::BasicUser]))
        .await
        .unwrap();
    let linked = f
        .service
        .link_account(member.id, member.revision, user.id)
        .await
        .unwrap();

    f.service
        .set_user_archived(user.id, user.revision, true)
        .await
        .unwrap();

    let after = f.service.get_member(member.id).await.unwrap();
    assert_eq!(after.linked_user_id, Some(user.id));
    assert_eq!(after.revision, linked.revision);
    assert!(!after.is_archived());
}

#[tokio::test]
async fn an_admin_reaches_any_members_health_data() {
    let f = fixture();
    let admin = an_admin(&f.service, "admin").await;
    let member = f.service.create_member(new_member("Joe")).await.unwrap();

    assert!(
        f.service
            .can_view_member_health_data(&admin, member.id)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn a_household_manager_does_not_reach_health_data() {
    let f = fixture();
    let manager = f
        .service
        .create_user(new_user("manager", vec![Role::HouseholdManager]))
        .await
        .unwrap();
    let member = f.service.create_member(new_member("Joe")).await.unwrap();

    assert!(
        !f.service
            .can_view_member_health_data(&manager, member.id)
            .await
            .unwrap(),
        "USR-005: managing the household is not consent"
    );
}

#[tokio::test]
async fn a_user_reaches_their_own_health_data() {
    let f = fixture();
    let user = f
        .service
        .create_user(new_user("joe", vec![Role::BasicUser]))
        .await
        .unwrap();
    let member = f.service.create_member(new_member("Joe")).await.unwrap();
    f.service
        .link_account(member.id, member.revision, user.id)
        .await
        .unwrap();

    assert!(
        f.service
            .can_view_member_health_data(&user, member.id)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn an_explicit_grant_opens_health_data() {
    let f = fixture();
    let nutritionist = f
        .service
        .create_user(new_user("nutritionist", vec![Role::Nutritionist]))
        .await
        .unwrap();
    let member = f.service.create_member(new_member("Joe")).await.unwrap();

    assert!(
        !f.service
            .can_view_member_health_data(&nutritionist, member.id)
            .await
            .unwrap()
    );

    f.service
        .grant_access(member.id, nutritionist.id, AccessScope::HealthData, None)
        .await
        .unwrap();

    assert!(
        f.service
            .can_view_member_health_data(&nutritionist, member.id)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn a_grant_covers_only_the_member_it_names() {
    let f = fixture();
    let viewer = f
        .service
        .create_user(new_user("viewer", vec![Role::BasicUser]))
        .await
        .unwrap();
    let joe = f.service.create_member(new_member("Joe")).await.unwrap();
    let ann = f.service.create_member(new_member("Ann")).await.unwrap();

    f.service
        .grant_access(joe.id, viewer.id, AccessScope::HealthData, None)
        .await
        .unwrap();

    assert!(
        f.service
            .can_view_member_health_data(&viewer, joe.id)
            .await
            .unwrap()
    );
    assert!(
        !f.service
            .can_view_member_health_data(&viewer, ann.id)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn revoking_a_grant_closes_access_again() {
    let f = fixture();
    let viewer = f
        .service
        .create_user(new_user("viewer", vec![Role::BasicUser]))
        .await
        .unwrap();
    let member = f.service.create_member(new_member("Joe")).await.unwrap();

    f.service
        .grant_access(member.id, viewer.id, AccessScope::HealthData, None)
        .await
        .unwrap();
    f.service
        .revoke_access(member.id, viewer.id, AccessScope::HealthData)
        .await
        .unwrap();

    assert_eq!(f.grants.count(), 0);
    assert!(
        !f.service
            .can_view_member_health_data(&viewer, member.id)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn revoking_a_grant_that_was_never_made_is_not_found() {
    let f = fixture();
    let viewer = f
        .service
        .create_user(new_user("viewer", vec![Role::BasicUser]))
        .await
        .unwrap();
    let member = f.service.create_member(new_member("Joe")).await.unwrap();

    let err = f
        .service
        .revoke_access(member.id, viewer.id, AccessScope::HealthData)
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::NotFound { .. }));
}

#[tokio::test]
async fn granting_twice_does_not_duplicate() {
    let f = fixture();
    let viewer = f
        .service
        .create_user(new_user("viewer", vec![Role::BasicUser]))
        .await
        .unwrap();
    let member = f.service.create_member(new_member("Joe")).await.unwrap();

    for _ in 0..2 {
        f.service
            .grant_access(member.id, viewer.id, AccessScope::HealthData, None)
            .await
            .unwrap();
    }

    assert_eq!(f.grants.count(), 1);
    assert_eq!(
        f.service.list_member_access(member.id).await.unwrap().len(),
        1
    );
}

#[tokio::test]
async fn a_grant_needs_a_real_member_and_user() {
    let f = fixture();
    let member = f.service.create_member(new_member("Joe")).await.unwrap();

    let err = f
        .service
        .grant_access(member.id, UserId::new(), AccessScope::HealthData, None)
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::NotFound { .. }));
}

#[tokio::test]
async fn listing_members_hides_archived_by_default() {
    let f = fixture();
    let joe = f.service.create_member(new_member("Joe")).await.unwrap();
    f.service.create_member(new_member("Ann")).await.unwrap();
    f.service
        .set_member_archived(joe.id, joe.revision, true)
        .await
        .unwrap();

    let visible = f
        .service
        .list_members(&MemberQuery::default())
        .await
        .unwrap();
    assert_eq!(visible.total, 1);

    let all = f
        .service
        .list_members(&MemberQuery {
            include_archived: true,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(all.total, 2);
}

#[tokio::test]
async fn members_can_be_filtered_by_whether_they_have_an_account() {
    let f = fixture();
    let joe = f.service.create_member(new_member("Joe")).await.unwrap();
    f.service.create_member(new_member("Ann")).await.unwrap();
    let user = f
        .service
        .create_user(new_user("joe", vec![Role::BasicUser]))
        .await
        .unwrap();
    f.service
        .link_account(joe.id, joe.revision, user.id)
        .await
        .unwrap();

    let with = f
        .service
        .list_members(&MemberQuery {
            with_account: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(with.total, 1);
    assert_eq!(with.items[0].display_name, "Joe");

    let without = f
        .service
        .list_members(&MemberQuery {
            with_account: Some(false),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(without.total, 1);
    assert_eq!(without.items[0].display_name, "Ann");
}

#[tokio::test]
async fn users_can_be_filtered_by_role() {
    let f = fixture();
    an_admin(&f.service, "admin").await;
    f.service
        .create_user(new_user("joe", vec![Role::BasicUser]))
        .await
        .unwrap();

    let admins = f
        .service
        .list_users(&UserQuery {
            role: Some(Role::Admin),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(admins.total, 1);
    assert_eq!(admins.items[0].username, "admin");
}

#[tokio::test]
async fn a_missing_member_is_not_found() {
    let f = fixture();
    let err = f
        .service
        .get_member(HouseholdMemberId::new())
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::NotFound { .. }));
}
