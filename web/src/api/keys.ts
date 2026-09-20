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

export type PreparedMealListParams = CatalogueListParams & {
  sort_by?: components['schemas']['IngredientSortDto'];
  sort?: components['schemas']['SortDirectionDto'];
};

export type MealTemplateListParams = {
  q?: string;
  include_archived?: boolean;
  page?: number;
  per_page?: number;
};

export type RecipeListParams = {
  q?: string;
  include_archived?: boolean;
  page?: number;
  per_page?: number;
};

export type StockListParams = {
  product_id?: string;
  include_archived?: boolean;
  page?: number;
  per_page?: number;
};

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

const ingredients = ['ingredients'] as const;
const ingredient = ['ingredient'] as const;
const preparedMeals = ['preparedMeals'] as const;
const preparedMeal = ['preparedMeal'] as const;
const products = ['products'] as const;
const product = ['product'] as const;
const mealTemplates = ['mealTemplates'] as const;
const mealTemplate = ['mealTemplate'] as const;
const members = ['members'] as const;
const member = ['member'] as const;
const users = ['users'] as const;
const user = ['user'] as const;
const myWeek = ['mealPlanWeek'] as const;
const householdWeek = ['plannerWeek'] as const;
const plannerWeek = ['planner'] as const;
const entry = ['mealPlanEntry'] as const;
const needsReview = ['mealPlanNeedsReview'] as const;
const nutritionTargets = ['nutritionTargets'] as const;
const nutritionPlan = ['nutritionPlan'] as const;
const weight = ['weight'] as const;
const recipes = ['recipes'] as const;
const recipe = ['recipe'] as const;
const stock = ['stock'] as const;
const shopping = ['shopping'] as const;

export const settingsKeys = {
  units: () => ['units'] as const,
  meta: () => ['meta'] as const,
  mealTimes: () => ['mealTimes'] as const,
};

export const catalogueKeys = {
  ingredients: () => ingredients,
  ingredientList: (params: IngredientListParams) => [...ingredients, params] as const,
  ingredient: () => ingredient,
  ingredientDetail: (id: string) => [...ingredient, id] as const,
  ingredientProducts: (id: string) => [...ingredient, id, 'products'] as const,
  preparedMeals: () => preparedMeals,
  preparedMealList: (params: PreparedMealListParams) => [...preparedMeals, params] as const,
  preparedMeal: () => preparedMeal,
  preparedMealDetail: (id: string) => [...preparedMeal, id] as const,
  preparedMealProducts: (id: string) => [...preparedMeal, id, 'products'] as const,
  products: () => products,
  productList: (params: ProductListParams) => [...products, params] as const,
  product: () => product,
  productDetail: (id: string) => [...product, id] as const,
};

export const mealTemplateKeys = {
  all: () => mealTemplates,
  list: (params: MealTemplateListParams) => [...mealTemplates, params] as const,
  detail: () => mealTemplate,
  one: (id: string) => [...mealTemplate, id] as const,
};

export const householdKeys = {
  members: () => members,
  memberList: (params: MemberListParams) => [...members, params] as const,
  member: () => member,
  memberDetail: (id: string) => [...member, id] as const,
  memberAccess: (id: string) => [...member, id, 'access'] as const,
  users: () => users,
  userList: (params: UserListParams) => [...users, params] as const,
  user: () => user,
  userDetail: (id: string) => [...user, id] as const,
};

export const mealPlanKeys = {
  myWeeks: () => myWeek,
  myWeek: (weekStart: string) => [...myWeek, weekStart] as const,
  householdWeeks: () => householdWeek,
  householdWeek: (weekStart: string) => [...householdWeek, weekStart] as const,
  plannerWeeks: () => plannerWeek,
  plannerWeek: (weekStart: string) => [...plannerWeek, weekStart] as const,
  entries: () => entry,
  entry: (id: string) => [...entry, id] as const,
  needsReview: () => needsReview,
};

export const nutritionTargetKeys = {
  all: () => nutritionTargets,
  forMember: (memberId: string) => [...nutritionTargets, memberId] as const,
};

export const nutritionPlanKeys = {
  all: () => nutritionPlan,
  plan: (memberId: string) => [...nutritionPlan, 'plan', memberId] as const,
  profile: (memberId: string) => [...nutritionPlan, 'profile', memberId] as const,
};

export const weightKeys = {
  all: () => weight,
  summary: (memberId: string) => [...weight, 'summary', memberId] as const,
  records: (memberId: string) => [...weight, 'records', memberId] as const,
  goal: (memberId: string) => [...weight, 'goal', memberId] as const,
};

export const recipeKeys = {
  recipes: () => recipes,
  recipeList: (params: RecipeListParams) => [...recipes, params] as const,
  recipe: () => recipe,
  recipeDetail: (id: string) => [...recipe, id] as const,
  nutrition: (id: string) => [...recipe, id, 'nutrition'] as const,
  photo: (id: string, size: 'card' | 'hero', version: number) =>
    [...recipe, id, 'photo', size, version] as const,
};

export const preparationKeys = {
  all: () => ['preparations'] as const,
  range: (from: string, to: string) => ['preparations', from, to] as const,
};

export type StockAvailabilityRange = { from?: string; to?: string };

export const stockKeys = {
  all: () => stock,
  list: (params: StockListParams) => [...stock, params] as const,
  item: (id: string) => [...stock, id] as const,
  events: (id: string) => [...stock, id, 'events'] as const,
  availability: (productId?: string, range?: StockAvailabilityRange) =>
    [...stock, 'availability', productId, range ?? null] as const,
};

export const shoppingKeys = {
  all: () => shopping,
  list: (opportunityDate?: string) => [...shopping, 'requirements', opportunityDate] as const,
  opportunities: () => [...shopping, 'opportunities'] as const,
  cadence: () => [...shopping, 'cadence'] as const,
  purchases: (state?: string) => [...shopping, 'purchases', state] as const,
  items: () => [...shopping, 'items'] as const,
  putAway: () => [...shopping, 'put-away'] as const,
};
