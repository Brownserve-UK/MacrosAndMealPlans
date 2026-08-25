use super::*;

fn new_user(username: &str) -> NewUser {
    NewUser {
        id: None,
        username: username.to_owned(),
        display_name: None,
        roles: vec![Role::BasicUser],
    }
}

fn new_member(name: &str) -> NewHouseholdMember {
    NewHouseholdMember {
        id: None,
        display_name: name.to_owned(),
        linked_user_id: None,
    }
}

#[test]
fn a_minimal_member_is_valid() {
    assert!(new_member("Joe").validate().is_ok());
}

#[test]
fn a_blank_member_name_is_rejected() {
    assert!(new_member("   ").validate().is_err());
}

#[test]
fn a_minimal_user_is_valid() {
    assert!(new_user("joe").validate().is_ok());
}

#[test]
fn a_username_with_spaces_is_rejected() {
    assert!(new_user("joe bloggs").validate().is_err());
}

#[test]
fn a_username_may_use_the_usual_punctuation() {
    assert!(new_user("joe.bloggs-1_x").validate().is_ok());
}

#[test]
fn a_one_character_username_is_rejected() {
    assert!(new_user("j").validate().is_err());
}

#[test]
fn an_over_long_username_is_rejected() {
    assert!(
        new_user(&"a".repeat(MAX_USERNAME_LEN + 1))
            .validate()
            .is_err()
    );
}

#[test]
fn a_user_needs_at_least_one_role() {
    let mut user = new_user("joe");
    user.roles = vec![];
    assert!(user.validate().is_err());
}

#[test]
fn empty_patches_are_detected() {
    assert!(HouseholdMemberPatch::default().is_empty());
    assert!(UserPatch::default().is_empty());
}

#[test]
fn clearing_a_display_name_is_not_an_empty_patch() {
    let patch = UserPatch {
        display_name: Patch::Clear,
        ..Default::default()
    };
    assert!(!patch.is_empty());
    assert!(patch.validate().is_ok());
}

#[test]
fn a_member_reports_whether_it_has_an_account() {
    let now = OffsetDateTime::now_utc();
    let mut member = HouseholdMember {
        id: HouseholdMemberId::new(),
        display_name: "Joe".to_owned(),
        linked_user_id: None,
        revision: Revision::INITIAL,
        created_at: now,
        updated_at: now,
        archived_at: None,
    };
    assert!(!member.has_account());
    member.linked_user_id = Some(UserId::new());
    assert!(member.has_account());
}
