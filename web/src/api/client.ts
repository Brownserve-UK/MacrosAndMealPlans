import createClient, { type Middleware } from 'openapi-fetch';
import type { components, paths } from './schema';

export type Problem = components['schemas']['Problem'];
export type Ingredient = components['schemas']['IngredientDto'];
export type Product = components['schemas']['ProductDto'];
export type Nutrition = components['schemas']['NutritionDto'];
export type Unit = components['schemas']['Unit'];
export type UnitInfo = components['schemas']['UnitDto'];
export type Origin = components['schemas']['CatalogueOrigin'];
export type Principal = components['schemas']['PrincipalDto'];
export type Member = components['schemas']['HouseholdMemberDto'];
export type User = components['schemas']['UserDto'];
export type Role = components['schemas']['Role'];
export type Quantity = components['schemas']['QuantityDto'];
export type Amount = components['schemas']['AmountDto'];
export type ConsumptionRecord = components['schemas']['ConsumptionRecordDto'];
export type DiaryEntry = components['schemas']['DiaryEntryDto'];
export type DiaryDay = components['schemas']['DiaryDayDto'];
export type NutritionQuality = components['schemas']['NutritionQuality'];
export type MealPlanEntry = components['schemas']['MealPlanEntryDto'];
export type MealPlanDay = components['schemas']['MealPlanDayDto'];
export type MealPlanWeek = components['schemas']['MealPlanWeekDto'];
export type MealPlanComponent = components['schemas']['MealPlanComponentDto'];
export type MealParticipant = components['schemas']['MealParticipantDto'];
export type MealParticipantAllocation = components['schemas']['MealParticipantAllocationDto'];
export type ComponentPreparation = components['schemas']['ComponentPreparationDto'];
export type ParticipantStatus = components['schemas']['ParticipantStatus'];
export type MealPlanScope = components['schemas']['MealPlanScope'];
export type MealPlanSummary = components['schemas']['NutritionSummaryDto'];
export type MealPlanStatus = components['schemas']['MealPlanStatus'];
export type MealSlot = components['schemas']['MealSlot'];
export type PlannerWeek = components['schemas']['PlannerWeekDto'];
export type PlannerMeal = components['schemas']['PlannerMealDto'];
export type PlannerFood = components['schemas']['PlannerFoodDto'];
export type PlannerPerson = components['schemas']['PlannerPersonDto'];
export type MealGuestGroup = components['schemas']['MealGuestGroupDto'];
export type MealItem = components['schemas']['MealItemDto'];
export type MealItemSource = components['schemas']['MealItemSourceDto'];
export type MealSlotView = components['schemas']['MealSlotViewDto'];
export type Meta = components['schemas']['MetaDto'];
export type MealTimesSettings = components['schemas']['HouseholdSettingsDto'];
export type Recipe = components['schemas']['RecipeDto'];
export type RecipeSummary = components['schemas']['RecipeSummaryDto'];
export type RecipeComponent = components['schemas']['RecipeComponentDto'];
export type RecipeNutrition = components['schemas']['RecipeNutritionDto'];
export type RecipeNutritionGap = components['schemas']['RecipeNutritionGapDto'];
export type NutritionTarget = components['schemas']['NutritionTargetDto'];
export type NutritionGoals = components['schemas']['NutritionGoalsDto'];
export type TargetDirection = components['schemas']['TargetDirectionDto'];
export type StockItem = components['schemas']['StockItemDto'];
export type StockLevel = components['schemas']['StockLevelDto'];
export type StockEvent = components['schemas']['StockEventDto'];
export type StorageLocation = components['schemas']['StorageLocationDto'];
export type TrackingMode = components['schemas']['TrackingModeDto'];
export type ProductAvailability = components['schemas']['ProductAvailabilityDto'];
export type Availability = components['schemas']['AvailabilityDto'];

const CREDENTIAL_KEY = 'mmp.credential';

let credential: string | null = readStoredCredential();

function readStoredCredential(): string | null {
  try {
    return sessionStorage.getItem(CREDENTIAL_KEY);
  } catch {
    return null;
  }
}

export function setCredential(value: string | null) {
  credential = value;
  try {
    if (value) sessionStorage.setItem(CREDENTIAL_KEY, value);
    else sessionStorage.removeItem(CREDENTIAL_KEY);
  } catch {
  }
}

export function hasCredential() {
  return credential !== null;
}

export function encodeCredential(username: string, password: string) {
  return `Basic ${btoa(`${username}:${password}`)}`;
}

export class ApiError extends Error {
  readonly status: number;
  readonly problem: Problem | null;

  constructor(status: number, problem: Problem | null) {
    super(problem?.detail ?? `The request failed with status ${status}.`);
    this.name = 'ApiError';
    this.status = status;
    this.problem = problem;
  }

  get isConflict() {
    return this.status === 409 && this.problem?.actual_revision != null;
  }

  get isUnauthorized() {
    return this.status === 401;
  }

  get fieldErrors(): Record<string, string> {
    const entries = this.problem?.errors ?? [];
    return Object.fromEntries(entries.map((e) => [e.field, e.message]));
  }
}

const authMiddleware: Middleware = {
  async onRequest({ request }) {
    if (credential) request.headers.set('Authorization', credential);
    return request;
  },
};

export const client = createClient<paths>({ baseUrl: '' });
client.use(authMiddleware);

type Result<T> = { data?: T; error?: unknown; response: Response };

export function unwrap<T>(result: Result<T>): T {
  if (result.error !== undefined || !result.response.ok) {
    const problem = (result.error ?? null) as Problem | null;
    throw new ApiError(result.response.status, problem);
  }
  return result.data as T;
}

export function ifMatch(revision: number): { 'If-Match': string } {
  return { 'If-Match': `"${revision}"` };
}

export function authenticatedFetch(input: RequestInfo | URL, init: RequestInit = {}) {
  const headers = new Headers(init.headers);
  if (credential) headers.set('Authorization', credential);
  return fetch(input, { ...init, headers });
}

export async function unwrapFetchJson<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const problem = response.headers.get('content-type')?.includes('json')
      ? ((await response.json()) as Problem)
      : null;
    throw new ApiError(response.status, problem);
  }
  return (await response.json()) as T;
}
