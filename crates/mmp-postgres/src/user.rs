use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{Revision, Role, User, UserId};
use mmp_core::ports::{Paginated, SortDirection, UpdateOutcome, UserQuery, UserRepository};
use sqlx::PgPool;

use crate::error::{map_db_error, repository_error};
use crate::rows::UserRow;

macro_rules! selection {
    () => {
        "SELECT u.id, u.username, u.display_name, u.auth_subject, \
         COALESCE(array_agg(r.role ORDER BY r.role) FILTER (WHERE r.role IS NOT NULL), '{}'::text[]) AS roles, \
         u.revision, u.created_at, u.updated_at, u.archived_at \
         FROM app_user u LEFT JOIN user_role r ON r.user_id = u.id"
    };
}

macro_rules! filter {
    () => {
        " WHERE ($1 OR u.archived_at IS NULL) \
          AND ($2::text IS NULL OR EXISTS ( \
              SELECT 1 FROM user_role f WHERE f.user_id = u.id AND f.role = $2)) \
          AND ($3::text IS NULL OR u.username ILIKE '%' || $3 || '%' \
               OR u.display_name ILIKE '%' || $3 || '%')"
    };
}

const GET_BY_ID: &str = concat!(selection!(), " WHERE u.id = $1 GROUP BY u.id");
const GET_BY_USERNAME: &str = concat!(
    selection!(),
    " WHERE lower(u.username) = lower($1) GROUP BY u.id"
);
const COUNT: &str = concat!("SELECT count(*) FROM app_user u", filter!());
const LIST_ASC: &str = concat!(
    selection!(),
    filter!(),
    " GROUP BY u.id ORDER BY lower(u.username) ASC LIMIT $4 OFFSET $5"
);
const LIST_DESC: &str = concat!(
    selection!(),
    filter!(),
    " GROUP BY u.id ORDER BY lower(u.username) DESC LIMIT $4 OFFSET $5"
);
const COUNT_WITH_ROLE: &str = "SELECT count(*) FROM app_user u \
     JOIN user_role r ON r.user_id = u.id \
     WHERE r.role = $1 AND ($2 OR u.archived_at IS NULL)";
const CURRENT_REVISION: &str = "SELECT revision FROM app_user WHERE id = $1";

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

async fn replace_roles(conn: &mut sqlx::PgConnection, user: &User) -> Result<()> {
    sqlx::query("DELETE FROM user_role WHERE user_id = $1")
        .bind(user.id.as_uuid())
        .execute(&mut *conn)
        .await
        .map_err(|e| repository_error("clearing user roles", e))?;

    if user.roles.is_empty() {
        return Ok(());
    }

    let codes: Vec<&str> = user.roles.iter().map(|r| r.code()).collect();
    sqlx::query(
        "INSERT INTO user_role (user_id, role) SELECT $1, unnest($2::text[]) \
         ON CONFLICT DO NOTHING",
    )
    .bind(user.id.as_uuid())
    .bind(&codes)
    .execute(&mut *conn)
    .await
    .map_err(|e| map_db_error(e, "assigning user roles"))?;

    Ok(())
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn get(&self, id: UserId) -> Result<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as(GET_BY_ID)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading a user", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as(GET_BY_USERNAME)
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("looking up a user by username", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list(&self, query: &UserQuery) -> Result<Paginated<User>> {
        let role = query.role.map(|r| r.code());
        let search = query.search.as_deref();
        let list_sql = match query.sort {
            SortDirection::Ascending => LIST_ASC,
            SortDirection::Descending => LIST_DESC,
        };

        let total: (i64,) = sqlx::query_as(COUNT)
            .bind(query.include_archived)
            .bind(role)
            .bind(search)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| repository_error("counting users", e))?;

        let rows: Vec<UserRow> = sqlx::query_as(list_sql)
            .bind(query.include_archived)
            .bind(role)
            .bind(search)
            .bind(query.page.limit())
            .bind(query.page.offset())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| repository_error("listing users", e))?;

        let items = rows
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<User>>>()?;
        Ok(Paginated::new(items, total.0, query.page))
    }

    async fn count_with_role(&self, role: Role, include_archived: bool) -> Result<i64> {
        let total: (i64,) = sqlx::query_as(COUNT_WITH_ROLE)
            .bind(role.code())
            .bind(include_archived)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| repository_error("counting users with a role", e))?;
        Ok(total.0)
    }

    async fn insert(&self, user: &User) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| repository_error("starting a transaction", e))?;

        sqlx::query(
            "INSERT INTO app_user (
                 id, username, display_name, auth_subject,
                 revision, created_at, updated_at, archived_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(user.id.as_uuid())
        .bind(&user.username)
        .bind(&user.display_name)
        .bind(&user.auth_subject)
        .bind(user.revision.get())
        .bind(user.created_at)
        .bind(user.updated_at)
        .bind(user.archived_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| map_db_error(e, "creating a user"))?;

        replace_roles(&mut tx, user).await?;

        tx.commit()
            .await
            .map_err(|e| repository_error("committing a new user", e))?;
        Ok(())
    }

    async fn update(&self, user: &User, expected: Revision) -> Result<UpdateOutcome> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| repository_error("starting a transaction", e))?;

        let affected = sqlx::query(
            "UPDATE app_user SET
                 username = $2, display_name = $3, auth_subject = $4,
                 revision = $5, updated_at = $6, archived_at = $7
             WHERE id = $1 AND revision = $8",
        )
        .bind(user.id.as_uuid())
        .bind(&user.username)
        .bind(&user.display_name)
        .bind(&user.auth_subject)
        .bind(user.revision.get())
        .bind(user.updated_at)
        .bind(user.archived_at)
        .bind(expected.get())
        .execute(&mut *tx)
        .await
        .map_err(|e| map_db_error(e, "updating a user"))?
        .rows_affected();

        if affected == 1 {
            replace_roles(&mut tx, user).await?;
            tx.commit()
                .await
                .map_err(|e| repository_error("committing a user update", e))?;
            return Ok(UpdateOutcome::Updated);
        }

        let current: Option<(i64,)> = sqlx::query_as(CURRENT_REVISION)
            .bind(user.id.as_uuid())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| repository_error("re-reading a user revision", e))?;

        Ok(match current {
            Some((actual,)) => UpdateOutcome::RevisionMismatch {
                actual: Revision::new(actual),
            },
            None => UpdateOutcome::NotFound,
        })
    }
}
