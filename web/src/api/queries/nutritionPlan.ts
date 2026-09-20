import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { client, ifMatch, unwrap } from '../client';
import { mealPlanKeys, nutritionPlanKeys, weightKeys } from '../keys';
import type { components } from '../schema';

function useNutritionPlanInvalidation() {
  const qc = useQueryClient();
  return (memberId: string) => {
    void qc.invalidateQueries({ queryKey: nutritionPlanKeys.plan(memberId) });
    void qc.invalidateQueries({ queryKey: nutritionPlanKeys.profile(memberId) });
    void qc.invalidateQueries({ queryKey: weightKeys.summary(memberId) });
    void qc.invalidateQueries({ queryKey: weightKeys.records(memberId) });
    void qc.invalidateQueries({ queryKey: weightKeys.goal(memberId) });
    void qc.invalidateQueries({ queryKey: mealPlanKeys.myWeeks() });
    void qc.invalidateQueries({ queryKey: mealPlanKeys.householdWeeks() });
  };
}

export function useNutritionPlan(memberId: string) {
  return useQuery({
    queryKey: nutritionPlanKeys.plan(memberId),
    enabled: Boolean(memberId),
    queryFn: async () =>
      unwrap(
        await client.GET('/api/v1/members/{member_id}/nutrition-plan', {
          params: { path: { member_id: memberId } },
        }),
      ),
  });
}

export function useBodyProfile(memberId: string) {
  return useQuery({
    queryKey: nutritionPlanKeys.profile(memberId),
    enabled: Boolean(memberId),
    queryFn: async () =>
      unwrap(
        await client.GET('/api/v1/members/{member_id}/body-profile', {
          params: { path: { member_id: memberId } },
        }),
      ),
  });
}

export function useUpdateBodyProfile() {
  const invalidate = useNutritionPlanInvalidation();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['UpdateBodyProfileRequest'];
    }) =>
      unwrap(
        await client.PUT('/api/v1/members/{member_id}/body-profile', {
          params: { path: { member_id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: (profile) => invalidate(profile.member_id),
  });
}

export function usePreviewCalorieTarget() {
  return useMutation({
    mutationFn: async (input: {
      id: string;
      body: components['schemas']['NutritionPlanAnswersRequest'];
    }) =>
      unwrap(
        await client.POST('/api/v1/members/{member_id}/calorie-target/preview', {
          params: { path: { member_id: input.id } },
          body: input.body,
        }),
      ),
  });
}

export function useSetGuidedCalorieTarget() {
  const invalidate = useNutritionPlanInvalidation();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      body: components['schemas']['NutritionPlanAnswersRequest'];
    }) =>
      unwrap(
        await client.PUT('/api/v1/members/{member_id}/calorie-target/guided', {
          params: { path: { member_id: input.id } },
          body: input.body,
        }),
      ),
    onSuccess: (plan) => invalidate(plan.target.member_id),
  });
}

export function useSetManualCalorieTarget() {
  const invalidate = useNutritionPlanInvalidation();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      body: components['schemas']['ManualCalorieTargetRequest'];
    }) =>
      unwrap(
        await client.PUT('/api/v1/members/{member_id}/calorie-target/manual', {
          params: { path: { member_id: input.id } },
          body: input.body,
        }),
      ),
    onSuccess: (target) => invalidate(target.member_id),
  });
}
