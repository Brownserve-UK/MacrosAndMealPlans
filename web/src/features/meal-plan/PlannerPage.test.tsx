import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { PlannerMeal, PlannerWeek } from '../../api/client';
import { PlannerPage } from './PlannerPage';

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  leave: vi.fn(),
  permissions: ['household:write'] as string[],
  week: undefined as PlannerWeek | undefined,
}));

vi.mock('@tanstack/react-router', () => ({ useNavigate: () => mocks.navigate }));
vi.mock('../../auth/AuthProvider', () => ({
  useAuth: () => ({ principal: { member_id: 'me', permissions: mocks.permissions } }),
}));
vi.mock('../../hooks/useHouseholdTimeZone', () => ({ useHouseholdTimeZone: () => 'UTC' }));
vi.mock('../../api/queries', () => ({
  useHouseholdPlannerWeek: () => ({ data: mocks.week, isLoading: false, isError: false, refetch: vi.fn() }),
  useOptOutOfMeal: () => ({ mutateAsync: mocks.leave, isPending: false }),
}));
vi.mock('./MealEditorDialog', () => ({ MealEditorDialog: ({ meal }: { meal: PlannerMeal | null }) => <div>{meal ? `Editing ${meal.id}` : 'Planning meal'}</div> }));

function meal(overrides: Partial<PlannerMeal>): PlannerMeal {
  return {
    id: 'meal-1',
    mine: true,
    scope: 'member',
    member_id: 'me',
    planned_on: '2026-09-15',
    planned_time: '12:30',
    slot: 'lunch',
    status: 'planned',
    foods: [{
      id: 'food-1',
      item_kind: 'product',
      product_id: 'product-1',
      item_name: 'Soup',
      amount: { kind: 'measure', value: 400, unit: 'g' },
      shortage: false,
      needs_cooking: false,
    }],
    people: [{ member_id: 'me', display_name: 'Sam Brown', status: 'planned', allocations: [], can_record: true }],
    guest_groups: [],
    opted_out: [],
    can_opt_out: false,
    can_join: false,
    capabilities: { can_edit: true, can_delete: true, can_record_guests: false },
    revision: 1,
    ...overrides,
  };
}

function plannerWeek(meals: PlannerMeal[]): PlannerWeek {
  const nutrition = { nutrition: { energy_kcal: 1709 }, unknown_count: 0, partial_count: 0 };
  return {
    week_start: '2026-09-14',
    week_end: '2026-09-20',
    meals,
    days: [{
      date: '2026-09-15',
      actual: nutrition,
      remaining_planned: nutrition,
      projected: nutrition,
      target: { energy_kcal: 2100 },
      calorie_direction: 'at_most',
    }],
    actual: nutrition,
    remaining_planned: nutrition,
    projected: nutrition,
    target: { energy_kcal: 2100 },
    calorie_direction: 'at_most',
  };
}

describe('PlannerPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.permissions = ['household:write'];
    mocks.week = plannerWeek([
      meal({}),
      meal({
        id: 'meal-2',
        mine: false,
        member_id: 'morgan',
        owner_name: 'Morgan Lee',
        foods: [{
          id: 'food-2',
          item_kind: 'product',
          product_id: 'product-2',
          item_name: 'Salad',
          amount: { kind: 'measure', value: 300, unit: 'g' },
          shortage: false,
          needs_cooking: false,
        }],
        people: [{ member_id: 'morgan', display_name: 'Morgan Lee', status: 'planned', allocations: [], can_record: false }],
        capabilities: { can_edit: false, can_delete: false, can_record_guests: false },
      }),
    ]);
  });

  it('shows one day with compact nutrition and the full-contrast household roster', () => {
    render(<PlannerPage weekStart="2026-09-14" day="2026-09-15" />);
    expect(screen.getByText('1,709 kcal projected of 2,100')).toBeInTheDocument();
    expect(screen.getByText('Soup')).toBeInTheDocument();
    expect(screen.getByText('12:30 · just you')).toBeInTheDocument();
    expect(screen.getByText('Also in lunch')).toBeInTheDocument();
    expect(screen.getByText('Salad')).toBeInTheDocument();
    expect(screen.getByText('Morgan Lee')).toBeInTheDocument();
    expect(screen.queryByText('Mine')).not.toBeInTheDocument();
    expect(screen.queryByText('Household')).not.toBeInTheDocument();
  });

  it('does not render other meals without household write permission', () => {
    mocks.permissions = [];
    render(<PlannerPage weekStart="2026-09-14" day="2026-09-15" />);
    expect(screen.getByText('Soup')).toBeInTheDocument();
    expect(screen.queryByText('Salad')).not.toBeInTheDocument();
  });

  it('opens the existing editor for an editable meal', async () => {
    render(<PlannerPage weekStart="2026-09-14" day="2026-09-15" />);
    await userEvent.setup().click(screen.getByText('Soup'));
    expect(screen.getByText('Editing meal-1')).toBeInTheDocument();
  });

  it('leaves a household meal through the existing opt-out mutation', async () => {
    mocks.week = plannerWeek([meal({
      id: 'shared-meal',
      scope: 'household',
      can_opt_out: true,
      capabilities: { can_edit: false, can_delete: false, can_record_guests: false },
    })]);
    render(<PlannerPage weekStart="2026-09-14" day="2026-09-15" />);
    const user = userEvent.setup();
    await user.click(screen.getByText('Soup'));
    await user.click(screen.getByRole('button', { name: 'Leave meal' }));
    expect(mocks.leave).toHaveBeenCalledWith({ id: 'shared-meal', revision: 1 });
  });
});
