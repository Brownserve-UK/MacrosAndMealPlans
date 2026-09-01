import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError, type MealItem, type MealPlanEntry, type MealPlanWeek } from '../../api/client';
import { MealPlanPage } from './MealPlanPage';

const mocks = vi.hoisted(() => ({
  markEaten: vi.fn(),
  markComponentEaten: vi.fn(),
  reopen: vi.fn(),
  updateEntry: vi.fn(() => Promise.resolve({ member_id: 'member-1' })),
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
  item_kind: 'product',
  product_id: 'product-1',
  item_name: 'Jumbo Oats',
  at: '08:30',
  amount: { kind: 'measure', value: 80, unit: 'g' },
  nutrition: { energy_kcal: 302 },
  quality: 'known',
  needs_attention: false,
  revision: 3,
};

const siblingItem: MealItem = {
  ...plannedItem,
  component_id: 'component-banana',
  item_kind: 'product',
  product_id: 'product-banana',
  item_name: 'Sample Bananas',
  amount: { kind: 'measure', value: 1, unit: 'item' },
};

const eatenItem: MealItem = {
  kind: 'planned',
  entry_id: 'entry-2',
  component_id: 'component-2',
  linked_record_id: 'record-2',
  status: 'eaten',
  item_kind: 'product',
  product_id: 'product-2',
  item_name: 'Whole Milk',
  amount: { kind: 'measure', value: 250, unit: 'ml' },
  nutrition: { energy_kcal: 124 },
  quality: 'known',
  needs_attention: false,
  revision: 5,
};

const loggedItem: MealItem = {
  kind: 'logged',
  record_id: 'record-shake',
  linked_record_id: 'record-shake',
  status: 'eaten',
  item_kind: 'product',
  product_id: 'product-shake',
  item_name: 'Protein Shake',
  amount: { kind: 'measure', value: 300, unit: 'ml' },
  nutrition: { energy_kcal: 180 },
  quality: 'known',
  needs_attention: false,
  revision: 1,
};

const latteItem: MealItem = {
  ...plannedItem,
  component_id: 'component-latte',
  item_kind: 'product',
  product_id: 'product-latte',
  item_name: 'Latte',
  amount: { kind: 'measure', value: 250, unit: 'ml' },
};

const plannedEntry = {
  id: 'entry-1',
  scope: 'member' as const,
  member_id: 'member-1',
  subject_member_id: 'member-1',
  participants: [],
  guest_groups: [],
  planned_on: DAY,
  planned_time: '08:30',
  slot: 'breakfast',
  status: 'planned',
  components: [],
  planned: nutrition,
  needs_attention: false,
  created_by: 'user-1',
  updated_by: 'user-1',
  revision: 3,
  created_at: '2026-08-24T10:00:00Z',
  updated_at: '2026-08-24T10:00:00Z',
} satisfies MealPlanEntry;

const snackEntry = {
  ...plannedEntry,
  id: 'entry-snacks',
  planned_time: '20:30',
  slot: 'snacks',
} satisfies MealPlanEntry;

let breakfastItems: MealItem[] = [];

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
        ? [{ slot: 'breakfast' as const, items: breakfastItems, nutrition }, ...emptySlots]
        : [{ slot: 'breakfast' as const, items: [], nutrition }, ...emptySlots];
    return {
      date: iso,
      entries: iso === DAY ? [plannedEntry, snackEntry] : [],
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
  useMarkMealPlanComponentEaten: () => ({ mutateAsync: mocks.markComponentEaten }),
  useReopenMealPlanComponent: () => ({ mutateAsync: mocks.reopen }),
  useUpdateMealPlanEntry: () => ({ mutateAsync: mocks.updateEntry, isPending: false }),
  useMembers: () => ({ data: { items: [] } }),
  useSetMealPlanParticipants: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));

function renderPage(workspace: 'today' | 'planner' = 'today') {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  render(
    <QueryClientProvider client={queryClient}>
      <MealPlanPage weekStart={WEEK_START} day={DAY} workspace={workspace} />
    </QueryClientProvider>,
  );
}

describe('MealPlanPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    breakfastItems = [plannedItem, siblingItem, eatenItem];
  });

  it('renders planned and eaten food in the same slot', () => {
    renderPage();

    expect(screen.getByText('Jumbo Oats')).toBeInTheDocument();
    expect(screen.getByText('Whole Milk')).toBeInTheDocument();
  });

  it('ticking one component in a shared meal only resolves that component', async () => {
    mocks.markComponentEaten.mockResolvedValue({});
    renderPage();
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: 'Mark Jumbo Oats eaten' }));

    expect(mocks.markComponentEaten).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 'entry-1',
        componentId: 'component-1',
        revision: 3,
        body: expect.objectContaining({
          consumed_at: '2026-08-25T08:30:00.000Z',
          amount: plannedItem.amount,
        }),
      }),
    );
    expect(mocks.markComponentEaten).toHaveBeenCalledTimes(1);
    expect(screen.getByRole('button', { name: 'Mark Sample Bananas eaten' })).toBeInTheDocument();
  });

  it('leaves the consumption time unknown when planned food has no scheduled time', async () => {
    mocks.markComponentEaten.mockResolvedValue({});
    breakfastItems = [{ ...plannedItem, at: undefined }];
    renderPage();
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: 'Mark Jumbo Oats eaten' }));

    expect(mocks.markComponentEaten).toHaveBeenCalledWith(
      expect.objectContaining({
        body: expect.objectContaining({ consumed_at: null }),
      }),
    );
  });

  it('tapping an eaten row unticks it back to planned', async () => {
    mocks.reopen.mockResolvedValue({});
    renderPage();
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: 'Mark Whole Milk not eaten yet' }));

    expect(mocks.reopen).toHaveBeenCalledWith({
      id: 'entry-2',
      componentId: 'component-2',
      revision: 5,
    });
  });

  it('shows the reason when unticking fails and leaves the row eaten', async () => {
    mocks.reopen.mockRejectedValue(
      new ApiError(409, { detail: 'Someone else changed this meal.' } as never),
    );
    renderPage();
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: 'Mark Whole Milk not eaten yet' }));

    expect(await screen.findByText('Someone else changed this meal.')).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'Mark Whole Milk not eaten yet' }),
    ).toBeInTheDocument();
  });

  it('uses passive item statuses without progress counts in the planner', () => {
    renderPage('planner');

    expect(screen.queryByRole('button', { name: 'Mark Jumbo Oats eaten' })).not.toBeInTheDocument();
    expect(screen.queryByText(/of \d+ resolved/)).not.toBeInTheDocument();
    expect(screen.getAllByText('Planned')).toHaveLength(2);
    expect(screen.getByRole('button', { name: '08:30' })).toBeInTheDocument();
    expect(within(screen.getByRole('button', { name: 'Open Whole Milk' })).getByText('Eaten')).toBeInTheDocument();
  });

  it('edits the whole slot time from a single control in the planner', async () => {
    const user = userEvent.setup();
    renderPage('planner');

    await user.click(screen.getByRole('button', { name: '08:30' }));
    const field = screen.getByLabelText('Planned meal time');
    await user.clear(field);
    await user.type(field, '09:15');
    await user.click(screen.getByRole('button', { name: 'Save' }));

    expect(mocks.updateEntry).toHaveBeenCalledWith({
      id: 'entry-1',
      revision: 3,
      body: { planned_time: '09:15' },
    });
  });

  it('does not offer the slot time control in the food log', () => {
    renderPage('today');

    expect(screen.queryByRole('button', { name: '08:30' })).not.toBeInTheDocument();
    expect(screen.getByText('· 08:30')).toBeInTheDocument();
  });

  it('keeps planned and unplanned food in the order supplied by the food log', () => {
    breakfastItems = [plannedItem, siblingItem, loggedItem, latteItem];
    renderPage();

    expect(
      screen
        .getAllByRole('button', { name: /^Open / })
        .map((row) => row.getAttribute('aria-label')),
    ).toEqual([
      'Open Jumbo Oats',
      'Open Sample Bananas',
      'Open Protein Shake',
      'Open Latte',
    ]);
  });
});
