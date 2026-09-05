import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { client, ifMatch, unwrap } from '../client';
import type { Ingredient, Product } from '../client';
import type { components } from '../schema';
import { catalogueKeys, mealPlanKeys } from '../keys';
import type { IngredientListParams, ProductListParams } from '../keys';

export function useIngredients(params: IngredientListParams) {
  return useQuery({
    queryKey: catalogueKeys.ingredientList(params),
    queryFn: async () => unwrap(await client.GET('/api/v1/ingredients', { params: { query: params } })),
  });
}

export function useIngredient(id: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: catalogueKeys.ingredientDetail(id),
    enabled: options?.enabled ?? true,
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/ingredients/{id}', { params: { path: { id } } })),
  });
}

export function useIngredientProducts(id: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: catalogueKeys.ingredientProducts(id),
    enabled: options?.enabled ?? true,
    queryFn: async () =>
      unwrap(
        await client.GET('/api/v1/ingredients/{id}/products', { params: { path: { id } } }),
      ),
  });
}

export function useCreateIngredient() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: components['schemas']['CreateIngredientRequest']) =>
      unwrap(await client.POST('/api/v1/ingredients', { body })),
    onSuccess: () => qc.invalidateQueries({ queryKey: catalogueKeys.ingredients() }),
  });
}

export function useUpdateIngredient() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['UpdateIngredientRequest'];
    }) =>
      unwrap(
        await client.PATCH('/api/v1/ingredients/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: (updated: Ingredient) => {
      qc.setQueryData(catalogueKeys.ingredientDetail(updated.id), updated);
      void qc.invalidateQueries({ queryKey: catalogueKeys.ingredients() });
    },
  });
}

export function useSetIngredientArchived() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; archived: boolean }) => {
      const path = input.archived
        ? ('/api/v1/ingredients/{id}/archive' as const)
        : ('/api/v1/ingredients/{id}/unarchive' as const);
      return unwrap(
        await client.POST(path, {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      );
    },
    onSuccess: (updated: Ingredient) => {
      qc.setQueryData(catalogueKeys.ingredientDetail(updated.id), updated);
      void qc.invalidateQueries({ queryKey: catalogueKeys.ingredients() });
    },
  });
}

export function useProducts(params: ProductListParams) {
  return useQuery({
    queryKey: catalogueKeys.productList(params),
    queryFn: async () => unwrap(await client.GET('/api/v1/products', { params: { query: params } })),
  });
}

export function useProduct(id: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: catalogueKeys.productDetail(id),
    enabled: options?.enabled ?? true,
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/products/{id}', { params: { path: { id } } })),
  });
}

export function useCreateProduct() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: components['schemas']['CreateProductRequest']) =>
      unwrap(await client.POST('/api/v1/products', { body })),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: catalogueKeys.products() });
      void qc.invalidateQueries({ queryKey: catalogueKeys.ingredients() });
      void qc.invalidateQueries({ queryKey: mealPlanKeys.needsReview() });
    },
  });
}

export function useUpdateProduct() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['UpdateProductRequest'];
    }) =>
      unwrap(
        await client.PATCH('/api/v1/products/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: (updated: Product) => {
      qc.setQueryData(catalogueKeys.productDetail(updated.id), updated);
      void qc.invalidateQueries({ queryKey: catalogueKeys.products() });
    },
  });
}

export function useSetProductArchived() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; archived: boolean }) => {
      const path = input.archived
        ? ('/api/v1/products/{id}/archive' as const)
        : ('/api/v1/products/{id}/unarchive' as const);
      return unwrap(
        await client.POST(path, {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      );
    },
    onSuccess: (updated: Product) => {
      qc.setQueryData(catalogueKeys.productDetail(updated.id), updated);
      void qc.invalidateQueries({ queryKey: catalogueKeys.products() });
    },
  });
}

export function useSetProductMapping() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; ingredientId: string | null }) => {
      const params = { path: { id: input.id }, header: ifMatch(input.revision) };
      return input.ingredientId === null
        ? unwrap(await client.DELETE('/api/v1/products/{id}/ingredient', { params }))
        : unwrap(
            await client.PUT('/api/v1/products/{id}/ingredient', {
              params,
              body: { ingredient_id: input.ingredientId },
            }),
          );
    },
    onSuccess: (updated: Product) => {
      qc.setQueryData(catalogueKeys.productDetail(updated.id), updated);
      void qc.invalidateQueries({ queryKey: catalogueKeys.products() });
      void qc.invalidateQueries({ queryKey: catalogueKeys.ingredient() });
      void qc.invalidateQueries({ queryKey: mealPlanKeys.needsReview() });
    },
  });
}
