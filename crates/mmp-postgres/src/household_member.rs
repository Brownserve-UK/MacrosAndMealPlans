use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{HouseholdMember, HouseholdMemberId, Revision, UserId};
use mmp_core::ports::{
    HouseholdMemberRepository, MemberQuery, Paginated, SortDirection, UpdateOutcome,
};
use sqlx::PgPool;

use crate::error::{map_db_error, repository_error};
use crate::rows::HouseholdMemberRow;

macro_rules! columns {
    () => {
        "id, display_name, linked_user_id, revision, created_at, updated_at, archived_at"
    };
}

macro_rules! filter {
    () => {
        " WHERE ($1 OR archived_at IS NULL) \
          AND ($2::bool IS NULL OR (linked_user_id IS NOT NULL) = $2) \
          AND ($3::text IS NULL OR display_name ILIKE '%' || $3 || '%')"
    };
}

const GET_BY_ID: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM household_member WHERE id = $1"
);
const GET_BY_NAME: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM household_member WHERE lower(display_name) = lower($1)"
);
const GET_BY_LINKED_USER: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM household_member WHERE linked_user_id = $1"
);
const COUNT: &str = concat!("SELECT count(*) FROM household_member", filter!());
const LIST_ASC: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM household_member",
    filter!(),
    " ORDER BY lower(display_name) ASC LIMIT $4 OFFSET $5"
);
const LIST_DESC: &str = concat!(
    "SELECT ",
    columns!(),
    " FROM household_member",
    filter!(),
    " ORDER BY lower(display_name) DESC LIMIT $4 OFFSET $5"
);
const CURRENT_REVISION: &str = "SELECT revision FROM household_member WHERE id = $1";

pub struct PgHouseholdMemberRepository {
    pool: PgPool,
}

impl PgHouseholdMemberRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HouseholdMemberRepository for PgHouseholdMemberRepository {
    async fn get(&self, id: HouseholdMemberId) -> Result<Option<HouseholdMember>> {
        let row: Option<HouseholdMemberRow> = sqlx::query_as(GET_BY_ID)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a household member", e))?;
        Ok(row.map(Into::into))
    }

    async fn find_by_display_name(&self, name: &str) -> Result<Option<HouseholdMember>> {
        let row: Option<HouseholdMemberRow> = sqlx::query_as(GET_BY_NAME)
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("looking up a household member by name", e))?;
        Ok(row.map(Into::into))
    }

    async fn find_by_linked_user(&self, user_id: UserId) -> Result<Option<HouseholdMember>> {
        let row: Option<HouseholdMemberRow> = sqlx::query_as(GET_BY_LINKED_USER)
            .bind(user_id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("looking up a household member by account", e))?;
        Ok(row.map(Into::into))
    }

    async fn list(&self, query: &MemberQuery) -> Result<Paginated<HouseholdMember>> {
        let search = query.search.as_deref();
        let list_sql = match query.sort {
            SortDirection::Ascending => LIST_ASC,
            SortDirection::Descending => LIST_DESC,
        };

        let total: (i64,) = sqlx::query_as(COUNT)
            .bind(query.include_archived)
            .bind(query.with_account)
            .bind(search)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| repository_error("counting household members", e))?;

        let rows: Vec<HouseholdMemberRow> = sqlx::query_as(list_sql)
            .bind(query.include_archived)
            .bind(query.with_account)
            .bind(search)
            .bind(query.page.limit())
            .bind(query.page.offset())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing household members", e))?;

        let items = rows.into_iter().map(Into::into).collect();
        Ok(Paginated::new(items, total.0, query.page))
    }

    async fn insert(&self, member: &HouseholdMember) -> Result<()> {
        sqlx::query(
            "INSERT INTO household_member (
                 id, display_name, linked_user_id, revision, created_at, updated_at, archived_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(member.id.as_uuid())
        .bind(&member.display_name)
        .bind(member.linked_user_id.map(|id| id.as_uuid()))
        .bind(member.revision.get())
        .bind(member.created_at)
        .bind(member.updated_at)
        .bind(member.archived_at)
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "creating a household member"))?;
        Ok(())
    }

    async fn update(&self, member: &HouseholdMember, expected: Revision) -> Result<UpdateOutcome> {
        let affected = sqlx::query(
            "UPDATE household_member SET
                 display_name = $2, linked_user_id = $3,
                 revision = $4, updated_at = $5, archived_at = $6
             WHERE id = $1 AND revision = $7",
        )
        .bind(member.id.as_uuid())
        .bind(&member.display_name)
        .bind(member.linked_user_id.map(|id| id.as_uuid()))
        .bind(member.revision.get())
        .bind(member.updated_at)
        .bind(member.archived_at)
        .bind(expected.get())
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "updating a household member"))?
        .rows_affected();

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }

        let current: Option<(i64,)> = sqlx::query_as(CURRENT_REVISION)
            .bind(member.id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("re-reading a household member revision", e))?;

        Ok(match current {
            Some((actual,)) => UpdateOutcome::RevisionMismatch {
                actual: Revision::new(actual),
            },
            None => UpdateOutcome::NotFound,
        })
    }
}
