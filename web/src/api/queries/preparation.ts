import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { client, ifMatch, unwrap } from '../client';
import type { components } from '../schema';
import { mealPlanKeys, preparationKeys, stockKeys } from '../keys';

export function useCooks(from: string, to: string) {
  return useQuery({
    queryKey: preparationKeys.range(from, to),
    enabled: Boolean(from && to),
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/preparations', { params: { query: { from, to } } })),
  });
}

export function useMoveCookedFood() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      recipeId: string;
      body: components['schemas']['MoveCookedFoodRequest'];
    }) =>
      unwrap(
        await client.POST('/api/v1/cooked-food/{recipe_id}/move', {
          params: { path: { recipe_id: input.recipeId } },
          body: input.body,
        }),
      ),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: mealPlanKeys.myWeeks() });
      void qc.invalidateQueries({ queryKey: mealPlanKeys.householdWeeks() });
      void qc.invalidateQueries({ queryKey: stockKeys.all() });
      void qc.invalidateQueries({ queryKey: preparationKeys.all() });
    },
  });
}

export function usePlacePortions() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['PlacePortionsRequest'];
    }) =>
      unwrap(
        await client.PUT('/api/v1/preparations/{id}/placements', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: mealPlanKeys.myWeeks() });
      void qc.invalidateQueries({ queryKey: mealPlanKeys.householdWeeks() });
      void qc.invalidateQueries({ queryKey: stockKeys.all() });
      void qc.invalidateQueries({ queryKey: preparationKeys.all() });
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
      void qc.invalidateQueries({ queryKey: preparationKeys.all() });
    },
  });
}
