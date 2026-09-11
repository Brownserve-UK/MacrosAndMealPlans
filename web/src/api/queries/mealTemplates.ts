import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { client, ifMatch, unwrap } from '../client';
import type { MealTemplate } from '../client';
import type { components } from '../schema';
import { mealTemplateKeys } from '../keys';
import type { MealTemplateListParams } from '../keys';

export function useMealTemplates(params: MealTemplateListParams) {
  return useQuery({
    queryKey: mealTemplateKeys.list(params),
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/meal-templates', { params: { query: params } })),
  });
}

export function useMealTemplate(id: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: mealTemplateKeys.one(id),
    enabled: options?.enabled ?? true,
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/meal-templates/{id}', { params: { path: { id } } })),
  });
}

export function useCreateMealTemplate() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: components['schemas']['CreateMealTemplateRequest']) =>
      unwrap(await client.POST('/api/v1/meal-templates', { body })),
    onSuccess: () => qc.invalidateQueries({ queryKey: mealTemplateKeys.all() }),
  });
}

export function useUpdateMealTemplate() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['UpdateMealTemplateRequest'];
    }) =>
      unwrap(
        await client.PATCH('/api/v1/meal-templates/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: (updated: MealTemplate) => {
      qc.setQueryData(mealTemplateKeys.one(updated.id), updated);
      void qc.invalidateQueries({ queryKey: mealTemplateKeys.all() });
    },
  });
}

export function useDeleteMealTemplate() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number }) =>
      unwrap(
        await client.DELETE('/api/v1/meal-templates/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: (_data, input) => {
      qc.removeQueries({ queryKey: mealTemplateKeys.one(input.id) });
      void qc.invalidateQueries({ queryKey: mealTemplateKeys.all() });
    },
  });
}

export function useCreateMealTemplateFromEntry() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { entryId: string; name: string }) =>
      unwrap(
        await client.POST('/api/v1/meal-templates/from-entry/{entry_id}', {
          params: { path: { entry_id: input.entryId } },
          body: { name: input.name },
        }),
      ),
    onSuccess: () => qc.invalidateQueries({ queryKey: mealTemplateKeys.all() }),
  });
}
