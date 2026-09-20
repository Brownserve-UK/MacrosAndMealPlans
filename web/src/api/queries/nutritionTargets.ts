import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { client, ifMatch, unwrap } from '../client';
import type { components } from '../schema';
import { mealPlanKeys, nutritionTargetKeys } from '../keys';

export function useNutritionTargets(memberId: string) {
  return useQuery({
    queryKey: nutritionTargetKeys.forMember(memberId),
    enabled: Boolean(memberId),
    queryFn: async () =>
      unwrap(
        await client.GET('/api/v1/members/{member_id}/nutrition-targets', {
          params: { path: { member_id: memberId } },
        }),
      ),
  });
}

function useNutritionTargetInvalidation() {
  const qc = useQueryClient();
  return (memberId: string) => {
    void qc.invalidateQueries({ queryKey: nutritionTargetKeys.forMember(memberId) });
    void qc.invalidateQueries({ queryKey: mealPlanKeys.myWeeks() });
    void qc.invalidateQueries({ queryKey: mealPlanKeys.householdWeeks() });
  };
}

export function useCreateNutritionTarget() {
  const invalidate = useNutritionTargetInvalidation();
  return useMutation({
    mutationFn: async (input: {
      memberId: string;
      body: components['schemas']['CreateNutritionTargetRequest'];
    }) =>
      unwrap(
        await client.POST('/api/v1/members/{member_id}/nutrition-targets', {
          params: { path: { member_id: input.memberId } },
          body: input.body,
        }),
      ),
    onSuccess: (target) => invalidate(target.member_id),
  });
}

export function useUpdateNutritionTarget() {
  const invalidate = useNutritionTargetInvalidation();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['UpdateNutritionTargetRequest'];
    }) =>
      unwrap(
        await client.PATCH('/api/v1/nutrition-targets/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: (target) => invalidate(target.member_id),
  });
}

export function useDeleteNutritionTarget() {
  const invalidate = useNutritionTargetInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; memberId: string }) =>
      unwrap(
        await client.DELETE('/api/v1/nutrition-targets/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: (_data, variables) => invalidate(variables.memberId),
  });
}
