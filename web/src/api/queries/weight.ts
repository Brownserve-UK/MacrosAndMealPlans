import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { client, ifMatch, unwrap } from '../client';
import type { components } from '../schema';
import { weightKeys } from '../keys';

function useWeightInvalidation() {
  const qc = useQueryClient();
  return (memberId: string) => {
    void qc.invalidateQueries({ queryKey: weightKeys.summary(memberId) });
    void qc.invalidateQueries({ queryKey: weightKeys.records(memberId) });
    void qc.invalidateQueries({ queryKey: weightKeys.goal(memberId) });
  };
}

export function useWeightSummary(memberId: string, from?: string) {
  return useQuery({
    queryKey: [...weightKeys.summary(memberId), from],
    enabled: Boolean(memberId),
    placeholderData: keepPreviousData,
    queryFn: async () =>
      unwrap(
        await client.GET('/api/v1/members/{member_id}/weight-summary', {
          params: { path: { member_id: memberId }, query: { from } },
        }),
      ),
  });
}

export function useWeightRecords(memberId: string, limit?: number, enabled = true) {
  return useQuery({
    queryKey: [...weightKeys.records(memberId), limit],
    enabled: Boolean(memberId) && enabled,
    queryFn: async () =>
      unwrap(
        await client.GET('/api/v1/members/{member_id}/weight-records', {
          params: { path: { member_id: memberId }, query: { limit } },
        }),
      ),
  });
}

export function useRecordWeighIn() {
  const invalidate = useWeightInvalidation();
  return useMutation({
    mutationFn: async (input: {
      memberId: string;
      body: components['schemas']['CreateWeightRecordRequest'];
    }) =>
      unwrap(
        await client.POST('/api/v1/members/{member_id}/weight-records', {
          params: { path: { member_id: input.memberId } },
          body: input.body,
        }),
      ),
    onSuccess: (record) => invalidate(record.member_id),
  });
}

export function useUpdateWeighIn() {
  const invalidate = useWeightInvalidation();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['UpdateWeightRecordRequest'];
    }) =>
      unwrap(
        await client.PATCH('/api/v1/weight-records/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: (record) => invalidate(record.member_id),
  });
}

export function useDeleteWeighIn() {
  const invalidate = useWeightInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; memberId: string }) =>
      unwrap(
        await client.DELETE('/api/v1/weight-records/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: (_data, variables) => invalidate(variables.memberId),
  });
}

export function useSetWeightGoal() {
  const invalidate = useWeightInvalidation();
  return useMutation({
    mutationFn: async (input: {
      memberId: string;
      body: components['schemas']['CreateWeightGoalRequest'];
    }) =>
      unwrap(
        await client.POST('/api/v1/members/{member_id}/weight-goal', {
          params: { path: { member_id: input.memberId } },
          body: input.body,
        }),
      ),
    onSuccess: (goal) => invalidate(goal.member_id),
  });
}

export function useUpdateWeightGoal() {
  const invalidate = useWeightInvalidation();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['UpdateWeightGoalRequest'];
    }) =>
      unwrap(
        await client.PATCH('/api/v1/weight-goals/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: (goal) => invalidate(goal.member_id),
  });
}

export function useClearWeightGoal() {
  const invalidate = useWeightInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; memberId: string }) =>
      unwrap(
        await client.DELETE('/api/v1/weight-goals/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: (_data, variables) => invalidate(variables.memberId),
  });
}
