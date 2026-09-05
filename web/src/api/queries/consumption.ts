import { useMutation, useQueryClient } from '@tanstack/react-query';
import { client, ifMatch, unwrap } from '../client';
import type { components } from '../schema';
import { mealPlanKeys } from '../keys';

export function useCreateConsumption() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: components['schemas']['CreateConsumptionRequest']) =>
      unwrap(await client.POST('/api/v1/consumption', { body })),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: mealPlanKeys.myWeeks() });
      void qc.invalidateQueries({ queryKey: mealPlanKeys.householdWeeks() });
    },
  });
}

export function useUpdateConsumption() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['UpdateConsumptionRequest'];
    }) =>
      unwrap(
        await client.PATCH('/api/v1/consumption/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: mealPlanKeys.myWeeks() });
      void qc.invalidateQueries({ queryKey: mealPlanKeys.householdWeeks() });
    },
  });
}

export function useDeleteConsumption() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; memberId: string }) =>
      unwrap(
        await client.DELETE('/api/v1/consumption/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: mealPlanKeys.myWeeks() });
      void qc.invalidateQueries({ queryKey: mealPlanKeys.householdWeeks() });
    },
  });
}
