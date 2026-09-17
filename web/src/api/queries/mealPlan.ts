import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { MealSlot } from '../client';
import { client, ifMatch, unwrap } from '../client';
import type { components } from '../schema';
import { mealPlanKeys } from '../keys';
import type {
  Attendance,
  GroupPatch,
  GroupView,
  NewGroup,
  OccasionPatch,
  OccasionView,
  PlannerWeek,
} from '../../features/meal-plan/planner/types';

type UntypedResult<T> = Promise<{ data?: T; error?: unknown; response: Response }>;

type UntypedClient = {
  GET: (path: string, init?: object) => UntypedResult<unknown>;
  POST: (path: string, init?: object) => UntypedResult<unknown>;
  PATCH: (path: string, init?: object) => UntypedResult<unknown>;
  PUT: (path: string, init?: object) => UntypedResult<unknown>;
  DELETE: (path: string, init?: object) => UntypedResult<unknown>;
};

// TODO: swap to generated types
const planner = client as unknown as UntypedClient;

export function useNeedsReview() {
  return useQuery({
    queryKey: mealPlanKeys.needsReview(),
    queryFn: async () => unwrap(await client.GET('/api/v1/needs-review', {})),
  });
}

export function useMealPlanWeek(weekStart: string, enabled = true) {
  return useQuery({
    queryKey: mealPlanKeys.myWeek(weekStart),
    enabled: Boolean(weekStart) && enabled,
    queryFn: async () =>
      unwrap(
        await client.GET('/api/v1/meal-plan/{week_start}', {
          params: { path: { week_start: weekStart } },
        }),
      ),
  });
}

export function usePlannerWeek(weekStart: string, enabled = true) {
  return useQuery({
    queryKey: mealPlanKeys.plannerWeek(weekStart),
    enabled: Boolean(weekStart) && enabled,
    queryFn: async () =>
      unwrap(
        await planner.GET('/api/v1/planner/{week_start}', {
          params: { path: { week_start: weekStart } },
        }),
      ) as PlannerWeek,
  });
}

function useMealPlanInvalidation() {
  const qc = useQueryClient();
  return () => {
    void qc.invalidateQueries({ queryKey: mealPlanKeys.myWeeks() });
    void qc.invalidateQueries({ queryKey: mealPlanKeys.householdWeeks() });
    void qc.invalidateQueries({ queryKey: mealPlanKeys.plannerWeeks() });
    void qc.invalidateQueries({ queryKey: mealPlanKeys.needsReview() });
  };
}

export function useCreateOccasion() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (body: { planned_on: string; slot: MealSlot; group: NewGroup }) =>
      unwrap(await planner.POST('/api/v1/planner/occasions', { body })) as OccasionView,
    onSuccess: () => invalidate(),
  });
}

export function useUpdateOccasion() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; body: OccasionPatch }) =>
      unwrap(
        await planner.PATCH('/api/v1/planner/occasions/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.body.revision) },
          body: input.body,
        }),
      ) as OccasionView,
    onSuccess: () => invalidate(),
  });
}

export function useDeleteOccasion() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number }) =>
      unwrap(
        await planner.DELETE('/api/v1/planner/occasions/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: () => invalidate(),
  });
}

export function useMoveOccasion() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; planned_on: string; slot: MealSlot }) =>
      unwrap(
        await planner.POST('/api/v1/planner/occasions/{id}/move', {
          params: { path: { id: input.id } },
          body: { planned_on: input.planned_on, slot: input.slot },
        }),
      ) as OccasionView,
    onSuccess: () => invalidate(),
  });
}

export function useCopyOccasion() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; planned_on: string; slot: MealSlot }) =>
      unwrap(
        await planner.POST('/api/v1/planner/occasions/{id}/copy', {
          params: { path: { id: input.id } },
          body: { planned_on: input.planned_on, slot: input.slot },
        }),
      ) as OccasionView,
    onSuccess: () => invalidate(),
  });
}

export function useAddGroup() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: { occasionId: string; body: NewGroup }) =>
      unwrap(
        await planner.POST('/api/v1/planner/occasions/{id}/groups', {
          params: { path: { id: input.occasionId } },
          body: input.body,
        }),
      ) as GroupView,
    onSuccess: () => invalidate(),
  });
}

export function useUpdateGroup() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; body: GroupPatch }) =>
      unwrap(
        await planner.PATCH('/api/v1/planner/groups/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.body.revision) },
          body: input.body,
        }),
      ) as GroupView,
    onSuccess: () => invalidate(),
  });
}

export function useDeleteGroup() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number }) =>
      unwrap(
        await planner.DELETE('/api/v1/planner/groups/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: () => invalidate(),
  });
}

export function useSetAttendance() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: { occasionId: string; memberId: string; attendance: Attendance }) =>
      unwrap(
        await planner.PUT('/api/v1/planner/occasions/{id}/attendance/{member_id}', {
          params: { path: { id: input.occasionId, member_id: input.memberId } },
          body: { attendance: input.attendance },
        }),
      ) as OccasionView,
    onSuccess: () => invalidate(),
  });
}

export function useCopyWeek() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: { weekStart: string; sourceWeekStart: string }) =>
      unwrap(
        await planner.POST('/api/v1/planner/{week_start}/copy-from/{source_week_start}', {
          params: { path: { week_start: input.weekStart, source_week_start: input.sourceWeekStart } },
        }),
      ) as PlannerWeek,
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
