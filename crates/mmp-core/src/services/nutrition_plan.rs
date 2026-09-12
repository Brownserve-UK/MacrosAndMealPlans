use std::sync::Arc;

use rust_decimal::Decimal;

use super::revision::commit_outcome;
use super::{NutritionTargetService, WeightService};
use crate::domain::{
    CalorieCalculation, CalorieCalculationId, CalorieCalculationInput, HabitualActivity,
    HouseholdMemberId, MemberBodyProfile, NewNutritionTarget, NewWeightGoal, NewWeightRecord,
    NutritionGoals, NutritionTarget, NutritionTargetId, Pace, Patch, Quantity, Revision, Sex,
    TargetSource, Unit, UserId, WeightGoal, WeightObjective, WeightRecord, WeightSource, calculate,
    current_weight,
};
use crate::error::Result;
use crate::ports::{
    CalorieCalculationRepository, Clock, HouseholdSettingsRepository, MemberBodyProfileRepository,
};

const BODY_PROFILE: &str = "member body profile";
const CALORIE_CALCULATION: &str = "calorie target calculation";

#[derive(Debug, Clone, Copy)]
pub struct NutritionPlanAnswers {
    pub member_id: HouseholdMemberId,
    pub date_of_birth: time::Date,
    pub sex: Sex,
    pub height_cm: Decimal,
    pub current_weight: Quantity,
    pub habitual_activity: HabitualActivity,
    pub objective: WeightObjective,
    pub target_weight: Option<Quantity>,
    pub pace: Option<Pace>,
    pub recorded_by: Option<UserId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuidedNutritionPlan {
    pub profile: MemberBodyProfile,
    pub weight_record: Option<WeightRecord>,
    pub goal: WeightGoal,
    pub target: NutritionTarget,
    pub calculation: CalorieCalculation,
}

#[derive(Debug, Clone, Copy)]
struct CalculationRecord {
    target_id: NutritionTargetId,
    calculation_id: CalorieCalculationId,
    revision: Revision,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

#[derive(Clone)]
pub struct NutritionPlanService {
    profiles: Arc<dyn MemberBodyProfileRepository>,
    calculations: Arc<dyn CalorieCalculationRepository>,
    targets: NutritionTargetService,
    weight: WeightService,
    settings: Arc<dyn HouseholdSettingsRepository>,
    clock: Arc<dyn Clock>,
}

impl NutritionPlanService {
    pub fn new(
        profiles: Arc<dyn MemberBodyProfileRepository>,
        calculations: Arc<dyn CalorieCalculationRepository>,
        targets: NutritionTargetService,
        weight: WeightService,
        settings: Arc<dyn HouseholdSettingsRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            profiles,
            calculations,
            targets,
            weight,
            settings,
            clock,
        }
    }

    pub async fn body_profile(
        &self,
        member_id: HouseholdMemberId,
    ) -> Result<Option<MemberBodyProfile>> {
        self.profiles.for_member(member_id).await
    }

    pub async fn calculation_for_target(
        &self,
        target_id: NutritionTargetId,
    ) -> Result<Option<CalorieCalculation>> {
        self.calculations.for_target(target_id).await
    }

    pub async fn preview(&self, answers: NutritionPlanAnswers) -> Result<CalorieCalculation> {
        let today = self.today().await?;
        let now = self.clock.now();
        self.calculate_answers(
            answers,
            CalculationRecord {
                target_id: NutritionTargetId::new(),
                calculation_id: CalorieCalculationId::new(),
                revision: Revision::INITIAL,
                created_at: now,
                updated_at: now,
            },
            today,
        )
    }

    pub async fn set_guided(&self, answers: NutritionPlanAnswers) -> Result<GuidedNutritionPlan> {
        let today = self.today().await?;
        let now = self.clock.now();
        let preview = self.calculate_answers(
            answers,
            CalculationRecord {
                target_id: NutritionTargetId::new(),
                calculation_id: CalorieCalculationId::new(),
                revision: Revision::INITIAL,
                created_at: now,
                updated_at: now,
            },
            today,
        )?;

        let profile = self.set_profile(answers, today, now).await?;
        let weight_record = self.record_changed_weight(answers, today).await?;
        let goal = self
            .set_goal(answers, preview.applied_rate_kg_per_week, today)
            .await?;
        let goals = self
            .goals_with_energy(answers.member_id, today, preview.recommended_kcal)
            .await?;
        let target = self
            .targets
            .set_for_date(
                NewNutritionTarget {
                    member_id: answers.member_id,
                    effective_from: today,
                    goals,
                },
                TargetSource::Calculated,
            )
            .await?;
        let calculation = self.set_calculation(answers, &target, today, now).await?;

        Ok(GuidedNutritionPlan {
            profile,
            weight_record,
            goal,
            target,
            calculation,
        })
    }

    pub async fn set_manual(
        &self,
        member_id: HouseholdMemberId,
        energy_kcal: Decimal,
    ) -> Result<NutritionTarget> {
        let today = self.today().await?;
        let goals = self
            .goals_with_energy(member_id, today, energy_kcal)
            .await?;
        let target = self
            .targets
            .set_for_date(
                NewNutritionTarget {
                    member_id,
                    effective_from: today,
                    goals,
                },
                TargetSource::UserDefined,
            )
            .await?;
        if let Some(calculation) = self.calculations.for_target(target.id).await? {
            commit_outcome(
                CALORIE_CALCULATION,
                calculation.id,
                calculation.revision,
                self.calculations
                    .delete(calculation.id, calculation.revision)
                    .await?,
            )?;
        }
        Ok(target)
    }

    async fn goals_with_energy(
        &self,
        member_id: HouseholdMemberId,
        today: time::Date,
        energy_kcal: Decimal,
    ) -> Result<NutritionGoals> {
        let mut goals = self
            .targets
            .list(member_id)
            .await?
            .into_iter()
            .find(|target| target.effective_from == today)
            .map_or_else(NutritionGoals::default, |target| target.goals);
        goals.energy_kcal = Some(energy_kcal);
        Ok(goals)
    }

    async fn today(&self) -> Result<time::Date> {
        Ok(
            super::calendar::household_calendar(&*self.settings, &self.clock)
                .await?
                .today(),
        )
    }

    fn calculate_answers(
        &self,
        answers: NutritionPlanAnswers,
        record: CalculationRecord,
        today: time::Date,
    ) -> Result<CalorieCalculation> {
        let weight_kg = NewWeightRecord {
            member_id: answers.member_id,
            weight: answers.current_weight,
            recorded_on: today,
            recorded_at: None,
            source: WeightSource::Manual,
            recorded_by: answers.recorded_by,
        }
        .weight_kg()?;
        NewWeightGoal {
            member_id: answers.member_id,
            objective: answers.objective,
            starting_weight: answers.current_weight,
            target_weight: answers.target_weight,
            planned_rate: answers
                .pace
                .map(|pace| pace.rate_for(answers.objective))
                .transpose()?
                .map(kilograms),
            started_on: today,
        }
        .validate()?;

        calculate(CalorieCalculationInput {
            id: record.calculation_id,
            member_id: answers.member_id,
            nutrition_target_id: record.target_id,
            calculated_on: today,
            date_of_birth: answers.date_of_birth,
            sex: answers.sex,
            height_cm: answers.height_cm,
            weight_kg,
            habitual_activity: answers.habitual_activity,
            objective: answers.objective,
            pace: answers.pace,
            revision: record.revision,
            created_at: record.created_at,
            updated_at: record.updated_at,
        })
    }

    async fn set_profile(
        &self,
        answers: NutritionPlanAnswers,
        today: time::Date,
        now: time::OffsetDateTime,
    ) -> Result<MemberBodyProfile> {
        let existing = self.profiles.for_member(answers.member_id).await?;
        let mut profile = existing.clone().unwrap_or(MemberBodyProfile {
            member_id: answers.member_id,
            date_of_birth: None,
            sex: None,
            height_cm: None,
            habitual_activity: None,
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
        });
        profile.date_of_birth = Some(answers.date_of_birth);
        profile.sex = Some(answers.sex);
        profile.height_cm = Some(answers.height_cm.round_dp(1));
        profile.habitual_activity = Some(answers.habitual_activity);
        profile.validate(today)?;

        if let Some(current) = existing {
            profile.revision = current.revision.next();
            profile.updated_at = now;
            commit_outcome(
                BODY_PROFILE,
                profile.member_id,
                current.revision,
                self.profiles.update(&profile, current.revision).await?,
            )?;
        } else {
            self.profiles.insert(&profile).await?;
        }
        Ok(profile)
    }

    async fn record_changed_weight(
        &self,
        answers: NutritionPlanAnswers,
        today: time::Date,
    ) -> Result<Option<WeightRecord>> {
        let records = self.weight.list_records(answers.member_id).await?;
        let entered_kg = NewWeightRecord {
            member_id: answers.member_id,
            weight: answers.current_weight,
            recorded_on: today,
            recorded_at: None,
            source: WeightSource::Manual,
            recorded_by: answers.recorded_by,
        }
        .weight_kg()?;
        if current_weight(&records).is_some_and(|record| record.weight_kg == entered_kg) {
            return Ok(None);
        }
        self.weight
            .record(NewWeightRecord {
                member_id: answers.member_id,
                weight: answers.current_weight,
                recorded_on: today,
                recorded_at: None,
                source: WeightSource::Manual,
                recorded_by: answers.recorded_by,
            })
            .await
            .map(Some)
    }

    async fn set_goal(
        &self,
        answers: NutritionPlanAnswers,
        applied_rate: Option<Decimal>,
        today: time::Date,
    ) -> Result<WeightGoal> {
        let input = NewWeightGoal {
            member_id: answers.member_id,
            objective: answers.objective,
            starting_weight: answers.current_weight,
            target_weight: answers.target_weight,
            planned_rate: applied_rate.map(kilograms),
            started_on: today,
        };
        if let Some(existing) = self.weight.goal(answers.member_id).await? {
            self.weight
                .update_goal(
                    existing.id,
                    existing.revision,
                    crate::domain::WeightGoalPatch {
                        objective: Some(input.objective),
                        starting_weight: Some(input.starting_weight),
                        target_weight: input.target_weight.map_or(Patch::Clear, Patch::Set),
                        planned_rate: input.planned_rate.map_or(Patch::Clear, Patch::Set),
                        started_on: Some(today),
                    },
                )
                .await
        } else {
            self.weight.set_goal(input).await
        }
    }

    async fn set_calculation(
        &self,
        answers: NutritionPlanAnswers,
        target: &NutritionTarget,
        today: time::Date,
        now: time::OffsetDateTime,
    ) -> Result<CalorieCalculation> {
        let existing = self.calculations.for_target(target.id).await?;
        let calculation = self.calculate_answers(
            answers,
            CalculationRecord {
                target_id: target.id,
                calculation_id: existing
                    .as_ref()
                    .map_or_else(CalorieCalculationId::new, |current| current.id),
                revision: existing
                    .as_ref()
                    .map_or(Revision::INITIAL, |current| current.revision.next()),
                created_at: existing.as_ref().map_or(now, |current| current.created_at),
                updated_at: now,
            },
            today,
        )?;
        if let Some(current) = existing {
            commit_outcome(
                CALORIE_CALCULATION,
                current.id,
                current.revision,
                self.calculations
                    .update(&calculation, current.revision)
                    .await?,
            )?;
        } else {
            self.calculations.insert(&calculation).await?;
        }
        Ok(calculation)
    }
}

fn kilograms(amount: Decimal) -> Quantity {
    Quantity::new(amount, Unit::Kilogram)
}

#[cfg(test)]
#[path = "nutrition_plan_tests.rs"]
mod tests;
