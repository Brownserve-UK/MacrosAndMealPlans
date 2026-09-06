import { useMutation, useQueryClient } from '@tanstack/react-query';
import { client, unwrap } from '../client';
import type { components } from '../schema';
import { mealPlanKeys, stockKeys } from '../keys';

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
