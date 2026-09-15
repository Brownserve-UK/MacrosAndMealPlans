import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { authenticatedFetch, client, ifMatch, unwrap, unwrapFetchJson } from '../client';
import type { Recipe } from '../client';
import type { components } from '../schema';
import { recipeKeys } from '../keys';
import type { RecipeListParams } from '../keys';

export function useRecipes(params: RecipeListParams) {
  return useQuery({
    queryKey: recipeKeys.recipeList(params),
    placeholderData: keepPreviousData,
    queryFn: async () => unwrap(await client.GET('/api/v1/recipes', { params: { query: params } })),
  });
}

export function useRecipe(id: string) {
  return useQuery({
    queryKey: recipeKeys.recipeDetail(id),
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/recipes/{id}', { params: { path: { id } } })),
  });
}

export function useRecipeNutrition(id: string) {
  return useQuery({
    queryKey: recipeKeys.nutrition(id),
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/recipes/{id}/nutrition', { params: { path: { id } } })),
  });
}

export function useRecipePhoto(
  id: string,
  size: 'card' | 'hero',
  version: number | null | undefined,
) {
  return useQuery({
    queryKey: recipeKeys.photo(id, size, version ?? 0),
    enabled: version != null,
    staleTime: Infinity,
    queryFn: async () => {
      const response = await authenticatedFetch(
        `/api/v1/recipes/${id}/photo/${size}?v=${version}`,
      );
      if (!response.ok) throw new Error('Could not load the recipe photo.');
      return response.blob();
    },
  });
}

export function useCreateRecipe() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: components['schemas']['CreateRecipeRequest']) =>
      unwrap(await client.POST('/api/v1/recipes', { body })),
    onSuccess: () => qc.invalidateQueries({ queryKey: recipeKeys.recipes() }),
  });
}

export function useUpdateRecipe() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['UpdateRecipeRequest'];
    }) =>
      unwrap(
        await client.PATCH('/api/v1/recipes/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: (updated: Recipe) => {
      qc.setQueryData(recipeKeys.recipeDetail(updated.id), updated);
      void qc.invalidateQueries({ queryKey: recipeKeys.recipes() });
      void qc.invalidateQueries({ queryKey: recipeKeys.nutrition(updated.id) });
    },
  });
}

export function useUploadRecipePhoto() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; file: File }) => {
      const response = await authenticatedFetch(`/api/v1/recipes/${input.id}/photo`, {
        method: 'PUT',
        headers: {
          'Content-Type': input.file.type,
          'If-Match': `"${input.revision}"`,
        },
        body: input.file,
      });
      return unwrapFetchJson<Recipe>(response);
    },
    onSuccess: (updated) => {
      qc.setQueryData(recipeKeys.recipeDetail(updated.id), updated);
      void qc.invalidateQueries({ queryKey: recipeKeys.recipes() });
    },
  });
}

export function useDeleteRecipePhoto() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number }) =>
      unwrap(
        await client.DELETE('/api/v1/recipes/{id}/photo', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: (updated: Recipe) => {
      qc.setQueryData(recipeKeys.recipeDetail(updated.id), updated);
      void qc.invalidateQueries({ queryKey: recipeKeys.recipes() });
    },
  });
}

export function useResolveRecipeComponent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      componentId: string;
      revision: number;
      body: components['schemas']['ResolveComponentRequest'];
    }) =>
      unwrap(
        await client.POST('/api/v1/recipes/{id}/components/{component_id}/resolve', {
          params: {
            path: { id: input.id, component_id: input.componentId },
            header: ifMatch(input.revision),
          },
          body: input.body,
        }),
      ),
    onSuccess: (updated: Recipe) => {
      qc.setQueryData(recipeKeys.recipeDetail(updated.id), updated);
      void qc.invalidateQueries({ queryKey: recipeKeys.recipes() });
      void qc.invalidateQueries({ queryKey: recipeKeys.nutrition(updated.id) });
    },
  });
}

export function useSetRecipeArchived() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; archived: boolean }) => {
      const path = input.archived
        ? ('/api/v1/recipes/{id}/archive' as const)
        : ('/api/v1/recipes/{id}/unarchive' as const);
      return unwrap(
        await client.POST(path, {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      );
    },
    onSuccess: (updated: Recipe) => {
      qc.setQueryData(recipeKeys.recipeDetail(updated.id), updated);
      void qc.invalidateQueries({ queryKey: recipeKeys.recipes() });
    },
  });
}
