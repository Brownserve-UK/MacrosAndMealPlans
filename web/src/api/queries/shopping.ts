import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { ApiError, client, ifMatch, unwrap } from '../client';
import type { ShoppingSection, StorageLocation, Unit } from '../client';
import { stockKeys, shoppingKeys } from '../keys';

export function useShoppingList(opportunityDate?: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: shoppingKeys.list(opportunityDate),
    enabled: options?.enabled ?? true,
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
    void qc.invalidateQueries({ queryKey: stockKeys.all() });
  };
}

export function useSetShoppingCadence() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async (input: {
      revision: number;
      interval_weeks: number;
      days: number[];
      anchor: string;
      usual_time?: string | null;
    }) =>
      unwrap(
        await client.PUT('/api/v1/shopping/cadence', {
          params: { header: ifMatch(input.revision) },
          body: {
            interval_weeks: input.interval_weeks,
            days: input.days,
            anchor: input.anchor,
            usual_time: input.usual_time,
          },
        }),
      ),
    onSuccess: invalidate,
  });
}

export function useClearShoppingCadence() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async (revision: number) =>
      unwrap(
        await client.DELETE('/api/v1/shopping/cadence', {
          params: { header: ifMatch(revision) },
        }),
      ),
    onSuccess: invalidate,
  });
}

export function useMoveShoppingOpportunity() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async (input: { date: string; to: string; revision: number }) =>
      unwrap(
        await client.PUT('/api/v1/shopping/opportunities/{date}', {
          params: { path: { date: input.date }, header: ifMatch(input.revision) },
          body: { to: input.to },
        }),
      ),
    onSuccess: invalidate,
  });
}

export function usePendingPutAway() {
  return useQuery({
    queryKey: shoppingKeys.putAway(),
    queryFn: async () => unwrap(await client.GET('/api/v1/shopping/put-away', {})),
  });
}

export function usePutAway() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      product_id: string;
      quantity: { amount: number; unit: Unit };
      storage_location?: StorageLocation;
    }) =>
      unwrap(
        await client.POST('/api/v1/shopping/put-away/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: {
            product_id: input.product_id,
            quantity: input.quantity,
            storage_location: input.storage_location,
          },
        }),
      ),
    onSuccess: invalidate,
  });
}

export function useShoppingListItems() {
  return useQuery({
    queryKey: shoppingKeys.items(),
    queryFn: async () => unwrap(await client.GET('/api/v1/shopping/items', {})),
  });
}

export function useAddShoppingListItem() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async (body: {
      name: string;
      ingredient_id?: string;
      product_id?: string;
      quantity?: { amount: number; unit: Unit };
      section?: ShoppingSection;
      opportunity_date?: string;
    }) => unwrap(await client.POST('/api/v1/shopping/items', { body })),
    onSuccess: invalidate,
  });
}

export function useRemoveShoppingListItem() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number }) =>
      unwrap(
        await client.DELETE('/api/v1/shopping/items/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: invalidate,
  });
}

export function useStartShop() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async (date: string) =>
      unwrap(
        await client.POST('/api/v1/shopping/opportunities/{date}/start', {
          params: { path: { date } },
        }),
      ),
    onSuccess: invalidate,
  });
}

export function useFinishShop() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async (input: { date: string; revision: number }) =>
      unwrap(
        await client.POST('/api/v1/shopping/opportunities/{date}/finish', {
          params: { path: { date: input.date }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: invalidate,
  });
}

export function useSkipShoppingOpportunity() {
  const invalidate = useShoppingInvalidation();
  return useMutation({
    mutationFn: async (input: { date: string; revision: number }) =>
      unwrap(
        await client.DELETE('/api/v1/shopping/opportunities/{date}', {
          params: { path: { date: input.date }, header: ifMatch(input.revision) },
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
      name?: string;
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
