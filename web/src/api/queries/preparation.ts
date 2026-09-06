import { useMutation, useQueryClient } from '@tanstack/react-query';
import { client, unwrap } from '../client';
import type { components } from '../schema';
import { mealPlanKeys, stockKeys } from '../keys';

export function usePlacePortions() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      body: components['schemas']['PlacePortionsRequest'];
    }) =>
      unwrap(
        await client.PUT('/api/v1/preparations/{id}/placements', {
          params: { path: { id: input.id } },
          body: input.body,
        }),
      ),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: mealPlanKeys.myWeeks() });
      void qc.invalidateQueries({ queryKey: mealPlanKeys.householdWeeks() });
      void qc.invalidateQueries({ queryKey: stockKeys.all() });
    },
  });
}

export function useRecordPreparation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: components['schemas']['RecordPreparationRequest']) =>
      unwrap(await client.POST('/api/v1/preparations', { body })),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: mealPlanKeys.myWeeks() });
      void qc.invalidateQueries({ queryKey: mealPlanKeys.householdWeeks() });
      void qc.invalidateQueries({ queryKey: mealPlanKeys.needsReview() });
      void qc.invalidateQueries({ queryKey: stockKeys.all() });
    },
  });
}
