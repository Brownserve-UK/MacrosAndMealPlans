import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MealPlanEntry } from '../../api/client';
import { NeedsReviewPage } from './NeedsReviewPage';

const mocks = vi.hoisted(() => ({
  markEaten: vi.fn(),
  markNotEaten: vi.fn(),
  needsReview: vi.fn(),
}));

const nutrition = { nutrition: {}, unknown_count: 0, partial_count: 0 };

function entry(id: string, plannedOn: string, name: string): MealPlanEntry {
  return {
    id,
    scope: 'member',
    member_id: 'member-1',
    subject_member_id: 'member-1',
    participants: [],
    guest_groups: [],
    opted_out: [],
    planned_on: plannedOn,
    planned_time: '08:30',
    slot: 'breakfast',
    portioning: 'equal',
    status: 'assumed',
    components: [
      {
        id: `${id}-component`,
        item_kind: 'product',
        product_id: 'product-1',
        item_name: name,
        amount: { kind: 'measure', value: 80, unit: 'g' },
        nutrition: {},
        quality: 'known',
        preparation: { prepared: { kind: 'measure', value: '80', unit: 'g' }, shortage: false },
        status: 'assumed',
        subject_status: 'assumed',
        position: 0,
        revision: 1,
      },
    ],
    planned: nutrition,
    needs_attention: false,
    created_by: 'user-1',
    updated_by: 'user-1',
    revision: 3,
    created_at: '2026-08-24T10:00:00Z',
    updated_at: '2026-08-24T10:00:00Z',
  } as MealPlanEntry;
}

vi.mock('../../auth/AuthProvider', () => ({
  useAuth: () => ({ principal: { member_id: 'member-1', permissions: [] } }),
}));

vi.mock('../../api/queries', () => ({
  useNeedsReview: () => mocks.needsReview(),
  useMarkMealPlanEaten: () => ({ mutateAsync: mocks.markEaten, isPending: false }),
  useMarkMealPlanNotEaten: () => ({ mutateAsync: mocks.markNotEaten, isPending: false }),
  useSetProductMapping: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useReviewMealOutcomes: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useProducts: () => ({ data: { items: [] }, isLoading: false }),
  useRecipes: () => ({ data: { items: [] }, isLoading: false }),
}));

function firstButton(name: string) {
  const [button] = screen.getAllByRole('button', { name });
  if (!button) throw new Error(`no button named ${name}`);
  return button;
}

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  render(
    <QueryClientProvider client={queryClient}>
      <NeedsReviewPage />
    </QueryClientProvider>,
  );
}

describe('NeedsReviewPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.needsReview.mockReturnValue({
      data: {
        personal_meals: [
          entry('entry-old', '2026-08-20', 'Older Porridge'),
          entry('entry-new', '2026-08-22', 'Newer Porridge'),
        ],
        household_meals: [],
        ingredient_mappings: [],
      },
      isLoading: false,
      isError: false,
      refetch: vi.fn(),
    });
  });

  it('lists unresolved assumptions in the order the API returned them', () => {
    renderPage();

    const names = screen.getAllByText(/Porridge/).map((node) => node.textContent);
    expect(names).toEqual(['Older Porridge', 'Newer Porridge']);
  });

  it('confirms a meal with its planned components', async () => {
    mocks.markEaten.mockResolvedValue({});
    renderPage();
    const user = userEvent.setup();

    await user.click(firstButton('Ate it'));

    expect(mocks.markEaten).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 'entry-old',
        revision: 3,
        body: expect.objectContaining({
          consumed_on: '2026-08-20',
          components: [
            { component_id: 'entry-old-component', amount: { kind: 'measure', value: 80, unit: 'g' } },
          ],
        }),
      }),
    );
  });

  it('rejects a meal without recording anything eaten', async () => {
    mocks.markNotEaten.mockResolvedValue({});
    renderPage();
    const user = userEvent.setup();

    await user.click(firstButton('Not eaten'));

    expect(mocks.markNotEaten).toHaveBeenCalledWith({ id: 'entry-old', revision: 3 });
  });

  it('says so when there is nothing to review', () => {
    mocks.needsReview.mockReturnValue({
      data: { personal_meals: [], household_meals: [], ingredient_mappings: [] },
      isLoading: false,
      isError: false,
      refetch: vi.fn(),
    });
    renderPage();

    expect(screen.getByText('No meals need review.')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'My meals (0)' })).toBeInTheDocument();
  });

  it('hides the household section without household meals to act on', () => {
    renderPage();

    expect(screen.queryByRole('tab', { name: /Household meals/ })).not.toBeInTheDocument();
  });
});
