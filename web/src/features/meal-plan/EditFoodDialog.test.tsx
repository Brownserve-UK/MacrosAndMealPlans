import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MealItem, Product } from '../../api/client';
import { EditFoodDialog } from './EditFoodDialog';

const mocks = vi.hoisted(() => ({
  updateConsumption: vi.fn(),
  deleteConsumption: vi.fn(),
  updateEntry: vi.fn(),
  deleteEntry: vi.fn(),
  markNotEaten: vi.fn(),
  reopen: vi.fn(),
  milk: {
    id: 'product-2',
    name: 'Whole Milk',
    package_quantity: { amount: 1000, unit: 'ml' },
    nutrition: { basis: { amount: 100, unit: 'ml' }, energy_kcal: 64 },
  },
}));

vi.mock('../../api/queries', () => ({
  useProduct: () => ({ data: mocks.milk as Product, isLoading: false }),
  useProducts: () => ({ data: { items: [mocks.milk] }, isLoading: false }),
  useUpdateConsumption: () => ({ isPending: false, mutateAsync: mocks.updateConsumption }),
  useDeleteConsumption: () => ({ isPending: false, mutateAsync: mocks.deleteConsumption }),
  useUpdateMealPlanEntry: () => ({ isPending: false, mutateAsync: mocks.updateEntry }),
  useDeleteMealPlanEntry: () => ({ isPending: false, mutateAsync: mocks.deleteEntry }),
  useMarkMealPlanComponentNotEaten: () => ({ isPending: false, mutateAsync: mocks.markNotEaten }),
  useReopenMealPlanComponent: () => ({ isPending: false, mutateAsync: mocks.reopen }),
  useUnits: () => ({
    data: [
      { code: 'g', label: 'gram', dimension: 'mass', convertible: true },
      { code: 'ml', label: 'millilitre', dimension: 'volume', convertible: true },
    ],
  }),
}));

const plannedItem: MealItem = {
  kind: 'planned',
  entry_id: 'entry-1',
  component_id: 'component-1',
  status: 'planned',
  product_id: 'product-2',
  product_name: 'Whole Milk',
  amount: { kind: 'measure', value: 250, unit: 'ml' },
  nutrition: { energy_kcal: 160 },
  quality: 'known',
  needs_attention: false,
  revision: 3,
};

const loggedItem: MealItem = {
  kind: 'logged',
  record_id: 'record-9',
  linked_record_id: 'record-9',
  status: 'eaten',
  product_id: 'product-2',
  product_name: 'Whole Milk',
  amount: { kind: 'measure', value: 250, unit: 'ml' },
  nutrition: { energy_kcal: 160 },
  quality: 'known',
  needs_attention: false,
  revision: 1,
};

function renderDialog(item: MealItem) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  const onClose = vi.fn();
  render(
    <QueryClientProvider client={queryClient}>
      <EditFoodDialog
        open
        onClose={onClose}
        item={item}
        date="2026-08-25"
        slot="breakfast"
        memberId="member-1"
      />
    </QueryClientProvider>,
  );
  return onClose;
}

describe('EditFoodDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('marks a still-planned item not eaten', async () => {
    mocks.markNotEaten.mockResolvedValue({});
    const onClose = renderDialog(plannedItem);
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: 'Not eaten' }));

    expect(mocks.markNotEaten).toHaveBeenCalledWith({
      id: 'entry-1',
      componentId: 'component-1',
      revision: 3,
    });
    expect(onClose).toHaveBeenCalled();
  });

  it('saves an amount change for directly logged food', async () => {
    mocks.updateConsumption.mockResolvedValue({});
    const onClose = renderDialog(loggedItem);
    const user = userEvent.setup();

    const amount = screen.getByRole('textbox', { name: 'Amount' });
    await user.clear(amount);
    await user.type(amount, '300');
    await user.click(screen.getByRole('button', { name: 'Save changes' }));

    expect(mocks.updateConsumption).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 'record-9',
        revision: 1,
        body: expect.objectContaining({ amount: { kind: 'measure', value: 300, unit: 'ml' } }),
      }),
    );
    expect(onClose).toHaveBeenCalled();
  });

  it('removes directly logged food', async () => {
    mocks.deleteConsumption.mockResolvedValue({});
    const onClose = renderDialog(loggedItem);
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: 'Remove' }));

    expect(mocks.deleteConsumption).toHaveBeenCalledWith({
      id: 'record-9',
      revision: 1,
      memberId: 'member-1',
    });
    expect(onClose).toHaveBeenCalled();
  });
});
