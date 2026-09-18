import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MealPlanEntry } from '../../api/client';
import { MealOutcomeDialog } from './MealOutcomeDialog';

const mocks = vi.hoisted(() => ({ review: vi.fn(), place: vi.fn() }));

vi.mock('../../api/queries', () => ({
  useReviewMealOutcomes: () => ({ mutateAsync: mocks.review, isPending: false }),
  usePlacePortions: () => ({ mutateAsync: mocks.place, isPending: false }),
}));

const nutrition = { nutrition: {}, unknown_count: 0, partial_count: 0 };

const productComponent: MealPlanEntry['components'][number] = {
  id: 'c1',
  item_kind: 'product',
  product_id: 'p1',
  item_name: 'Chilli',
  amount: { kind: 'measure', value: 600, unit: 'g' },
  nutrition: {},
  quality: 'known',
  preparation: { prepared: { kind: 'measure', value: '600', unit: 'g' }, shortage: false },
  status: 'planned',
  subject_status: 'planned',
  position: 0,
  revision: 1,
  needs_cooking: false,
};

function mealWith(overrides: Partial<MealPlanEntry>): MealPlanEntry {
  return {
    id: 'meal-1',
    occasion_id: 'occasion-1',
    everyone: true,
    planned_on: '2026-08-25',
    planned_time: '18:30',
    slot: 'dinner',
    status: 'planned',
    components: [productComponent],
    participants: [
      { member_id: 'm1', display_name: 'Alex', status: 'planned', nutrition, allocations: [{ component_id: 'c1', allocated: { kind: 'measure', value: '300', unit: 'g' }, status: 'planned' }] },
      { member_id: 'm2', display_name: 'Morgan', status: 'eaten', nutrition, allocations: [{ component_id: 'c1', allocated: { kind: 'measure', value: '300', unit: 'g' }, status: 'eaten' }] },
    ],
    guest_groups: [],
    planned: nutrition,
    needs_attention: false,
    created_by: 'user-1',
    updated_by: 'user-1',
    created_at: '2026-08-24T10:00:00Z',
    updated_at: '2026-08-24T10:00:00Z',
    revision: 4,
    ...overrides,
  };
}

function cookedMeal(servingsProduced: number): MealPlanEntry {
  return mealWith({
    components: [{
      id: 'c1',
      item_kind: 'recipe',
      recipe_id: 'r1',
      item_name: 'Curry',
      amount: { kind: 'servings', value: 4 },
      nutrition: {},
      quality: 'known',
      preparation: { prepared: { kind: 'servings', value: '4' }, shortage: false },
      status: 'planned',
      subject_status: 'planned',
      position: 0,
      revision: 1,
      needs_cooking: true,
      cooked: { prepared_batch_id: 'b1', prepared_at: '2026-08-25T17:00:00Z', servings_produced: servingsProduced, revision: 2 },
    }],
    participants: [
      { member_id: 'm1', display_name: 'Alex', status: 'planned', nutrition, allocations: [{ component_id: 'c1', allocated: { kind: 'servings', value: '1' }, status: 'planned' }] },
      { member_id: 'm2', display_name: 'Morgan', status: 'planned', nutrition, allocations: [{ component_id: 'c1', allocated: { kind: 'servings', value: '1' }, status: 'planned' }] },
    ],
  });
}

function renderDialog(meal: MealPlanEntry) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  const onClose = vi.fn();
  render(
    <QueryClientProvider client={qc}>
      <MealOutcomeDialog meal={meal} canRecord canRecordGuests onClose={onClose} />
    </QueryClientProvider>,
  );
  return onClose;
}

describe('MealOutcomeDialog', () => {
  beforeEach(() => vi.clearAllMocks());

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

  it('puts the whole leftover in the freezer by default', async () => {
    mocks.review.mockResolvedValue({});
    mocks.place.mockResolvedValue({});
    renderDialog(cookedMeal(4));

    await userEvent.setup().click(screen.getByRole('button', { name: 'Record meal' }));

    expect(mocks.place).toHaveBeenCalledWith({
      id: 'b1',
      revision: 2,
      body: { placements: [{ storage_location: 'frozen', servings: 2 }] },
    });
  });

  it('splits the leftover between the fridge and the freezer', async () => {
    mocks.review.mockResolvedValue({});
    mocks.place.mockResolvedValue({});
    renderDialog(cookedMeal(4));
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: 'More in the fridge' }));
    await user.click(screen.getByRole('button', { name: 'More in the fridge' }));
    await user.click(screen.getByRole('button', { name: 'Record meal' }));

    expect(mocks.place).toHaveBeenCalledWith({
      id: 'b1',
      revision: 2,
      body: {
        placements: [
          { storage_location: 'chilled', servings: 1 },
          { storage_location: 'frozen', servings: 1 },
        ],
      },
    });
  });

  it('does not put anything away when it was all eaten', async () => {
    mocks.review.mockResolvedValue({});
    renderDialog(cookedMeal(2));

    await userEvent.setup().click(screen.getByRole('button', { name: 'Record meal' }));

    expect(mocks.place).not.toHaveBeenCalled();
  });

  it('shows nothing to record once everyone is resolved', () => {
    renderDialog(mealWith({
      participants: [
        { member_id: 'm1', display_name: 'Alex', status: 'eaten', nutrition, allocations: [{ component_id: 'c1', allocated: { kind: 'measure', value: '300', unit: 'g' }, status: 'eaten' }] },
      ],
    }));
    expect(screen.getByText(/already been recorded/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Record meal' })).toBeDisabled();
  });
});
