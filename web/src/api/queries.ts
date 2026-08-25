import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { client, ifMatch, unwrap } from './client';
import type { Amount, ConsumptionRecord, Ingredient, Member, Product, User } from './client';
import type { components } from './schema';

export type IngredientListParams = {
  q?: string;
  origin?: components['schemas']['CatalogueOrigin'];
  include_archived?: boolean;
  page?: number;
  per_page?: number;
};

export type ProductListParams = IngredientListParams & {
  barcode?: string;
  retailer?: string;
  mapped_ingredient_id?: string;
};

export const keys = {
  units: ['units'] as const,
  meta: ['meta'] as const,
  me: ['me'] as const,
  ingredients: (params: IngredientListParams) => ['ingredients', params] as const,
  ingredient: (id: string) => ['ingredient', id] as const,
  ingredientProducts: (id: string) => ['ingredient', id, 'products'] as const,
  products: (params: ProductListParams) => ['products', params] as const,
  product: (id: string) => ['product', id] as const,
  nutritionTargets: (memberId: string) => ['nutritionTargets', memberId] as const,
};

export function useUnits() {
  return useQuery({
    queryKey: keys.units,
    staleTime: Infinity,
    queryFn: async () => unwrap(await client.GET('/api/v1/units')),
  });
}

export function useMeta() {
  return useQuery({
    queryKey: keys.meta,
    staleTime: Infinity,
    queryFn: async () => unwrap(await client.GET('/api/v1/meta')),
  });
}

export function useIngredients(params: IngredientListParams) {
  return useQuery({
    queryKey: keys.ingredients(params),
    queryFn: async () => unwrap(await client.GET('/api/v1/ingredients', { params: { query: params } })),
  });
}

export function useIngredient(id: string) {
  return useQuery({
    queryKey: keys.ingredient(id),
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/ingredients/{id}', { params: { path: { id } } })),
  });
}

export function useIngredientProducts(id: string) {
  return useQuery({
    queryKey: keys.ingredientProducts(id),
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
    onSuccess: () => qc.invalidateQueries({ queryKey: ['ingredients'] }),
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
      qc.setQueryData(keys.ingredient(updated.id), updated);
      void qc.invalidateQueries({ queryKey: ['ingredients'] });
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
      qc.setQueryData(keys.ingredient(updated.id), updated);
      void qc.invalidateQueries({ queryKey: ['ingredients'] });
    },
  });
}

export function useProducts(params: ProductListParams) {
  return useQuery({
    queryKey: keys.products(params),
    queryFn: async () => unwrap(await client.GET('/api/v1/products', { params: { query: params } })),
  });
}

export function useProduct(id: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: keys.product(id),
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
    onSuccess: () => qc.invalidateQueries({ queryKey: ['products'] }),
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
      qc.setQueryData(keys.product(updated.id), updated);
      void qc.invalidateQueries({ queryKey: ['products'] });
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
      qc.setQueryData(keys.product(updated.id), updated);
      void qc.invalidateQueries({ queryKey: ['products'] });
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
      qc.setQueryData(keys.product(updated.id), updated);
      void qc.invalidateQueries({ queryKey: ['products'] });
      void qc.invalidateQueries({ queryKey: ['ingredient'] });
    },
  });
}

export type MemberListParams = {
  q?: string;
  with_account?: boolean;
  include_archived?: boolean;
  page?: number;
  per_page?: number;
};

export type UserListParams = {
  q?: string;
  role?: components['schemas']['Role'];
  include_archived?: boolean;
  page?: number;
  per_page?: number;
};

export const householdKeys = {
  members: (params: MemberListParams) => ['members', params] as const,
  member: (id: string) => ['member', id] as const,
  memberAccess: (id: string) => ['member', id, 'access'] as const,
  users: (params: UserListParams) => ['users', params] as const,
  user: (id: string) => ['user', id] as const,
};

export function useMembers(params: MemberListParams) {
  return useQuery({
    queryKey: householdKeys.members(params),
    queryFn: async () => unwrap(await client.GET('/api/v1/members', { params: { query: params } })),
  });
}

export function useMember(id: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: householdKeys.member(id),
    enabled: options?.enabled ?? true,
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/members/{id}', { params: { path: { id } } })),
  });
}

export function useCreateMember() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: components['schemas']['CreateMemberRequest']) =>
      unwrap(await client.POST('/api/v1/members', { body })),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['members'] }),
  });
}

export function useUpdateMember() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['UpdateMemberRequest'];
    }) =>
      unwrap(
        await client.PATCH('/api/v1/members/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: (updated: Member) => {
      qc.setQueryData(householdKeys.member(updated.id), updated);
      void qc.invalidateQueries({ queryKey: ['members'] });
    },
  });
}

export function useSetMemberArchived() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; archived: boolean }) => {
      const path = input.archived
        ? ('/api/v1/members/{id}/archive' as const)
        : ('/api/v1/members/{id}/unarchive' as const);
      return unwrap(
        await client.POST(path, {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      );
    },
    onSuccess: (updated: Member) => {
      qc.setQueryData(householdKeys.member(updated.id), updated);
      void qc.invalidateQueries({ queryKey: ['members'] });
    },
  });
}

export function useSetMemberAccount() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; userId: string | null }) => {
      const params = { path: { id: input.id }, header: ifMatch(input.revision) };
      return input.userId === null
        ? unwrap(await client.DELETE('/api/v1/members/{id}/account', { params }))
        : unwrap(
            await client.PUT('/api/v1/members/{id}/account', {
              params,
              body: { user_id: input.userId },
            }),
          );
    },
    onSuccess: (updated: Member) => {
      qc.setQueryData(householdKeys.member(updated.id), updated);
      void qc.invalidateQueries({ queryKey: ['members'] });
      void qc.invalidateQueries({ queryKey: ['users'] });
    },
  });
}

export function useUsers(params: UserListParams) {
  return useQuery({
    queryKey: householdKeys.users(params),
    queryFn: async () => unwrap(await client.GET('/api/v1/users', { params: { query: params } })),
  });
}

export function useCreateUser() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: components['schemas']['CreateUserRequest']) =>
      unwrap(await client.POST('/api/v1/users', { body })),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['users'] }),
  });
}

export function useSetUserRoles() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      roles: components['schemas']['Role'][];
    }) =>
      unwrap(
        await client.PUT('/api/v1/users/{id}/roles', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: { roles: input.roles },
        }),
      ),
    onSuccess: (updated: User) => {
      qc.setQueryData(householdKeys.user(updated.id), updated);
      void qc.invalidateQueries({ queryKey: ['users'] });
    },
  });
}

export function useSetUserArchived() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; archived: boolean }) => {
      const path = input.archived
        ? ('/api/v1/users/{id}/archive' as const)
        : ('/api/v1/users/{id}/unarchive' as const);
      return unwrap(
        await client.POST(path, {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      );
    },
    onSuccess: (updated: User) => {
      qc.setQueryData(householdKeys.user(updated.id), updated);
      void qc.invalidateQueries({ queryKey: ['users'] });
      void qc.invalidateQueries({ queryKey: ['members'] });
    },
  });
}

export const diaryKeys = {
  day: (memberId: string, date: string) => ['diaryDay', memberId, date] as const,
};

export function useDiaryDay(memberId: string, date: string) {
  return useQuery({
    queryKey: diaryKeys.day(memberId, date),
    enabled: Boolean(memberId) && Boolean(date),
    queryFn: async () =>
      unwrap(
        await client.GET('/api/v1/diary/{member_id}/{date}', {
          params: { path: { member_id: memberId, date } },
        }),
      ),
  });
}

export type MealNutritionInput = { productId: string; amount: Amount | null };
export type MealNutritionTotals = {
  energy_kcal?: number;
  protein_g?: number;
  carbohydrate_g?: number;
  fat_g?: number;
};

const MEAL_NUTRITION_KEYS = ['energy_kcal', 'protein_g', 'carbohydrate_g', 'fat_g'] as const;

export function useMealNutrition(items: MealNutritionInput[]) {
  const active = items.filter(
    (item): item is { productId: string; amount: Amount } =>
      Boolean(item.productId) && item.amount !== null,
  );

  const query = useQuery({
    queryKey: ['mealNutrition', active] as const,
    enabled: active.length > 0,
    placeholderData: keepPreviousData,
    queryFn: async () => {
      const parts = await Promise.all(
        active.map(async ({ productId, amount }) =>
          unwrap(
            await client.GET('/api/v1/products/{id}/nutrition', {
              params: {
                path: { id: productId },
                query:
                  amount.kind === 'measure'
                    ? { kind: amount.kind, value: amount.value, unit: amount.unit }
                    : { kind: amount.kind, value: amount.value },
              },
            }),
          ),
        ),
      );

      const total: MealNutritionTotals = {};
      for (const key of MEAL_NUTRITION_KEYS) {
        const values = parts
          .map((part) => part.nutrition[key])
          .filter((value): value is number => value != null);
        if (values.length > 0) total[key] = values.reduce((sum, value) => sum + value, 0);
      }
      return total;
    },
  });

  return { total: active.length > 0 ? query.data ?? null : null };
}

export function useCreateConsumption() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: components['schemas']['CreateConsumptionRequest']) =>
      unwrap(await client.POST('/api/v1/consumption', { body })),
    onSuccess: (created: ConsumptionRecord) => {
      void qc.invalidateQueries({ queryKey: ['diaryDay', created.member_id] });
      void qc.invalidateQueries({ queryKey: ['mealPlanWeek'] });
    },
  });
}

export function useUpdateConsumption() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['UpdateConsumptionRequest'];
    }) =>
      unwrap(
        await client.PATCH('/api/v1/consumption/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: (updated: ConsumptionRecord) => {
      void qc.invalidateQueries({ queryKey: ['diaryDay', updated.member_id] });
      void qc.invalidateQueries({ queryKey: ['mealPlanWeek'] });
    },
  });
}

export function useDeleteConsumption() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; memberId: string }) =>
      unwrap(
        await client.DELETE('/api/v1/consumption/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: (_data, variables) => {
      void qc.invalidateQueries({ queryKey: ['diaryDay', variables.memberId] });
      void qc.invalidateQueries({ queryKey: ['mealPlanWeek'] });
    },
  });
}

export const mealPlanKeys = {
  week: (weekStart: string) => ['mealPlanWeek', weekStart] as const,
  entry: (id: string) => ['mealPlanEntry', id] as const,
};

export function useMealPlanWeek(weekStart: string) {
  return useQuery({
    queryKey: mealPlanKeys.week(weekStart),
    enabled: Boolean(weekStart),
    queryFn: async () =>
      unwrap(
        await client.GET('/api/v1/meal-plan/{week_start}', {
          params: { path: { week_start: weekStart } },
        }),
      ),
  });
}

function useMealPlanInvalidation() {
  const qc = useQueryClient();
  return (memberId?: string) => {
    void qc.invalidateQueries({ queryKey: ['mealPlanWeek'] });
    if (memberId) void qc.invalidateQueries({ queryKey: ['diaryDay', memberId] });
  };
}

export function useCreateMealPlanEntry() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (body: components['schemas']['CreateMealPlanEntryRequest']) =>
      unwrap(await client.POST('/api/v1/meal-plan-entries', { body })),
    onSuccess: (entry) => invalidate(entry.member_id),
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
    onSuccess: (entry) => invalidate(entry.member_id),
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
    onSuccess: (entry) => invalidate(entry.member_id),
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
    onSuccess: (entry) => invalidate(entry.member_id),
  });
}

export function useReopenMealPlanEntry() {
  const invalidate = useMealPlanInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number }) =>
      unwrap(
        await client.POST('/api/v1/meal-plan-entries/{id}/reopen', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: (entry) => invalidate(entry.member_id),
  });
}

export function useNutritionTargets(memberId: string) {
  return useQuery({
    queryKey: keys.nutritionTargets(memberId),
    enabled: Boolean(memberId),
    queryFn: async () =>
      unwrap(
        await client.GET('/api/v1/members/{member_id}/nutrition-targets', {
          params: { path: { member_id: memberId } },
        }),
      ),
  });
}

function useNutritionTargetInvalidation() {
  const qc = useQueryClient();
  return (memberId: string) => {
    void qc.invalidateQueries({ queryKey: keys.nutritionTargets(memberId) });
    void qc.invalidateQueries({ queryKey: ['mealPlanWeek'] });
  };
}

export function useCreateNutritionTarget() {
  const invalidate = useNutritionTargetInvalidation();
  return useMutation({
    mutationFn: async (input: {
      memberId: string;
      body: components['schemas']['CreateNutritionTargetRequest'];
    }) =>
      unwrap(
        await client.POST('/api/v1/members/{member_id}/nutrition-targets', {
          params: { path: { member_id: input.memberId } },
          body: input.body,
        }),
      ),
    onSuccess: (target) => invalidate(target.member_id),
  });
}

export function useUpdateNutritionTarget() {
  const invalidate = useNutritionTargetInvalidation();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      revision: number;
      body: components['schemas']['UpdateNutritionTargetRequest'];
    }) =>
      unwrap(
        await client.PATCH('/api/v1/nutrition-targets/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
          body: input.body,
        }),
      ),
    onSuccess: (target) => invalidate(target.member_id),
  });
}

export function useDeleteNutritionTarget() {
  const invalidate = useNutritionTargetInvalidation();
  return useMutation({
    mutationFn: async (input: { id: string; revision: number; memberId: string }) =>
      unwrap(
        await client.DELETE('/api/v1/nutrition-targets/{id}', {
          params: { path: { id: input.id }, header: ifMatch(input.revision) },
        }),
      ),
    onSuccess: (_data, variables) => invalidate(variables.memberId),
  });
}
