import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MealPlanComponent, StockItem } from '../../api/client';
import type { PlannedCook } from '../meal-plan/CookDialog';
import { addDays, startOfWeekIso, todayIso } from '../meal-plan/date';
import type { GroupView, OccasionView, PlannerWeek } from '../meal-plan/planner/types';
import { KitchenPage } from './KitchenPage';

const mocks = vi.hoisted(() => ({
  week: undefined as PlannerWeek | undefined,
  stock: [] as StockItem[],
}));

vi.mock('../../hooks/useHouseholdTimeZone', () => ({ useHouseholdTimeZone: () => 'UTC' }));
vi.mock('../../api/queries', () => ({
  usePlannerWeek: () => ({
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
  CookDialog: ({ cook }: { cook: PlannedCook }) => (
    <div role="dialog">{`Cook ${cook.entryId} ${cook.componentId} ${cook.planned}`}</div>
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

const today = todayIso('UTC');
const tomorrow = addDays(today, 1);

function component(
  id: string,
  name: string,
  ref: { item_kind: 'recipe'; recipe_id: string } | { item_kind: 'product'; product_id: string } | { item_kind: 'dish'; dish_recipe_id: string },
  extra: Partial<MealPlanComponent> = {},
): MealPlanComponent {
  return {
    ...ref,
    id,
    item_name: name,
    amount: { kind: 'servings', value: 4 },
    needs_cooking: ref.item_kind === 'recipe',
    nutrition: {},
    position: 0,
    preparation: { prepared: { amount: 0, unit: 'serving' }, shortage: false },
    quality: 'known',
    revision: 1,
    status: 'planned',
    subject_status: 'planned',
    ...extra,
  } as MealPlanComponent;
}

function group(id: string, name: string, components: MealPlanComponent[], extra: Partial<GroupView> = {}): GroupView {
  return {
    id,
    name,
    label: null,
    ad_hoc: null,
    components,
    everyone: true,
    participants: [],
    guest_count: 0,
    guests: [],
    serves: 4,
    cooking_servings: null,
    effective_cooking_servings: 4,
    cook_minutes: 35,
    to_buy: 0,
    leftover_servings_available: null,
    revision: 1,
    ...extra,
  };
}

function occasion(date: string, slot: OccasionView['slot'], groups: GroupView[]): OccasionView {
  return {
    id: `${date}-${slot}`,
    planned_on: date,
    slot,
    planned_time: null,
    effective_time: slot === 'dinner' ? '18:00' : '08:00',
    note: null,
    groups,
    absent_member_ids: [],
    unaccounted_member_ids: [],
    revision: 1,
  };
}

function plannerWeek(occasions: OccasionView[]): PlannerWeek {
  const days = Array.from({ length: 7 }, (_, index) => {
    const date = addDays(today, index);
    const slots: OccasionView['slot'][] = ['breakfast', 'lunch', 'dinner', 'snacks'];
    return {
      date,
      occasions: slots.map((slot) => occasions.find((candidate) => candidate.planned_on === date && candidate.slot === slot) ?? null),
    };
  });
  return {
    week_start: startOfWeekIso(today),
    usual_times: { breakfast: '08:00', lunch: '12:30', dinner: '18:00', snacks: null },
    members: [],
    days,
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

function fullWeek(): PlannerWeek {
  return plannerWeek([
    occasion(today, 'dinner', [
      group('dinner-group', 'Chicken and rice', [
        component('recipe-to-cook', 'Chicken and rice', { item_kind: 'recipe', recipe_id: 'recipe-1' }),
      ]),
      group('cooked-group', 'Already made curry', [
        component('already-cooked', 'Already made curry', { item_kind: 'recipe', recipe_id: 'recipe-2' }, {
          cooked: { prepared_at: `${today}T12:00:00Z`, prepared_batch_id: 'batch-2', servings_produced: 2, revision: 1 },
        }),
      ]),
      group('leftovers-group', 'Leftover lasagne', [
        component('lasagne', 'Leftover lasagne', { item_kind: 'dish', dish_recipe_id: 'recipe-3' }),
      ]),
      group('out-group', 'Eating out', [], { ad_hoc: 'eating_out', label: 'Eating out' }),
    ]),
    occasion(tomorrow, 'breakfast', [
      group('pancakes-group', 'Pancakes', [
        component('pancakes', 'Pancakes', { item_kind: 'recipe', recipe_id: 'recipe-4' }),
      ], { cook_minutes: 20, to_buy: 1 }),
      group('pizza-group', 'Margherita pizza', [
        component('pizza', 'Margherita pizza', { item_kind: 'product', product_id: 'product-1' }),
      ], { cook_minutes: null }),
    ]),
  ]);
}

describe('KitchenPage', () => {
  beforeEach(() => {
    mocks.week = fullWeek();
    mocks.stock = [
      cookedStock('one', 1),
      cookedStock('two', 1.5),
      cookedStock('cold', 3, 'chilled'),
    ];
  });

  it('lists what still needs cooking today with the cooking count', () => {
    render(<KitchenPage />);

    expect(screen.getByRole('heading', { name: 'Kitchen' })).toBeInTheDocument();
    const todaySection = screen.getByRole('heading', { name: 'Today' }).parentElement as HTMLElement;
    expect(within(todaySection).getByText('Chicken and rice')).toBeInTheDocument();
    expect(within(todaySection).getByText('Dinner · 18:00 · cooking 4 · 35 mins')).toBeInTheDocument();
    expect(screen.queryByText('Already made curry')).not.toBeInTheDocument();
    expect(screen.queryByText('Leftover lasagne')).not.toBeInTheDocument();
    expect(screen.queryByText('Eating out')).not.toBeInTheDocument();

    const leftovers = screen.getByRole('heading', { name: 'Leftovers to put away' }).parentElement
      ?.parentElement;
    expect(leftovers).not.toBeNull();
    expect(within(leftovers as HTMLElement).getByText('Bolognese')).toBeInTheDocument();
    expect(within(leftovers as HTMLElement).getByText('2.5 servings')).toBeInTheDocument();
  });

  it('shows the rest of the week under coming up with anything still to buy', () => {
    render(<KitchenPage />);

    const comingUp = screen.getByRole('heading', { name: 'Coming up' }).parentElement as HTMLElement;
    expect(within(comingUp).getByText('Pancakes')).toBeInTheDocument();
    expect(within(comingUp).getByText(/Breakfast · cooking 4 · 20 mins/)).toBeInTheDocument();
    expect(within(comingUp).getByText('· 1 to buy')).toBeInTheDocument();
    expect(within(comingUp).getByText('Margherita pizza')).toBeInTheDocument();
    expect(within(comingUp).getByText('Product')).toBeInTheDocument();
    expect(within(comingUp).queryByRole('button', { name: 'Cooked it' })).not.toBeInTheDocument();
  });

  it('shows short empty states when there is nothing to do', () => {
    mocks.week = plannerWeek([]);
    mocks.stock = [];
    render(<KitchenPage />);

    expect(screen.getByText('Nothing left to cook today.')).toBeInTheDocument();
    expect(screen.getByText('Nothing else to cook this week.')).toBeInTheDocument();
    expect(screen.getByText('Nothing to put away.')).toBeInTheDocument();
  });

  it('opens each existing cooking dialog from its action', async () => {
    const user = userEvent.setup();
    render(<KitchenPage />);

    await user.click(screen.getByRole('button', { name: 'Cooked it' }));
    expect(screen.getByRole('dialog')).toHaveTextContent('Cook dinner-group recipe-to-cook 4');

    await user.click(screen.getByRole('button', { name: 'Put away' }));
    expect(screen.getAllByRole('dialog')[1]).toHaveTextContent('Put away Bolognese 2.5');

    await user.click(screen.getByRole('button', { name: 'Cooked something' }));
    expect(screen.getAllByRole('dialog')[2]).toHaveTextContent('Cooked something dialog');
  });
});
