import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MealPlanEntry, MealPlanWeek } from '../../api/client';
import { MyPlannerPage } from './MyPlannerPage';

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  remove: vi.fn(),
  optOut: vi.fn(),
  rejoin: vi.fn(),
}));

const WEEK_START = '2026-08-24';
const DAY = '2026-08-25';
const nutrition = { nutrition: {}, unknown_count: 0, partial_count: 0 };

function baseEntry(overrides: Partial<MealPlanEntry>): MealPlanEntry {
  return {
    id: 'entry',
    scope: 'member',
    member_id: 'me',
    subject_member_id: 'me',
    participants: [],
    guest_groups: [],
    opted_out: [],
    planned_on: DAY,
    planned_time: '18:30',
    slot: 'dinner',
    portioning: 'equal',
    status: 'planned',
    components: [{
      id: 'c1',
      item_kind: 'product',
      product_id: 'p1',
      item_name: 'Pasta bake',
      amount: { kind: 'measure', value: 300, unit: 'g' },
      position: 0,
      nutrition: {},
      quality: 'known',
      status: 'planned',
      subject_status: 'planned',
      preparation: { prepared: { kind: 'measure', value: '300', unit: 'g' }, shortage: false },
      revision: 1,
    }],
    planned: nutrition,
    needs_attention: false,
    created_by: 'u',
    updated_by: 'u',
    revision: 2,
    created_at: '2026-08-24T10:00:00Z',
    updated_at: '2026-08-24T10:00:00Z',
    ...overrides,
  } as MealPlanEntry;
}

let entries: MealPlanEntry[] = [];

function week(): MealPlanWeek {
  const days = Array.from({ length: 7 }, (_, index) => {
    const date = new Date('2026-08-24T00:00:00');
    date.setDate(date.getDate() + index);
    const iso = date.toISOString().slice(0, 10);
    return {
      date: iso,
      entries: iso === DAY ? entries : [],
      slots: ['breakfast', 'lunch', 'dinner', 'snacks'].map((slot) => ({ slot: slot as 'dinner', items: [], nutrition })),
      actual: nutrition,
      remaining_planned: nutrition,
      projected: nutrition,
    };
  });
  return {
    member_id: 'me',
    week_start: WEEK_START,
    week_end: '2026-08-30',
    days,
    actual: nutrition,
    remaining_planned: nutrition,
    projected: nutrition,
  };
}

vi.mock('@tanstack/react-router', () => ({ useNavigate: () => mocks.navigate }));
vi.mock('../../auth/AuthProvider', () => ({ useAuth: () => ({ principal: { member_id: 'me' } }) }));
vi.mock('../../api/queries', () => ({
  useMealPlanWeek: () => ({ data: week(), isLoading: false, isError: false, refetch: vi.fn() }),
  useMeta: () => ({ data: { nutrient_directions: {} } }),
  useDeleteMealPlanEntry: () => ({ mutateAsync: mocks.remove, isPending: false }),
  useOptOutOfMeal: () => ({ mutateAsync: mocks.optOut, isPending: false }),
  useRejoinMeal: () => ({ mutateAsync: mocks.rejoin, isPending: false }),
}));
vi.mock('./NutritionSummary', () => ({ DayWeekNutrition: () => <div>nutrition panel</div> }));
vi.mock('./MealEditorDialog', () => ({ MealEditorDialog: () => <div>Meal editor</div> }));

describe('MyPlannerPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    entries = [];
  });

  it('shows a personal meal with edit controls', () => {
    entries = [baseEntry({ id: 'mine', scope: 'member' })];
    render(<MyPlannerPage weekStart={WEEK_START} day={DAY} />);
    expect(screen.getByText('Pasta bake')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Edit meal' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Delete' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Opt out' })).not.toBeInTheDocument();
  });

  it('shows a household meal read-only with an opt-out action', async () => {
    entries = [baseEntry({
      id: 'shared',
      scope: 'household',
      member_id: null,
      participants: [{
        member_id: 'me',
        display_name: 'Me',
        status: 'planned',
        allocations: [{ component_id: 'c1', allocated: { kind: 'measure', value: '150', unit: 'g' }, status: 'planned' }],
        nutrition,
      }],
    })];
    render(<MyPlannerPage weekStart={WEEK_START} day={DAY} />);
    expect(screen.getByText('Household')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Edit meal' })).not.toBeInTheDocument();
    await userEvent.setup().click(screen.getByRole('button', { name: 'Opt out' }));
    expect(mocks.optOut).toHaveBeenCalledWith({ id: 'shared', revision: 2 });
  });

  it('collapses an opted-out household meal to a Join action and frees the slot', async () => {
    entries = [baseEntry({
      id: 'shared',
      scope: 'household',
      member_id: null,
      opted_out: [{ member_id: 'me', created_by: 'u', created_at: '2026-08-24T10:00:00Z' }],
    })];
    render(<MyPlannerPage weekStart={WEEK_START} day={DAY} />);
    expect(screen.getByText('Opted out')).toBeInTheDocument();
    expect(screen.queryByText('Pasta bake')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add food' })).toBeInTheDocument();
    await userEvent.setup().click(screen.getByRole('button', { name: 'Join meal' }));
    expect(mocks.rejoin).toHaveBeenCalledWith({ id: 'shared', revision: 2 });
  });
});
