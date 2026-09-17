import type { Amount, MealPlanComponent, MealSlot } from '../../../api/client';

export type AdHocKind = 'eating_out' | 'takeaway' | 'fend_for_yourself';

export type PlannerMember = {
  id: string;
  name: string;
  initials: string;
};

export type UsualTimes = {
  breakfast: string;
  lunch: string;
  dinner: string;
  snacks: string | null;
};

export type GroupParticipant = {
  member_id: string;
  name: string;
  note: string | null;
};

export type GroupView = {
  id: string;
  name: string;
  label: string | null;
  ad_hoc: AdHocKind | null;
  components: MealPlanComponent[];
  everyone: boolean;
  participants: GroupParticipant[];
  guest_count: number;
  serves: number;
  cooking_servings: number | null;
  effective_cooking_servings: number;
  cook_minutes: number | null;
  to_buy: number;
  leftover_servings_available: number | null;
  revision: number;
};

export type OccasionView = {
  id: string;
  planned_on: string;
  slot: MealSlot;
  planned_time: string | null;
  effective_time: string | null;
  note: string | null;
  groups: GroupView[];
  absent_member_ids: string[];
  unaccounted_member_ids: string[];
  revision: number;
};

export type PlannerDay = {
  date: string;
  occasions: (OccasionView | null)[];
};

export type PlannerWeek = {
  week_start: string;
  usual_times: UsualTimes;
  members: PlannerMember[];
  days: PlannerDay[];
};

export type NewGroupComponent = (
  | { product_id: string }
  | { recipe_id: string }
  | { dish_recipe_id: string }
  | { ingredient_id: string }
  | { prepared_meal_id: string }
) & { amount: Amount };

export type NewGroup = {
  label?: string | null;
  ad_hoc?: AdHocKind | null;
  components?: NewGroupComponent[];
  everyone?: boolean;
  participants?: { member_id: string; note?: string | null }[];
  guest_count?: number;
  cooking_servings?: number | null;
};

export type GroupPatch = {
  label?: string | null;
  ad_hoc?: AdHocKind | null;
  components?: NewGroupComponent[];
  everyone?: boolean;
  participants?: { member_id: string; note?: string | null }[];
  guest_count?: number;
  cooking_servings?: number | null;
  revision: number;
};

export type OccasionPatch = {
  planned_time?: string | null;
  note?: string | null;
  revision: number;
};

export type Attendance =
  | { kind: 'eating'; group_id: string; note?: string | null }
  | { kind: 'elsewhere' }
  | { kind: 'unaccounted' };

export type PickerRow = {
  id: string;
  title: string;
  caption: string | null;
  concept: 'meal' | 'recipe' | 'food' | 'dish' | 'saved_meal' | 'out' | 'takeaway' | 'fend';
  section: 'matches' | 'quick';
  group: NewGroup;
};
