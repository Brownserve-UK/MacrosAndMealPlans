import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { PlannerMeal, PlannerWeek } from '../../api/client';
import { PlannerPage } from './PlannerPage';

const mocks = vi.hoisted(() => ({
  deleteMeal: vi.fn(),
  join: vi.fn(),
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
  useDeleteMealPlanEntry: () => ({ mutateAsync: mocks.deleteMeal, isPending: false }),
  useOptOutOfMeal: () => ({ mutateAsync: mocks.leave, isPending: false }),
  useRejoinMeal: () => ({ mutateAsync: mocks.join, isPending: false }),
}));
vi.mock('./MealEditorDialog', () => ({ MealEditorDialog: ({ meal }: { meal: PlannerMeal | null }) => <div>{meal ? `Editing ${meal.id}` : 'Planning meal'}</div> }));
vi.mock('./GuidedMealDialog', () => ({ GuidedMealDialog: () => <div>Guided planning</div> }));

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

  it('opens the meal sheet from the user meal, then opens the editor from its action', async () => {
    render(<PlannerPage weekStart="2026-09-14" day="2026-09-15" />);
    const user = userEvent.setup();
    await user.click(screen.getByText('Soup'));
    const dialog = within(screen.getByRole('dialog'));
    expect(dialog.getByRole('heading')).toHaveTextContent('Soup');
    expect(dialog.getByText('Lunch · Tuesday 15 September · 12:30')).toBeInTheDocument();
    await user.click(dialog.getByRole('button', { name: 'Edit meal' }));
    expect(screen.getByText('Editing meal-1')).toBeInTheDocument();
  });

  it('opens the meal sheet from another member roster row', async () => {
    render(<PlannerPage weekStart="2026-09-14" day="2026-09-15" />);
    await userEvent.setup().click(screen.getByText('Salad'));
    const dialog = within(screen.getByRole('dialog'));
    expect(dialog.getByRole('heading')).toHaveTextContent('Salad');
    expect(dialog.getByText('Morgan Lee')).toBeInTheDocument();
  });

  it('opens the guided dialog when creating a meal', async () => {
    render(<PlannerPage weekStart="2026-09-14" day="2026-09-15" />);
    await userEvent.setup().click(screen.getByText('Plan breakfast'));
    expect(screen.getByText('Guided planning')).toBeInTheDocument();
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
    await user.click(screen.getByRole('button', { name: 'Leave this meal' }));
    await user.click(screen.getByRole('button', { name: 'Leave meal' }));
    expect(mocks.leave).toHaveBeenCalledWith({ id: 'shared-meal', revision: 1 });
  });

  it('joins a meal from the sheet', async () => {
    mocks.week = plannerWeek([meal({
      id: 'joinable-meal',
      mine: false,
      can_join: true,
      capabilities: { can_edit: false, can_delete: false, can_record_guests: false },
    })]);
    render(<PlannerPage weekStart="2026-09-14" day="2026-09-15" />);
    const user = userEvent.setup();
    await user.click(screen.getByText('Soup'));
    await user.click(screen.getByRole('button', { name: 'Join this meal' }));
    expect(mocks.join).toHaveBeenCalledWith({ id: 'joinable-meal', revision: 1 });
  });

  it('deletes a meal from the sheet through the existing confirmation', async () => {
    render(<PlannerPage weekStart="2026-09-14" day="2026-09-15" />);
    const user = userEvent.setup();
    await user.click(screen.getByText('Soup'));
    await user.click(screen.getByRole('button', { name: 'Delete meal' }));
    expect(screen.getByText('Delete this meal?')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Delete meal' }));
    expect(mocks.deleteMeal).toHaveBeenCalledWith({ id: 'meal-1', revision: 1 });
  });
});
