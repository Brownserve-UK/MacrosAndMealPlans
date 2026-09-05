import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { client, ifMatch, unwrap } from '../client';
import type { components } from '../schema';
import { mealPlanKeys } from '../keys';

export function useNeedsReview() {
  return useQuery({
    queryKey: mealPlanKeys.needsReview(),
    queryFn: async () => unwrap(await client.GET('/api/v1/needs-review', {})),
  });
}

export function useMealPlanWeek(weekStart: string) {
  return useQuery({
    queryKey: mealPlanKeys.myWeek(weekStart),
    enabled: Boolean(weekStart),
    queryFn: async () =>
      unwrap(
        await client.GET('/api/v1/meal-plan/{week_start}', {
          params: { path: { week_start: weekStart } },
        }),
      ),
  });
}

export function useHouseholdPlannerWeek(weekStart: string) {
  return useQuery({
    queryKey: mealPlanKeys.householdWeek(weekStart),
    enabled: Boolean(weekStart),
    queryFn: async () =>
      unwrap(
        await client.GET('/api/v1/planner/{week_start}', {
          params: { path: { week_start: weekStart } },
        }),
      ),
  });
}

function useMealPlanInvalidation() {
  const qc = useQueryClient();
  return () => {
    void qc.invalidateQueries({ queryKey: mealPlanKeys.myWeeks() });
    void qc.invalidateQueries({ queryKey: mealPlanKeys.householdWeeks() });
    void qc.invalidateQueries({ queryKey: mealPlanKeys.slotAttendances() });
    void qc.invalidateQueries({ queryKey: mealPlanKeys.needsReview() });
  };
}

export function useCreateMealPlanEntry() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (body: components['schemas']['CreateMealPlanEntryRequest']) =>
      unwrap(await client.POST('/api/v1/meal-plan-entries', { body })),
    onSuccess: () => invalidate(),
  });
}

export function useUpdateMealPlanEntry() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['UpdateMealPlanEntryRequest'];
    }) =>
      unwrap(
        await client.PATCH('/api/v1/meal-plan-entries/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: () => invalidate(),
  });
}

export function useDeleteMealPlanEntry() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number }) =>
      unwrap(
        await client.DELETE('/api/v1/meal-plan-entries/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: () => invalidate(),
  });
}

export function useMarkMealPlanEaten() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['MarkMealPlanEatenRequest'];
    }) =>
      unwrap(
        await client.POST('/api/v1/meal-plan-entries/{id}/eaten', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: () => invalidate(),
  });
}

export function useMarkMealPlanNotEaten() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number }) =>
      unwrap(
        await client.POST('/api/v1/meal-plan-entries/{id}/not-eaten', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: () => invalidate(),
  });
}

export function useMarkMealPlanComponentEaten() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      componentId: string;
      revision: number;
      body: components['schemas']['MarkMealPlanComponentEatenRequest'];
    }) =>
      unwrap(
        await client.POST('/api/v1/meal-plan-entries/{id}/components/{component_id}/eaten', {
          params: {
            path: { id: input.id, component_id: input.componentId },
            header: ifMatch(input.revision),
          },
          body: input.body,
        }),
      ),
    onSuccess: () => invalidate(),
  });
}

export function useMarkMealPlanComponentNotEaten() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; componentId: string; revision: number }) =>
      unwrap(
        await client.POST('/api/v1/meal-plan-entries/{id}/components/{component_id}/not-eaten', {
          params: {
            path: { id: input.id, component_id: input.componentId },
            header: ifMatch(input.revision),
          },
        }),
      ),
    onSuccess: () => invalidate(),
  });
}

export function useReopenMealPlanComponent() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; componentId: string; revision: number }) =>
      unwrap(
        await client.POST('/api/v1/meal-plan-entries/{id}/components/{component_id}/reopen', {
          params: {
            path: { id: input.id, component_id: input.componentId },
            header: ifMatch(input.revision),
          },
        }),
      ),
    onSuccess: () => invalidate(),
  });
}

export function useSetMealPlanParticipants() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['SetMealPlanParticipantsRequest'];
    }) =>
      unwrap(
        await client.PUT('/api/v1/meal-plan-entries/{id}/participants', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: () => invalidate(),
  });
}

export function useOptOutOfMeal() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number }) =>
      unwrap(
        await client.POST('/api/v1/meal-plan-entries/{id}/opt-out', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: () => invalidate(),
  });
}

export function useRejoinMeal() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number }) =>
      unwrap(
        await client.DELETE('/api/v1/meal-plan-entries/{id}/opt-out', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: () => invalidate(),
  });
}

export function useHouseholdSlotAttendance(
  date: string,
  slot: string,
  excludeEntry?: string,
) {
  return useQuery({
    queryKey: mealPlanKeys.slotAttendance(date, slot, excludeEntry),
    enabled: Boolean(date && slot),
    queryFn: async () =>
      unwrap(
        await client.GET('/api/v1/household/planner/attendance/{date}/{slot}', {
          params: {
            path: { date, slot },
            query: excludeEntry ? { exclude_entry: excludeEntry } : {},
          },
        }),
      ),
  });
}

export function useReviewMealOutcomes() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['ReviewMealOutcomesRequest'];
    }) =>
      unwrap(
        await client.POST('/api/v1/meal-plan-entries/{id}/outcomes', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: () => invalidate(),
  });
}
