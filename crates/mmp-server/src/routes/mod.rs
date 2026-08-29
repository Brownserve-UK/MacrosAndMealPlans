pub mod auth;
pub mod diary;
pub mod ingredients;
pub mod meal_plan;
pub mod members;
pub mod meta;
pub mod nutrition_target;
pub mod products;
pub mod recipes;
pub mod settings;
pub mod stock;
pub mod users;

use mmp_core::domain::HouseholdMemberId;

use crate::auth::{AuthError, Permission, Principal};
use crate::error::ApiResult;
use crate::state::AppState;

pub(crate) async fn require_member_access(
    state: &AppState,
    principal: &Principal,
    member: HouseholdMemberId,
) -> ApiResult<()> {
    let user = state.household.get_user(principal.user_id).await?;
    if state
        .household
        .can_view_member_health_data(&user, member)
        .await?
    {
        Ok(())
    } else {
        Err(AuthError::Forbidden(Permission::MemberHealthData).into())
    }
}
