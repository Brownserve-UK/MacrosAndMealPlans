import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MealPlanEntry, Product } from '../../api/client';
import { AddFoodDialog } from './AddFoodDialog';
import { todayIso } from './date';

const mocks = vi.hoisted(() => ({
  createConsumption: vi.fn(),
  createPlan: vi.fn(),
  updatePlan: vi.fn(),
  milk: {
    id: '10000000-0000-0000-0000-000000000001',
    name: 'Whole Milk',
    package_quantity: { amount: 1000, unit: 'ml' },
    nutrition: { basis: { amount: 100, unit: 'ml' }, energy_kcal: 64 },
  },
  curry: {
    id: '30000000-0000-0000-0000-000000000001',
    name: 'Chicken Curry',
    servings: 4,
  },
}));

const milk = mocks.milk as Product;

vi.mock('../../api/queries', () => ({
  useCreateConsumption: () => ({ isPending: false, mutateAsync: mocks.createConsumption }),
  useCreateMealPlanEntry: () => ({ isPending: false, mutateAsync: mocks.createPlan }),
  useUpdateMealPlanEntry: () => ({ isPending: false, mutateAsync: mocks.updatePlan }),
  useProducts: () => ({ data: { items: [mocks.milk] }, isLoading: false }),
  useRecipes: () => ({ data: { items: [mocks.curry] }, isLoading: false }),
  useUnits: () => ({
    data: [
      { code: 'g', label: 'gram', dimension: 'mass', convertible: true },
      { code: 'ml', label: 'millilitre', dimension: 'volume', convertible: true },
    ],
  }),
}));

function renderDialog(
  date: string,
  slot: 'breakfast' | 'lunch' | 'dinner' | 'snacks',
  kind: 'planned' | 'eaten' = 'eaten',
  entry: MealPlanEntry | null = null,
) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  const onClose = vi.fn();
  render(
    <QueryClientProvider client={queryClient}>
      <AddFoodDialog
        open
        onClose={onClose}
        memberId="20000000-0000-0000-0000-000000000001"
        date={date}
        slot={slot}
        kind={kind}
        entry={entry}
      />
    </QueryClientProvider>,
  );
  return onClose;
}

async function pickMilkAndEnterAmount() {
  const user = userEvent.setup();
  await user.click(screen.getByRole('combobox', { name: 'Product' }));
  await user.click(screen.getByRole('option', { name: 'Whole Milk' }));
  await user.type(screen.getByRole('textbox', { name: 'Amount' }), '250');
  return user;
}

describe('AddFoodDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('adds future food to the selected meal plan without offering a state toggle', async () => {
    mocks.createPlan.mockResolvedValue({});
    const onClose = renderDialog('2999-08-26', 'dinner', 'planned');
    expect(screen.queryByLabelText(/Planned meal time/)).not.toBeInTheDocument();
    const user = await pickMilkAndEnterAmount();

    expect(screen.queryByRole('button', { name: 'Eaten' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Add to meal' }));

    expect(mocks.createPlan).not.toHaveBeenCalled();
    await user.click(screen.getByRole('button', { name: 'Save meal' }));

    expect(mocks.createPlan).toHaveBeenCalledWith({
      planned_on: '2999-08-26',
      slot: 'dinner',
      components: [
        {
          product_id: milk.id,
          amount: { kind: 'measure', value: 250, unit: 'ml' },
        },
      ],
    });
    expect(onClose).toHaveBeenCalled();
  });

  it('logs food eaten today by default and keeps the dialog open to add more', async () => {
    mocks.createConsumption.mockResolvedValue({});
    const onClose = renderDialog(todayIso(), 'lunch');
    expect(screen.getByLabelText('Time eaten (optional)')).toHaveValue('');
    const user = await pickMilkAndEnterAmount();

    await user.click(screen.getByRole('button', { name: 'Add' }));

    expect(mocks.createConsumption).toHaveBeenCalledWith({
      member_id: '20000000-0000-0000-0000-000000000001',
      product_id: milk.id,
      slot: 'lunch',
      amount: { kind: 'measure', value: 250, unit: 'ml' },
      consumed_on: todayIso(),
      consumed_at: null,
    });
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByText('Whole Milk')).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'Product' })).toHaveValue('');
  });

  it('appends planned food to the existing meal slot and retains its time', async () => {
    mocks.updatePlan.mockResolvedValue({});
    const entry = {
      id: 'entry-1',
      scope: 'member' as const,
      member_id: 'member-1',
      subject_member_id: 'member-1',
      participants: [],
      guest_groups: [],
      planned_on: '2999-08-26',
      planned_time: '18:30',
      slot: 'dinner',
      status: 'planned',
      components: [
        {
          id: 'component-1',
          item_kind: 'product',
          product_id: 'product-existing',
          item_name: 'Oats',
          amount: { kind: 'measure', value: 80, unit: 'g' },
          position: 0,
          nutrition: {},
          quality: 'unknown',
          status: 'planned',
          preparation: { prepared: { kind: 'servings', value: '0' }, shortage: false },
          subject_status: 'planned' as const,
          revision: 1,
        },
      ],
      planned: { nutrition: {}, unknown_count: 1, partial_count: 0 },
      needs_attention: false,
      created_by: 'user-1',
      updated_by: 'user-1',
      revision: 4,
      created_at: '2026-08-26T10:00:00Z',
      updated_at: '2026-08-26T10:00:00Z',
    } satisfies MealPlanEntry;
    renderDialog('2999-08-26', 'dinner', 'planned', entry);
    expect(screen.queryByLabelText(/Planned meal time/)).not.toBeInTheDocument();
    const user = await pickMilkAndEnterAmount();

    await user.click(screen.getByRole('button', { name: 'Add to meal' }));
    await user.click(screen.getByRole('button', { name: 'Save meal' }));

    expect(mocks.updatePlan).toHaveBeenCalledWith({
      id: 'entry-1',
      revision: 4,
      body: {
        components: [
          {
            id: 'component-1',
            product_id: 'product-existing',
            amount: { kind: 'measure', value: 80, unit: 'g' },
          },
          {
            product_id: milk.id,
            amount: { kind: 'measure', value: 250, unit: 'ml' },
          },
        ],
      },
    });
    expect(mocks.createPlan).not.toHaveBeenCalled();
  });

  it('logs food against a meal chosen in the dialog', async () => {
    mocks.createConsumption.mockResolvedValue({});
    renderDialog(todayIso(), 'breakfast');
    const user = userEvent.setup();
    await user.click(screen.getByRole('combobox', { name: 'Meal' }));
    await user.click(screen.getByRole('option', { name: 'Lunch' }));
    await pickMilkAndEnterAmount();

    await user.click(screen.getByRole('button', { name: 'Add' }));

    expect(mocks.createConsumption).toHaveBeenCalledWith(
      expect.objectContaining({ slot: 'lunch' }),
    );
  });

  it('does not expose planned state in the food log', async () => {
    const onClose = renderDialog(todayIso(), 'breakfast');
    const user = await pickMilkAndEnterAmount();

    expect(screen.queryByRole('button', { name: 'Planned' })).not.toBeInTheDocument();
    mocks.createConsumption.mockResolvedValue({});
    await user.click(screen.getByRole('button', { name: 'Add' }));

    expect(mocks.createConsumption).toHaveBeenCalled();
    expect(mocks.createPlan).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  it('plans a recipe serving', async () => {
    mocks.createPlan.mockResolvedValue({});
    renderDialog('2999-08-26', 'dinner', 'planned');
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: 'Recipe' }));
    await user.click(screen.getByRole('combobox', { name: 'Recipe' }));
    await user.click(screen.getByRole('option', { name: 'Chicken Curry' }));
    const servings = screen.getByRole('spinbutton', { name: 'Servings' });
    await user.clear(servings);
    await user.type(servings, '2');

    await user.click(screen.getByRole('button', { name: 'Add to meal' }));
    await user.click(screen.getByRole('button', { name: 'Save meal' }));

    expect(mocks.createPlan).toHaveBeenCalledWith({
      planned_on: '2999-08-26',
      slot: 'dinner',
      components: [
        {
          recipe_id: '30000000-0000-0000-0000-000000000001',
          amount: { kind: 'servings', value: 2 },
        },
      ],
    });
  });
});
