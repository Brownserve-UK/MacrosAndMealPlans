import type { MealPlanComponent } from '../../../api/client';
import type { GroupView, OccasionView, PlannerWeek } from './types';

export const MEMBERS = [
  { id: 'steve', name: 'Steve', initials: 'SB' },
  { id: 'sarah', name: 'Sarah', initials: 'SR' },
  { id: 'emily', name: 'Emily', initials: 'EB' },
  { id: 'jack', name: 'Jack', initials: 'JB' },
];

function singleComponent(name: string): MealPlanComponent {
  return {
    id: 'component-1',
    item_kind: 'product',
    product_id: 'product-1',
    item_name: name,
    amount: { kind: 'servings', value: 1 },
    nutrition: {},
    quality: 'known',
    preparation: { prepared: { kind: 'servings', value: '1' }, shortage: false },
    status: 'planned',
    subject_status: 'planned',
    position: 0,
    effective_cooking_servings: 1,
    revision: 1,
    needs_cooking: false,
  };
}

export function group(overrides: Partial<GroupView> = {}): GroupView {
  const name = overrides.name ?? 'Spaghetti bolognese';
  const components = overrides.components ?? [singleComponent(name)];
  return {
    id: 'group-1',
    name,
    label: null,
    ad_hoc: null,
    components,
    everyone: true,
    participants: [],
    guest_count: 0,
    guests: [],
    serves: 4,
    cook_minutes: 35,
    to_buy: 2,
    leftover_servings_available: null,
    revision: 1,
    ...overrides,
  };
}

export function occasion(overrides: Partial<OccasionView> = {}): OccasionView {
  return {
    id: 'occasion-1',
    planned_on: '2026-09-17',
    slot: 'dinner',
    planned_time: null,
    effective_time: '18:00',
    note: null,
    groups: [group()],
    cooking: [],
    absent_member_ids: [],
    unaccounted_member_ids: [],
    revision: 3,
    ...overrides,
  };
}

export function week(occasions: OccasionView[] = [occasion()]): PlannerWeek {
  const dates = Array.from({ length: 7 }, (_, index) => `2026-09-${String(14 + index).padStart(2, '0')}`);
  return {
    week_start: '2026-09-14',
    usual_times: { breakfast: '08:00', lunch: '12:30', dinner: '18:00', snacks: null },
    members: MEMBERS,
    days: dates.map((date) => ({
      date,
      occasions: (['breakfast', 'lunch', 'dinner', 'snacks'] as const).map(
        (slot) => occasions.find((candidate) => candidate.planned_on === date && candidate.slot === slot) ?? null,
      ),
    })),
  };
}
