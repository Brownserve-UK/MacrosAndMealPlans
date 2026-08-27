use async_trait::async_trait;
use mmp_core::Result;
use mmp_core::domain::{HouseholdSettings, Revision};
use mmp_core::ports::{HouseholdSettingsRepository, UpdateOutcome};
use sqlx::PgPool;

use crate::error::{map_db_error, repository_error};
use crate::rows::HouseholdSettingsRow;

const GET: &str = "SELECT breakfast_time, lunch_time, dinner_time, revision, created_at, updated_at \
     FROM household_settings WHERE singleton";
const CURRENT_REVISION: &str = "SELECT revision FROM household_settings WHERE singleton";

pub struct PgHouseholdSettingsRepository {
    pool: PgPool,
}

impl PgHouseholdSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HouseholdSettingsRepository for PgHouseholdSettingsRepository {
    async fn get(&self) -> Result<HouseholdSettings> {
        let row: Option<HouseholdSettingsRow> = sqlx::query_as(GET)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("loading household settings", e))?;
        row.map(Into::into).ok_or_else(|| {
            repository_error(
                "loading household settings",
                sqlx::Error::RowNotFound,
            )
        })
    }

    async fn update(
        &self,
        settings: &HouseholdSettings,
        expected: Revision,
    ) -> Result<UpdateOutcome> {
        let times = &settings.meal_times;
        let affected = sqlx::query(
            "UPDATE household_settings SET
                 breakfast_time = $1, lunch_time = $2, dinner_time = $3,
                 revision = $4, updated_at = $5
             WHERE singleton AND revision = $6",
        )
        .bind(times.breakfast)
        .bind(times.lunch)
        .bind(times.dinner)
        .bind(settings.revision.get())
        .bind(settings.updated_at)
        .bind(expected.get())
        .execute(&self.pool)
        .await
        .map_err(|e| map_db_error(e, "updating household settings"))?
        .rows_affected();

        if affected == 1 {
            return Ok(UpdateOutcome::Updated);
        }

        let current: Option<(i64,)> = sqlx::query_as(CURRENT_REVISION)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| repository_error("re-reading the household settings revision", e))?;

        Ok(match current {
            Some((actual,)) => UpdateOutcome::RevisionMismatch {
                actual: Revision::new(actual),
            },
            None => UpdateOutcome::NotFound,
        })
    }
}
