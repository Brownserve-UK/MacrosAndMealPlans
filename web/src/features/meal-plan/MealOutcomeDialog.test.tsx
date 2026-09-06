import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { PlannerMeal } from '../../api/client';
import { MealOutcomeDialog } from './MealOutcomeDialog';

const mocks = vi.hoisted(() => ({ review: vi.fn(), moveStock: vi.fn(), stockItems: [] as unknown[] }));

vi.mock('../../api/queries', () => ({
  useReviewMealOutcomes: () => ({ mutateAsync: mocks.review, isPending: false }),
  useStock: () => ({ data: { items: mocks.stockItems } }),
  useUpdateStockItem: () => ({ mutateAsync: mocks.moveStock, isPending: false }),
}));

function mealWith(overrides: Partial<PlannerMeal>): PlannerMeal {
  return {
    id: 'meal-1',
    scope: 'household',
    planned_on: '2026-08-25',
    planned_time: '18:30',
    slot: 'dinner',
    status: 'planned',
    foods: [{ id: 'c1', item_kind: 'product', product_id: 'p1', item_name: 'Chilli', amount: { kind: 'measure', value: 600, unit: 'g' }, shortage: false }],
    people: [
      { member_id: 'm1', display_name: 'Alex', status: 'planned', can_record: true, allocations: [{ component_id: 'c1', allocated: { kind: 'measure', value: '300', unit: 'g' }, status: 'planned' }] },
      { member_id: 'm2', display_name: 'Morgan', status: 'eaten', can_record: true, allocations: [{ component_id: 'c1', allocated: { kind: 'measure', value: '300', unit: 'g' }, status: 'eaten' }] },
    ],
    guest_groups: [],
    opted_out: [],
    can_opt_out: false,
    can_join: false,
    capabilities: { can_edit: true, can_delete: true, can_record_guests: true },
    revision: 4,
    ...overrides,
  } as PlannerMeal;
}

function cookedMeal(servingsProduced: number): PlannerMeal {
  return mealWith({
    foods: [{
      id: 'c1',
      item_kind: 'recipe',
      recipe_id: 'r1',
      item_name: 'Curry',
      amount: { kind: 'servings', value: 4 },
      cooked: { prepared_batch_id: 'b1', prepared_at: '2026-08-25T17:00:00Z', servings_produced: servingsProduced },
      shortage: false,
    }],
    people: [
      { member_id: 'm1', display_name: 'Alex', status: 'planned', can_record: true, allocations: [{ component_id: 'c1', allocated: { kind: 'servings', value: '1' }, status: 'planned' }] },
      { member_id: 'm2', display_name: 'Morgan', status: 'planned', can_record: true, allocations: [{ component_id: 'c1', allocated: { kind: 'servings', value: '1' }, status: 'planned' }] },
    ],
  } as Partial<PlannerMeal>);
}

function renderDialog(meal: PlannerMeal) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  const onClose = vi.fn();
  render(
    <QueryClientProvider client={qc}>
      <MealOutcomeDialog meal={meal} onClose={onClose} />
    </QueryClientProvider>,
  );
  return onClose;
}

describe('MealOutcomeDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.stockItems = [];
  });

  it('only offers the still-pending participants and records what they ate', async () => {
    mocks.review.mockResolvedValue({});
    renderDialog(mealWith({}));

    expect(screen.getByText('Alex')).toBeInTheDocument();
    expect(screen.queryByText('Morgan')).not.toBeInTheDocument();

    await userEvent.setup().click(screen.getByRole('button', { name: 'Record meal' }));
    expect(mocks.review).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 'meal-1',
        revision: 4,
        body: expect.objectContaining({
          members: [{
            member_id: 'm1',
            result: 'changed',
            components: [{ component_id: 'c1', amount: { kind: 'measure', unit: 'g', value: 300 } }],
          }],
        }),
      }),
    );
  });

  it('records a member who did not eat', async () => {
    mocks.review.mockResolvedValue({});
    renderDialog(mealWith({}));
    const user = userEvent.setup();

    const field = screen.getByRole('spinbutton', { name: 'Chilli' });
    await user.clear(field);
    await user.type(field, '0');
    await user.click(screen.getByRole('button', { name: 'Record meal' }));

    expect(mocks.review).toHaveBeenCalledWith(
      expect.objectContaining({
        body: expect.objectContaining({
          members: [{ member_id: 'm1', result: 'not_eaten' }],
        }),
      }),
    );
  });

  it('shares out only what was actually cooked', async () => {
    renderDialog(cookedMeal(1));

    expect(screen.getByText('1 serving of Curry made')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'More for Alex' })).toBeDisabled();
    expect(screen.getByText('Did not eat')).toBeInTheDocument();
    expect(screen.getByText('Nothing left over')).toBeInTheDocument();
  });

  it('reports what is left when more was cooked than eaten', () => {
    renderDialog(cookedMeal(4));
    expect(screen.getByText('2 left over')).toBeInTheDocument();
    expect(screen.getByText('Put it away')).toBeInTheDocument();
  });

  it('moves the leftover portion where you put it', async () => {
    mocks.review.mockResolvedValue({});
    mocks.moveStock.mockResolvedValue({ id: 's1' });
    mocks.stockItems = [
      { id: 's1', prepared_batch_id: 'b1', revision: 2, storage_location: 'chilled' },
    ];
    renderDialog(cookedMeal(4));
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: 'Freezer' }));
    await user.click(screen.getByRole('button', { name: 'Record meal' }));

    expect(mocks.moveStock).toHaveBeenCalledWith({
      id: 's1',
      revision: 2,
      body: { storage_location: 'frozen' },
    });
  });

  it('leaves the leftover alone when it is already where you want it', async () => {
    mocks.review.mockResolvedValue({});
    mocks.stockItems = [
      { id: 's1', prepared_batch_id: 'b1', revision: 2, storage_location: 'chilled' },
    ];
    renderDialog(cookedMeal(4));
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: 'Fridge' }));
    await user.click(screen.getByRole('button', { name: 'Record meal' }));

    expect(mocks.moveStock).not.toHaveBeenCalled();
  });

  it('shows nothing to record once everyone is resolved', () => {
    renderDialog(mealWith({
      people: [
        { member_id: 'm1', display_name: 'Alex', status: 'eaten', can_record: true, allocations: [{ component_id: 'c1', allocated: { kind: 'measure', value: '300', unit: 'g' }, status: 'eaten' }] },
      ],
    }));
    expect(screen.getByText(/already been recorded/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Record meal' })).toBeDisabled();
  });
});
