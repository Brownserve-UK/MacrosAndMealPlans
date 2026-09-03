import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { ApiError, authenticatedFetch, client, ifMatch, unwrap, unwrapFetchJson } from './client';
import type {
  Ingredient,
  Member,
  Product,
  Recipe,
  Unit,
  User,
} from './client';
import type { components } from './schema';

type CatalogueListParams = {
  q?: string;
  origin?: components['schemas']['CatalogueOrigin'];
  needs_products?: boolean;
  include_archived?: boolean;
  page?: number;
  per_page?: number;
};

export type IngredientListParams = CatalogueListParams & {
  sort_by?: components['schemas']['IngredientSortDto'];
  sort?: components['schemas']['SortDirectionDto'];
};

export type ProductListParams = CatalogueListParams & {
  barcode?: string;
  retailer?: string;
  mapped_ingredient_id?: string;
  unmapped?: boolean;
};

export type RecipeListParams = {
  q?: string;
  include_archived?: boolean;
  page?: number;
  per_page?: number;
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
  recipes: (params: RecipeListParams) => ['recipes', params] as const,
  recipe: (id: string) => ['recipe', id] as const,
  recipeNutrition: (id: string) => ['recipe', id, 'nutrition'] as const,
  recipePhoto: (id: string, size: 'card' | 'hero', version: number) =>
    ['recipe', id, 'photo', size, version] as const,
  nutritionTargets: (memberId: string) => ['nutritionTargets', memberId] as const,
  mealTimes: ['mealTimes'] as const,
  stock: (params: StockListParams) => ['stock', params] as const,
  stockItem: (id: string) => ['stock', id] as const,
  stockEvents: (id: string) => ['stock', id, 'events'] as const,
  stockAvailability: (productId?: string) => ['stock', 'availability', productId] as const,
  shoppingList: (opportunityDate?: string) => ['shopping', 'requirements', opportunityDate] as const,
  shoppingOpportunities: () => ['shopping', 'opportunities'] as const,
  shoppingCadence: () => ['shopping', 'cadence'] as const,
  purchases: (state?: string) => ['shopping', 'purchases', state] as const,
};

export type StockListParams = {
  product_id?: string;
  include_archived?: boolean;
  page?: number;
  per_page?: number;
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

export function useMealTimes() {
  return useQuery({
    queryKey: keys.mealTimes,
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
      qc.setQueryData(keys.mealTimes, updated);
    },
  });
}

export function useIngredients(params: IngredientListParams) {
  return useQuery({
    queryKey: keys.ingredients(params),
    queryFn: async () => unwrap(await client.GET('/api/v1/ingredients', { params: { query: params } })),
  });
}

export function useIngredient(id: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: keys.ingredient(id),
    enabled: options?.enabled ?? true,
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/ingredients/{id}', { params: { path: { id } } })),
  });
}

export function useIngredientProducts(id: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: keys.ingredientProducts(id),
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
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['products'] });
      void qc.invalidateQueries({ queryKey: ['ingredients'] });
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
      void qc.invalidateQueries({ queryKey: mealPlanKeys.needsReview() });
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

export function useMemberAccess(id: string) {
  return useQuery({
    queryKey: householdKeys.memberAccess(id),
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/members/{id}/access', { params: { path: { id } } })),
  });
}

export function useGrantMemberAccess() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      body: components['schemas']['GrantAccessRequest'];
    }) =>
      unwrap(
        await client.PUT('/api/v1/members/{id}/access', {
          params: { path: { id: input.id } },
          body: input.body,
        }),
      ),
    onSuccess: (_data, input) =>
      qc.invalidateQueries({ queryKey: householdKeys.memberAccess(input.id) }),
  });
}

export function useRevokeMemberAccess() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: {
      id: string;
      userId: string;
      scope: components['schemas']['AccessScope'];
    }) =>
      unwrap(
        await client.DELETE('/api/v1/members/{id}/access/{user_id}/{scope}', {
          params: { path: { id: input.id, user_id: input.userId, scope: input.scope } },
        }),
      ),
    onSuccess: (_data, input) =>
      qc.invalidateQueries({ queryKey: householdKeys.memberAccess(input.id) }),
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

export function useCreateConsumption() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: components['schemas']['CreateConsumptionRequest']) =>
      unwrap(await client.POST('/api/v1/consumption', { body })),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['mealPlanWeek'] });
      void qc.invalidateQueries({ queryKey: ['plannerWeek'] });
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
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['mealPlanWeek'] });
      void qc.invalidateQueries({ queryKey: ['plannerWeek'] });
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
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['mealPlanWeek'] });
      void qc.invalidateQueries({ queryKey: ['plannerWeek'] });
    },
  });
}

export const mealPlanKeys = {
  myWeek: (weekStart: string) => ['mealPlanWeek', weekStart] as const,
  householdWeek: (weekStart: string) => ['plannerWeek', weekStart] as const,
  entry: (id: string) => ['mealPlanEntry', id] as const,
  needsReview: () => ['mealPlanNeedsReview'] as const,
};

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
    void qc.invalidateQueries({ queryKey: ['mealPlanWeek'] });
    void qc.invalidateQueries({ queryKey: ['plannerWeek'] });
    void qc.invalidateQueries({ queryKey: ['householdSlotAttendance'] });
    void qc.invalidateQueries({ queryKey: ['mealPlanNeedsReview'] });
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
    queryKey: ['householdSlotAttendance', date, slot, excludeEntry ?? null],
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
    void qc.invalidateQueries({ queryKey: ['plannerWeek'] });
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

export function useRecipes(params: RecipeListParams) {
  return useQuery({
    queryKey: keys.recipes(params),
    placeholderData: keepPreviousData,
    queryFn: async () => unwrap(await client.GET('/api/v1/recipes', { params: { query: params } })),
  });
}

export function useRecipe(id: string) {
  return useQuery({
    queryKey: keys.recipe(id),
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/recipes/{id}', { params: { path: { id } } })),
  });
}

export function useRecipeNutrition(id: string) {
  return useQuery({
    queryKey: keys.recipeNutrition(id),
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
    queryKey: keys.recipePhoto(id, size, version ?? 0),
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
    onSuccess: () => qc.invalidateQueries({ queryKey: ['recipes'] }),
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
      qc.setQueryData(keys.recipe(updated.id), updated);
      void qc.invalidateQueries({ queryKey: ['recipes'] });
      void qc.invalidateQueries({ queryKey: keys.recipeNutrition(updated.id) });
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
      qc.setQueryData(keys.recipe(updated.id), updated);
      void qc.invalidateQueries({ queryKey: ['recipes'] });
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
      qc.setQueryData(keys.recipe(updated.id), updated);
      void qc.invalidateQueries({ queryKey: ['recipes'] });
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
      qc.setQueryData(keys.recipe(updated.id), updated);
      void qc.invalidateQueries({ queryKey: ['recipes'] });
      void qc.invalidateQueries({ queryKey: keys.recipeNutrition(updated.id) });
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
      qc.setQueryData(keys.recipe(updated.id), updated);
      void qc.invalidateQueries({ queryKey: ['recipes'] });
    },
  });
}

export function useStock(params: StockListParams) {
  return useQuery({
    queryKey: keys.stock(params),
    queryFn: async () => unwrap(await client.GET('/api/v1/stock', { params: { query: params } })),
    placeholderData: keepPreviousData,
  });
}

export function useStockItem(id: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: keys.stockItem(id),
    enabled: options?.enabled ?? true,
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/stock/{id}', { params: { path: { id } } })),
  });
}

export function useStockEvents(id: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: keys.stockEvents(id),
    enabled: options?.enabled ?? true,
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/stock/{id}/events', { params: { path: { id } } })),
  });
}

export function useStockAvailability(productId?: string) {
  return useQuery({
    queryKey: keys.stockAvailability(productId),
    queryFn: async () =>
      unwrap(
        await client.GET('/api/v1/stock/availability', {
          params: { query: { product_id: productId } },
        }),
      ),
  });
}

export function useCreateStockItem() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: components['schemas']['CreateStockItemRequest']) =>
      unwrap(await client.POST('/api/v1/stock', { body })),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['stock'] });
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
      qc.setQueryData(keys.stockItem((updated as { id: string }).id), updated);
      void qc.invalidateQueries({ queryKey: ['stock'] });
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
      void qc.invalidateQueries({ queryKey: ['stock'] });
    },
  });
}

export function useShoppingList(opportunityDate?: string) {
  return useQuery({
    queryKey: keys.shoppingList(opportunityDate),
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
    queryKey: keys.shoppingOpportunities(),
    queryFn: async () => unwrap(await client.GET('/api/v1/shopping/opportunities', {})),
  });
}

export function useShoppingCadence() {
  return useQuery({
    queryKey: keys.shoppingCadence(),
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
    queryKey: keys.purchases(state),
    queryFn: async () =>
      unwrap(await client.GET('/api/v1/purchases', { params: { query: { state } } })),
  });
}

function useShoppingInvalidation() {
  const qc = useQueryClient();
  return () => {
    void qc.invalidateQueries({ queryKey: ['shopping'] });
    // Buying something changes what is in the cupboard too.
    void qc.invalidateQueries({ queryKey: ['stock'] });
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
