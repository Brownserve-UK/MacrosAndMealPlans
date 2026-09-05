import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { client, ifMatch, unwrap } from '../client';
import type { components } from '../schema';
import { settingsKeys } from '../keys';

export function useUnits() {
  return useQuery({
    queryKey: settingsKeys.units(),
    staleTime: Infinity,
    queryFn: async () => unwrap(await client.GET('/api/v1/units')),
  });
}

export function useMeta() {
  return useQuery({
    queryKey: settingsKeys.meta(),
    staleTime: Infinity,
    queryFn: async () => unwrap(await client.GET('/api/v1/meta')),
  });
}

export function useMealTimes() {
  return useQuery({
    queryKey: settingsKeys.mealTimes(),
    queryFn: async () => unwrap(await client.GET('/api/v1/household/meal-times')),
  });
}

export function useUpdateMealTimes() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      revision: number;
      body: components['schemas']['UpdateMealTimesRequest'];
    }) =>
      unwrap(
        await client.PUT('/api/v1/household/meal-times', {
          params: { header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: (updated) => {
      qc.setQueryData(settingsKeys.mealTimes(), updated);
    },
  });
}
