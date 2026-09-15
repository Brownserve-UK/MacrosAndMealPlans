use std::sync::Arc;

use rust_decimal::Decimal;
use time::Date;

use super::revision::{commit_outcome, require_revision};
use crate::domain::{
    GoalProjection, HouseholdMemberId, NewWeightGoal, NewWeightRecord, Revision, WeightGoal,
    WeightGoalId, WeightGoalPatch, WeightRecord, WeightRecordId, WeightRecordPatch, current_weight,
    latest_per_day, project_goal,
};
use crate::error::{CoreError, Result};
use crate::ports::{
    Clock, HouseholdSettingsRepository, WeightGoalRepository, WeightRecordRepository,
};

const WEIGHT_RECORD: &str = "weight record";
const WEIGHT_GOAL: &str = "weight goal";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeightPoint {
    pub on: Date,
    pub weight_kg: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightSummary {
    pub latest: Option<WeightRecord>,
    pub goal: Option<WeightGoal>,
    pub projection: Option<GoalProjection>,
    pub change_since_start_kg: Option<Decimal>,
    pub series: Vec<WeightPoint>,
}

#[derive(Clone)]
pub struct WeightService {
    records: Arc<dyn WeightRecordRepository>,
    goals: Arc<dyn WeightGoalRepository>,
    settings: Arc<dyn HouseholdSettingsRepository>,
    clock: Arc<dyn Clock>,
}

impl WeightService {
    pub fn new(
        records: Arc<dyn WeightRecordRepository>,
        goals: Arc<dyn WeightGoalRepository>,
        settings: Arc<dyn HouseholdSettingsRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            records,
            goals,
            settings,
            clock,
        }
    }

    pub async fn list_records(&self, member_id: HouseholdMemberId) -> Result<Vec<WeightRecord>> {
        self.records.list_for_member(member_id).await
    }

    pub async fn list_records_filtered(
        &self,
        member_id: HouseholdMemberId,
        from: Option<Date>,
        limit: Option<u32>,
    ) -> Result<Vec<WeightRecord>> {
        self.records
            .list_for_member_filtered(member_id, from, limit)
            .await
    }

    pub async fn get_record(&self, id: WeightRecordId) -> Result<WeightRecord> {
        self.records
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(WEIGHT_RECORD, id))
    }

    pub async fn record(&self, input: NewWeightRecord) -> Result<WeightRecord> {
        let weight_kg = input.weight_kg()?;
        let now = self.clock.now();
        let record = WeightRecord {
            id: WeightRecordId::new(),
            member_id: input.member_id,
            weight_kg,
            recorded_on: input.recorded_on,
            recorded_at: input.recorded_at,
            source: input.source,
            recorded_by: input.recorded_by,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
        };
        self.records.insert(&record).await?;
        Ok(record)
    }

    pub async fn update_record(
        &self,
        id: WeightRecordId,
        expected: Revision,
        patch: WeightRecordPatch,
    ) -> Result<WeightRecord> {
        let mut current = self.get_record(id).await?;
        require_revision(WEIGHT_RECORD, id, expected, current.revision)?;

        if patch.is_empty() {
            return Ok(current);
        }

        if let Some(weight) = patch.weight {
            current.weight_kg = NewWeightRecord {
                member_id: current.member_id,
                weight,
                recorded_on: current.recorded_on,
                recorded_at: current.recorded_at,
                source: current.source,
                recorded_by: current.recorded_by,
            }
            .weight_kg()?;
        }
        if let Some(recorded_on) = patch.recorded_on {
            current.recorded_on = recorded_on;
        }
        current.recorded_at = patch.recorded_at.apply(current.recorded_at);

        current.revision = current.revision.next();
        current.updated_at = self.clock.now();
        commit_outcome(
            WEIGHT_RECORD,
            id,
            expected,
            self.records.update(&current, expected).await?,
        )?;
        Ok(current)
    }

    pub async fn delete_record(&self, id: WeightRecordId, expected: Revision) -> Result<()> {
        let current = self.get_record(id).await?;
        require_revision(WEIGHT_RECORD, id, expected, current.revision)?;
        commit_outcome(
            WEIGHT_RECORD,
            id,
            expected,
            self.records.delete(id, expected).await?,
        )
    }

    pub async fn goal(&self, member_id: HouseholdMemberId) -> Result<Option<WeightGoal>> {
        self.goals.for_member(member_id).await
    }

    pub async fn get_goal(&self, id: WeightGoalId) -> Result<WeightGoal> {
        self.goals
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(WEIGHT_GOAL, id))
    }

    pub async fn set_goal(&self, input: NewWeightGoal) -> Result<WeightGoal> {
        let amounts = input.resolve()?;
        let now = self.clock.now();
        let goal = WeightGoal {
            id: WeightGoalId::new(),
            member_id: input.member_id,
            objective: input.objective,
            starting_weight_kg: amounts.starting_kg,
            target_weight_kg: amounts.target_kg,
            planned_rate_kg_per_week: amounts.rate_kg_per_week,
            started_on: input.started_on,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
        };
        self.goals.insert(&goal).await?;
        Ok(goal)
    }

    pub async fn update_goal(
        &self,
        id: WeightGoalId,
        expected: Revision,
        patch: WeightGoalPatch,
    ) -> Result<WeightGoal> {
        let mut current = self.get_goal(id).await?;
        require_revision(WEIGHT_GOAL, id, expected, current.revision)?;

        if patch.is_empty() {
            return Ok(current);
        }

        let objective = patch.objective.unwrap_or(current.objective);
        let candidate = NewWeightGoal {
            member_id: current.member_id,
            objective,
            starting_weight: patch
                .starting_weight
                .unwrap_or_else(|| kilograms(current.starting_weight_kg)),
            target_weight: patch
                .target_weight
                .apply(current.target_weight_kg.map(kilograms)),
            planned_rate: patch
                .planned_rate
                .apply(current.planned_rate_kg_per_week.map(kilograms)),
            started_on: patch.started_on.unwrap_or(current.started_on),
        };
        let amounts = candidate.resolve()?;

        current.objective = objective;
        current.starting_weight_kg = amounts.starting_kg;
        current.target_weight_kg = amounts.target_kg;
        current.planned_rate_kg_per_week = amounts.rate_kg_per_week;
        current.started_on = candidate.started_on;

        current.revision = current.revision.next();
        current.updated_at = self.clock.now();
        commit_outcome(
            WEIGHT_GOAL,
            id,
            expected,
            self.goals.update(&current, expected).await?,
        )?;
        Ok(current)
    }

    pub async fn clear_goal(&self, id: WeightGoalId, expected: Revision) -> Result<()> {
        let current = self.get_goal(id).await?;
        require_revision(WEIGHT_GOAL, id, expected, current.revision)?;
        commit_outcome(
            WEIGHT_GOAL,
            id,
            expected,
            self.goals.delete(id, expected).await?,
        )
    }

    pub async fn summary(&self, member_id: HouseholdMemberId) -> Result<WeightSummary> {
        self.summary_since(member_id, None).await
    }

    pub async fn summary_since(
        &self,
        member_id: HouseholdMemberId,
        from: Option<Date>,
    ) -> Result<WeightSummary> {
        let records = self
            .records
            .list_for_member_filtered(member_id, from, None)
            .await?;
        let latest = if from.is_some() {
            self.records
                .list_for_member_filtered(member_id, None, Some(1))
                .await?
                .into_iter()
                .next()
        } else {
            current_weight(&records).copied()
        };
        let goal = self.goals.for_member(member_id).await?;

        let series: Vec<WeightPoint> = latest_per_day(&records)
            .into_iter()
            .map(|record| WeightPoint {
                on: record.recorded_on,
                weight_kg: record.weight_kg,
            })
            .collect();
        let today = super::calendar::household_calendar(&*self.settings, &self.clock)
            .await?
            .today();
        let projection = goal
            .as_ref()
            .map(|goal| project_goal(goal, latest.map(|record| record.weight_kg), today));
        let change_since_start_kg = goal
            .as_ref()
            .and_then(|goal| latest.map(|record| record.weight_kg - goal.starting_weight_kg));

        Ok(WeightSummary {
            latest,
            goal,
            projection,
            change_since_start_kg,
            series,
        })
    }
}

fn kilograms(amount: Decimal) -> crate::domain::Quantity {
    crate::domain::Quantity::new(amount, crate::domain::Unit::Kilogram)
}

#[cfg(test)]
#[path = "weight_tests.rs"]
mod tests;
