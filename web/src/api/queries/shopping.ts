import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { ApiError, client, ifMatch, unwrap } from '../client';
import type { Unit } from '../client';
import { stockKeys, shoppingKeys } from '../keys';

export function useShoppingList(opportunityDate?: string) {
  return useQuery({
    queryKey: shoppingKeys.list(opportunityDate),
    queryFn: async () =>
      unwrap(
        await client.GET('/api/v1/shopping/requirements', {
          params: { query: { opportunity_date: opportunityDate } },
        }),
      ),
  });
}

export function useShoppingOpportunities() {
  return useQuery({
    queryKey: shoppingKeys.opportunities(),
    queryFn: async () => unwrap(await client.GET('/api/v1/shopping/opportunities', {})),
  });
}

export function useShoppingCadence() {
  return useQuery({
    queryKey: shoppingKeys.cadence(),
    queryFn: async () => {
      try {
        return await unwrap(await client.GET('/api/v1/shopping/cadence', {}));
      } catch (error) {
        // No cadence configured is a normal state, not a failure.
        if (error instanceof ApiError && error.status === 404) return null;
        throw error;
      }
    },
  });
}

export function usePurchases(state?: 'pending' | 'reconciled' | 'cancelled') {
  return useQuery({
    queryKey: shoppingKeys.purchases(state),
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/purchases', { params: { query: { state } } })),
  });
}

function useShoppingInvalidation() {
  const qc = useQueryClient();
  return () => {
    void qc.invalidateQueries({ queryKey: shoppingKeys.all() });
    // Buying something changes what is in the cupboard too.
    void qc.invalidateQueries({ queryKey: stockKeys.all() });
  };
}

export function useSetShoppingCadence() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async (body: {
      interval_weeks: number;
      days: number[];
      anchor: string;
      usual_time?: string | null;
    }) => unwrap(await client.PUT('/api/v1/shopping/cadence', { body })),
    onSuccess: invalidate,
  });
}

export function useClearShoppingCadence() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async () => unwrap(await client.DELETE('/api/v1/shopping/cadence', {})),
    onSuccess: invalidate,
  });
}

export function useMoveShoppingOpportunity() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async (input: { date: string; to: string }) =>
      unwrap(
        await client.PUT('/api/v1/shopping/opportunities/{date}', {
          params: { path: { date: input.date } },
          body: { to: input.to },
        }),
      ),
    onSuccess: invalidate,
  });
}

export function useFinishShop() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async (date: string) =>
      unwrap(
        await client.POST('/api/v1/shopping/opportunities/{date}/finish', {
          params: { path: { date } },
        }),
      ),
    onSuccess: invalidate,
  });
}

export function useSkipShoppingOpportunity() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async (date: string) =>
      unwrap(
        await client.DELETE('/api/v1/shopping/opportunities/{date}', {
          params: { path: { date } },
        }),
      ),
    onSuccess: invalidate,
  });
}

export function useAddShoppingOpportunity() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async (input: { date: string; note?: string }) =>
      unwrap(await client.POST('/api/v1/shopping/opportunities', { body: input })),
    onSuccess: invalidate,
  });
}

export function useRecordPurchase() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async (body: {
      ingredient_id?: string;
      product_id?: string;
      quantity?: { amount: number; unit: Unit };
      opportunity_date?: string;
      note?: string;
    }) => unwrap(await client.POST('/api/v1/purchases', { body })),
    onSuccess: invalidate,
  });
}

export function useUpdatePurchase() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      product_id?: string;
      quantity?: { amount: number; unit: Unit };
      cancelled?: boolean;
    }) =>
      unwrap(
        await client.PATCH('/api/v1/purchases/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: {
            product_id: input.product_id,
            quantity: input.quantity,
            cancelled: input.cancelled,
          },
        }),
      ),
    onSuccess: invalidate,
  });
}
