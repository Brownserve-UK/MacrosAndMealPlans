import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MealItem, MealPlanWeek } from '../../api/client';
import { MealPlanPage } from './MealPlanPage';

const mocks = vi.hoisted(() => ({
  markEaten: vi.fn(),
  reopen: vi.fn(),
  navigate: vi.fn(),
}));

const WEEK_START = '2026-08-24';
const DAY = '2026-08-25';

const nutrition = { nutrition: {}, unknown_count: 0, partial_count: 0 };

const plannedItem: MealItem = {
  kind: 'planned',
  entry_id: 'entry-1',
  component_id: 'component-1',
  status: 'planned',
  product_id: 'product-1',
  product_name: 'Jumbo Oats',
  amount: { kind: 'measure', value: 80, unit: 'g' },
  nutrition: { energy_kcal: 302 },
  quality: 'known',
  needs_attention: false,
  revision: 3,
};

const eatenItem: MealItem = {
  kind: 'planned',
  entry_id: 'entry-2',
  component_id: 'component-2',
  linked_record_id: 'record-2',
  status: 'eaten',
  product_id: 'product-2',
  product_name: 'Whole Milk',
  amount: { kind: 'measure', value: 250, unit: 'ml' },
  nutrition: { energy_kcal: 124 },
  quality: 'known',
  needs_attention: false,
  revision: 5,
};

function week(): MealPlanWeek {
  const emptySlots = ['lunch', 'dinner', 'snacks'].map((slot) => ({
    slot: slot as 'lunch' | 'dinner' | 'snacks',
    items: [],
    nutrition,
  }));
  const days = Array.from({ length: 7 }, (_, index) => {
    const date = new Date('2026-08-24T00:00:00');
    date.setDate(date.getDate() + index);
    const iso = date.toISOString().slice(0, 10);
    const slots =
      iso === DAY
        ? [{ slot: 'breakfast' as const, items: [plannedItem, eatenItem], nutrition }, ...emptySlots]
        : [{ slot: 'breakfast' as const, items: [], nutrition }, ...emptySlots];
    return {
      date: iso,
      entries: [],
      slots,
      actual: nutrition,
      remaining_planned: nutrition,
      projected: nutrition,
    };
  });
  return {
    member_id: 'member-1',
    week_start: WEEK_START,
    week_end: '2026-08-30',
    days,
    actual: nutrition,
    remaining_planned: nutrition,
    projected: nutrition,
  };
}

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mocks.navigate,
}));

vi.mock('../../auth/AuthProvider', () => ({
  useAuth: () => ({ principal: { member_id: 'member-1' } }),
}));

vi.mock('../../api/queries', () => ({
  useMealPlanWeek: () => ({ data: week(), isLoading: false, isError: false, refetch: vi.fn() }),
  useMeta: () => ({ data: { nutrient_directions: {} } }),
  useMarkMealPlanEaten: () => ({ mutateAsync: mocks.markEaten }),
  useReopenMealPlanEntry: () => ({ mutateAsync: mocks.reopen }),
}));

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  render(
    <QueryClientProvider client={queryClient}>
      <MealPlanPage weekStart={WEEK_START} day={DAY} />
    </QueryClientProvider>,
  );
}

describe('MealPlanPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders planned and eaten food in the same slot', () => {
    renderPage();

    expect(screen.getByText('Jumbo Oats')).toBeInTheDocument();
    expect(screen.getByText('Whole Milk')).toBeInTheDocument();
  });

  it('ticking a planned row marks it eaten at the planned amount', async () => {
    mocks.markEaten.mockResolvedValue({});
    renderPage();
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: 'Mark Jumbo Oats eaten' }));

    expect(mocks.markEaten).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 'entry-1',
        revision: 3,
        body: expect.objectContaining({
          components: [{ component_id: 'component-1', amount: plannedItem.amount }],
        }),
      }),
    );
  });

  it('tapping an eaten row unticks it back to planned', async () => {
    mocks.reopen.mockResolvedValue({});
    renderPage();
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: 'Mark Whole Milk not eaten yet' }));

    expect(mocks.reopen).toHaveBeenCalledWith({ id: 'entry-2', revision: 5 });
  });
});
