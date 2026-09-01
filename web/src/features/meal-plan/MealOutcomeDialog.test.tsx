import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { PlannerMeal } from '../../api/client';
import { MealOutcomeDialog } from './MealOutcomeDialog';

const mocks = vi.hoisted(() => ({ review: vi.fn() }));

vi.mock('../../api/queries', () => ({
  useReviewMealOutcomes: () => ({ mutateAsync: mocks.review, isPending: false }),
}));

function mealWith(overrides: Partial<PlannerMeal>): PlannerMeal {
  return {
    id: 'meal-1',
    scope: 'household',
    planned_on: '2026-08-25',
    planned_time: '18:30',
    slot: 'dinner',
    portioning: 'equal',
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
  beforeEach(() => vi.clearAllMocks());

  it('only offers the still-pending participants and defaults them to as planned', async () => {
    mocks.review.mockResolvedValue({});
    renderDialog(mealWith({}));

    expect(screen.getByText('Alex')).toBeInTheDocument();
    expect(screen.queryByText('Morgan')).not.toBeInTheDocument();

    await userEvent.setup().click(screen.getByRole('button', { name: 'Confirm meal' }));
    expect(mocks.review).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 'meal-1',
        revision: 4,
        body: expect.objectContaining({
          members: [{ member_id: 'm1', result: 'as_planned' }],
        }),
      }),
    );
  });

  it('records a member who did not eat', async () => {
    mocks.review.mockResolvedValue({});
    renderDialog(mealWith({}));
    const user = userEvent.setup();

    await user.click(screen.getByRole('combobox', { name: 'Outcome' }));
    await user.click(screen.getByRole('option', { name: 'Did not eat' }));
    await user.click(screen.getByRole('button', { name: 'Confirm meal' }));

    expect(mocks.review).toHaveBeenCalledWith(
      expect.objectContaining({
        body: expect.objectContaining({
          members: [{ member_id: 'm1', result: 'not_eaten' }],
        }),
      }),
    );
  });

  it('shows nothing to record once everyone is resolved', () => {
    renderDialog(mealWith({
      people: [
        { member_id: 'm1', display_name: 'Alex', status: 'eaten', can_record: true, allocations: [{ component_id: 'c1', allocated: { kind: 'measure', value: '300', unit: 'g' }, status: 'eaten' }] },
      ],
    }));
    expect(screen.getByText(/no unresolved outcomes/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Confirm meal' })).toBeDisabled();
  });
});
