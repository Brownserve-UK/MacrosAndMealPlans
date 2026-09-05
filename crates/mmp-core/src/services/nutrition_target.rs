use std::sync::Arc;

use super::revision::{commit_outcome, require_revision};
use crate::domain::{
    HouseholdMemberId, NewNutritionTarget, NutritionTarget, NutritionTargetId,
    NutritionTargetPatch, Revision, validate_goals,
};
use crate::error::{CoreError, Result, ValidationErrors};
use crate::ports::{Clock, NutritionTargetRepository};

const NUTRITION_TARGET: &str = "nutrition target";

#[derive(Clone)]
pub struct NutritionTargetService {
    targets: Arc<dyn NutritionTargetRepository>,
    clock: Arc<dyn Clock>,
}

impl NutritionTargetService {
    pub fn new(targets: Arc<dyn NutritionTargetRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { targets, clock }
    }

    pub async fn list(&self, member_id: HouseholdMemberId) -> Result<Vec<NutritionTarget>> {
        self.targets.list_for_member(member_id).await
    }

    pub async fn get(&self, id: NutritionTargetId) -> Result<NutritionTarget> {
        self.targets
            .get(id)
            .await?
            .ok_or_else(|| CoreError::not_found(NUTRITION_TARGET, id))
    }

    pub async fn create(&self, input: NewNutritionTarget) -> Result<NutritionTarget> {
        input.validate()?;
        let now = self.clock.now();
        let target = NutritionTarget {
            id: NutritionTargetId::new(),
            member_id: input.member_id,
            effective_from: input.effective_from,
            goals: input.goals,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
        };
        self.targets.insert(&target).await?;
        Ok(target)
    }

    pub async fn update(
        &self,
        id: NutritionTargetId,
        expected: Revision,
        patch: NutritionTargetPatch,
    ) -> Result<NutritionTarget> {
        let mut current = self.get(id).await?;
        require_revision(NUTRITION_TARGET, id, expected, current.revision)?;

        if patch.is_empty() {
            return Ok(current);
        }

        if let Some(effective_from) = patch.effective_from {
            current.effective_from = effective_from;
        }
        let goals = patch.goals.apply(current.goals);
        let mut errors = ValidationErrors::new();
        validate_goals(&goals, &mut errors);
        errors.into_result()?;
        current.goals = goals;

        current.revision = current.revision.next();
        current.updated_at = self.clock.now();
        commit_outcome(
            NUTRITION_TARGET,
            id,
            expected,
            self.targets.update(&current, expected).await?,
        )?;
        Ok(current)
    }

    pub async fn delete(&self, id: NutritionTargetId, expected: Revision) -> Result<()> {
        let current = self.get(id).await?;
        require_revision(NUTRITION_TARGET, id, expected, current.revision)?;
        commit_outcome(
            NUTRITION_TARGET,
            id,
            expected,
            self.targets.delete(id, expected).await?,
        )
    }
}

#[cfg(test)]
#[path = "nutrition_target_tests.rs"]
mod tests;
