import { keepPreviousData, useMutation, useQueries, useQuery, useQueryClient } from '@tanstack/react-query';
import { client, ifMatch, unwrap } from '../client';
import type { components } from '../schema';
import { stockKeys } from '../keys';
import type { StockAvailabilityRange, StockListParams } from '../keys';

export function useStock(params: StockListParams) {
  return useQuery({
    queryKey: stockKeys.list(params),
    queryFn: async () => unwrap(await client.GET('/api/v1/stock', { params: { query: params } })),
    placeholderData: keepPreviousData,
  });
}

export function useStockItem(id: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: stockKeys.item(id),
    enabled: options?.enabled ?? true,
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/stock/{id}', { params: { path: { id } } })),
  });
}

export function useStockEvents(id: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: stockKeys.events(id),
    enabled: options?.enabled ?? true,
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/stock/{id}/events', { params: { path: { id } } })),
  });
}

export function useStockAvailability(productId?: string, range?: StockAvailabilityRange) {
  return useQuery({
    queryKey: stockKeys.availability(productId, range),
    queryFn: async () =>
      unwrap(
        await client.GET('/api/v1/stock/availability', {
          params: { query: { product_id: productId, from: range?.from, to: range?.to } },
        }),
      ),
  });
}

export function useStockEventsFor(ids: string[]) {
  return useQueries({
    queries: ids.map((id) => ({
      queryKey: stockKeys.events(id),
      queryFn: async () =>
        unwrap(await client.GET('/api/v1/stock/{id}/events', { params: { path: { id } } })),
    })),
    combine: (results) => ({
      data: results.flatMap((result) => result.data ?? []),
      isLoading: results.some((result) => result.isLoading),
    }),
  });
}

export function useCreateStockItem() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: components['schemas']['CreateStockItemRequest']) =>
      unwrap(await client.POST('/api/v1/stock', { body })),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: stockKeys.all() });
    },
  });
}

export function useUpdateStockItem() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['UpdateStockItemRequest'];
    }) =>
      unwrap(
        await client.PATCH('/api/v1/stock/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: (updated) => {
      qc.setQueryData(stockKeys.item((updated as { id: string }).id), updated);
      void qc.invalidateQueries({ queryKey: stockKeys.all() });
    },
  });
}

export function useArchiveStockItem() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number }) =>
      unwrap(
        await client.POST('/api/v1/stock/{id}/archive', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: stockKeys.all() });
    },
  });
}
