import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { PlannerMeal, PlannerWeek, StockItem } from '../../api/client';
import { startOfWeekIso, todayIso } from '../meal-plan/date';
import { KitchenPage } from './KitchenPage';

const mocks = vi.hoisted(() => ({
  week: undefined as PlannerWeek | undefined,
  stock: [] as StockItem[],
}));

vi.mock('../../hooks/useHouseholdTimeZone', () => ({ useHouseholdTimeZone: () => 'UTC' }));
vi.mock('../../api/queries', () => ({
  useHouseholdPlannerWeek: () => ({
    data: mocks.week,
    isLoading: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
  }),
  useStock: () => ({
    data: { items: mocks.stock },
    isLoading: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
  }),
}));
vi.mock('../meal-plan/CookDialog', () => ({
  CookDialog: ({ meal, componentId }: { meal: PlannerMeal; componentId?: string }) => (
    <div role="dialog">{`Cook ${meal.id} ${componentId}`}</div>
  ),
}));
vi.mock('../stock/MoveCookedDialog', () => ({
  MoveCookedDialog: ({ name, available }: { name: string; available: number }) => (
    <div role="dialog">{`Put away ${name} ${available}`}</div>
  ),
}));
vi.mock('../meal-plan/CookSomethingDialog', () => ({
  CookSomethingDialog: () => <div role="dialog">Cooked something dialog</div>,
}));

function meal(): PlannerMeal {
  const today = todayIso('UTC');
  return {
    id: 'dinner',
    mine: true,
    scope: 'member',
    member_id: 'me',
    planned_on: today,
    planned_time: '18:00',
    slot: 'dinner',
    status: 'planned',
    foods: [
      {
        id: 'recipe-to-cook',
        item_kind: 'recipe',
        recipe_id: 'recipe-1',
        item_name: 'Chicken and rice',
        amount: { kind: 'servings', value: 2 },
        shortage: false,
        needs_cooking: true,
      },
      {
        id: 'already-cooked',
        item_kind: 'recipe',
        recipe_id: 'recipe-2',
        item_name: 'Already made curry',
        amount: { kind: 'servings', value: 2 },
        shortage: false,
        needs_cooking: true,
        cooked: {
          prepared_at: `${today}T12:00:00Z`,
          prepared_batch_id: 'batch-2',
          servings_produced: 2,
          revision: 1,
        },
      },
    ],
    people: [],
    guest_groups: [],
    opted_out: [],
    can_opt_out: false,
    can_join: false,
    capabilities: { can_edit: true, can_delete: true, can_record_guests: false },
    revision: 1,
  };
}

function plannerWeek(meals: PlannerMeal[]): PlannerWeek {
  const today = todayIso('UTC');
  const weekStart = startOfWeekIso(today);
  const nutrition = { nutrition: {}, unknown_count: 0, partial_count: 0 };
  return {
    week_start: weekStart,
    week_end: weekStart,
    meals,
    days: [],
    actual: nutrition,
    remaining_planned: nutrition,
    projected: nutrition,
  };
}

function cookedStock(
  id: string,
  servings: number,
  storageLocation: StockItem['storage_location'] = 'ambient',
): StockItem {
  return {
    id,
    prepared_batch_id: `batch-${id}`,
    prepared_recipe_id: 'leftovers-recipe',
    prepared_batch_name: 'Bolognese',
    level: { mode: 'exact', quantity: { amount: servings, unit: 'serving' } },
    storage_location: storageLocation,
    subject_kind: 'prepared_portion',
    tracking_mode: 'exact',
    revision: 1,
    created_at: '2026-09-16T12:00:00Z',
    updated_at: '2026-09-16T12:00:00Z',
  };
}

describe('KitchenPage', () => {
  beforeEach(() => {
    mocks.week = plannerWeek([meal()]);
    mocks.stock = [
      cookedStock('one', 1),
      cookedStock('two', 1.5),
      cookedStock('cold', 3, 'chilled'),
    ];
  });

  it('shows uncooked planned recipes and ambient cooked portions', () => {
    render(<KitchenPage />);

    expect(screen.getByRole('heading', { name: 'Kitchen' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Today' })).toBeInTheDocument();
    expect(screen.getByText('Chicken and rice')).toBeInTheDocument();
    expect(screen.getByText('18:00 · Dinner')).toBeInTheDocument();
    expect(screen.queryByText('Already made curry')).not.toBeInTheDocument();

    const leftovers = screen.getByRole('heading', { name: 'Leftovers to put away' }).parentElement
      ?.parentElement;
    expect(leftovers).not.toBeNull();
    expect(within(leftovers as HTMLElement).getByText('Bolognese')).toBeInTheDocument();
    expect(within(leftovers as HTMLElement).getByText('2.5 servings')).toBeInTheDocument();
  });

  it('shows short empty states when there is nothing to do', () => {
    mocks.week = plannerWeek([]);
    mocks.stock = [];
    render(<KitchenPage />);

    expect(screen.getByText('Nothing left to cook today.')).toBeInTheDocument();
    expect(screen.getByText('Nothing to put away.')).toBeInTheDocument();
  });

  it('opens each existing cooking dialog from its action', async () => {
    const user = userEvent.setup();
    render(<KitchenPage />);

    await user.click(screen.getByRole('button', { name: 'Cooked it' }));
    expect(screen.getByRole('dialog')).toHaveTextContent('Cook dinner recipe-to-cook');

    await user.click(screen.getByRole('button', { name: 'Put away' }));
    expect(screen.getAllByRole('dialog')[1]).toHaveTextContent('Put away Bolognese 2.5');

    await user.click(screen.getByRole('button', { name: 'Cooked something' }));
    expect(screen.getAllByRole('dialog')[2]).toHaveTextContent('Cooked something dialog');
  });
});
