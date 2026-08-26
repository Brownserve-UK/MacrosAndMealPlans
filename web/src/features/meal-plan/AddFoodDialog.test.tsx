import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Product } from '../../api/client';
import { AddFoodDialog } from './AddFoodDialog';
import { todayIso } from './date';

const mocks = vi.hoisted(() => ({
  createConsumption: vi.fn(),
  createPlan: vi.fn(),
  milk: {
    id: '10000000-0000-0000-0000-000000000001',
    name: 'Whole Milk',
    package_quantity: { amount: 1000, unit: 'ml' },
    nutrition: { basis: { amount: 100, unit: 'ml' }, energy_kcal: 64 },
  },
}));

const milk = mocks.milk as Product;

vi.mock('../../api/queries', () => ({
  useCreateConsumption: () => ({ isPending: false, mutateAsync: mocks.createConsumption }),
  useCreateMealPlanEntry: () => ({ isPending: false, mutateAsync: mocks.createPlan }),
  useProducts: () => ({ data: { items: [mocks.milk] }, isLoading: false }),
  useUnits: () => ({
    data: [
      { code: 'g', label: 'gram', dimension: 'mass', convertible: true },
      { code: 'ml', label: 'millilitre', dimension: 'volume', convertible: true },
    ],
  }),
}));

function renderDialog(date: string, slot: 'breakfast' | 'lunch' | 'dinner' | 'snacks') {
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
    const onClose = renderDialog('2999-08-26', 'dinner');
    const user = await pickMilkAndEnterAmount();

    expect(screen.queryByRole('button', { name: 'Eaten' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Add to plan' }));

    expect(mocks.createPlan).toHaveBeenCalledWith({
      planned_on: '2999-08-26',
      planned_time: null,
      slot: 'dinner',
      components: [
        {
          product_id: milk.id,
          amount: { kind: 'measure', value: 250, unit: 'ml' },
        },
      ],
    });
    expect(onClose).not.toHaveBeenCalled();

    await user.click(screen.getByRole('button', { name: 'Done' }));
    expect(onClose).toHaveBeenCalled();
  });

  it('logs food eaten today by default and keeps the dialog open to add more', async () => {
    mocks.createConsumption.mockResolvedValue({});
    const onClose = renderDialog(todayIso(), 'lunch');
    const user = await pickMilkAndEnterAmount();

    await user.click(screen.getByRole('button', { name: 'Add' }));

    expect(mocks.createConsumption).toHaveBeenCalledWith(
      expect.objectContaining({
        member_id: '20000000-0000-0000-0000-000000000001',
        product_id: milk.id,
        slot: 'lunch',
        amount: { kind: 'measure', value: 250, unit: 'ml' },
        consumed_on: todayIso(),
      }),
    );
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByText('Whole Milk')).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'Product' })).toHaveValue('');
  });

  it('switches to planned when eaten is not what happened', async () => {
    const onClose = renderDialog(todayIso(), 'breakfast');
    const user = await pickMilkAndEnterAmount();

    await user.click(screen.getByRole('button', { name: 'Planned' }));
    mocks.createPlan.mockResolvedValue({});
    await user.click(screen.getByRole('button', { name: 'Add to plan' }));

    expect(mocks.createPlan).toHaveBeenCalledWith(
      expect.objectContaining({ planned_on: todayIso(), slot: 'breakfast' }),
    );
    expect(onClose).not.toHaveBeenCalled();
  });
});
