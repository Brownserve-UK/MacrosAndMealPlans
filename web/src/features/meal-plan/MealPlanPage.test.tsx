import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError, type MealItem, type MealPlanEntry, type MealPlanWeek, type StockItem } from '../../api/client';
import { MealPlanPage } from './MealPlanPage';
import type { GroupView, OccasionView, PlannerWeek } from './planner/types';

const mocks = vi.hoisted(() => ({
  markComponentEaten: vi.fn(),
  reopen: vi.fn(),
  setAttendance: vi.fn(),
  addGroup: vi.fn(),
  navigate: vi.fn(),
  absent: false,
  stock: [] as StockItem[],
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

const assumedItem: MealItem = {
  ...plannedItem,
  component_id: 'component-assumed',
  status: 'assumed',
  item_name: 'Assumed Porridge',
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
  occasion_id: 'occasion-1',
  everyone: true,
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
      entries: iso === DAY ? [plannedEntry] : [],
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

const breakfastGroup: GroupView = {
  id: 'entry-1',
  name: 'Porridge',
  label: 'Porridge',
  ad_hoc: null,
  components: [],
  everyone: true,
  participants: [{ member_id: 'member-1', name: 'Sam', note: 'extra honey' }],
  guest_count: 0,
  guests: [],
  serves: 2,
  cook_minutes: 10,
  to_buy: 0,
  leftover_servings_available: null,
  revision: 7,
};

function plannerWeek(): PlannerWeek {
  const days = Array.from({ length: 7 }, (_, index) => {
    const date = new Date('2026-08-24T00:00:00');
    date.setDate(date.getDate() + index);
    const iso = date.toISOString().slice(0, 10);
    const breakfast: OccasionView | null =
      iso === DAY
        ? {
            id: 'occasion-1',
            planned_on: DAY,
            slot: 'breakfast',
            planned_time: '08:30',
            effective_time: '08:30',
            note: null,
            groups: [breakfastGroup],
            cooking: [],
            absent_member_ids: mocks.absent ? ['member-1'] : [],
            unaccounted_member_ids: [],
            revision: 2,
          }
        : null;
    return { date: iso, occasions: [breakfast, null, null, null] };
  });
  return {
    week_start: WEEK_START,
    usual_times: { breakfast: '08:00', lunch: '12:30', dinner: '18:00', snacks: null },
    members: [
      { id: 'member-1', name: 'Sam', initials: 'S' },
      { id: 'member-2', name: 'Alex', initials: 'A' },
    ],
    days,
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
  usePlannerWeek: () => ({ data: plannerWeek(), isLoading: false, isError: false, refetch: vi.fn() }),
  useHouseholdSettings: () => ({ data: undefined }),
  useMeta: () => ({ data: { nutrient_directions: {} } }),
  useStock: () => ({ data: { items: mocks.stock } }),
  useMarkMealPlanComponentEaten: () => ({ mutateAsync: mocks.markComponentEaten }),
  useReopenMealPlanComponent: () => ({ mutateAsync: mocks.reopen }),
  useSetAttendance: () => ({ mutateAsync: mocks.setAttendance, isPending: false }),
  useAddGroup: () => ({ mutateAsync: mocks.addGroup, isPending: false }),
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
    mocks.absent = false;
    mocks.stock = [];
    breakfastItems = [plannedItem, siblingItem, eatenItem];
  });

  it('is called My food', () => {
    renderPage();

    expect(screen.getByRole('heading', { name: 'My food' })).toBeInTheDocument();
  });

  it('says where planned food came from and how you have it', () => {
    renderPage();

    expect(screen.getByText('extra honey · 80 g · from Tuesday breakfast')).toBeInTheDocument();
  });

  it('lets you say you are eating elsewhere', async () => {
    mocks.setAttendance.mockResolvedValue({});
    renderPage();
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: 'Not eating this' }));
    await user.click(screen.getByText('Eating elsewhere'));

    expect(mocks.setAttendance).toHaveBeenCalledWith({
      occasionId: 'occasion-1',
      memberId: 'member-1',
      attendance: { kind: 'elsewhere' },
    });
  });

  it('only offers leftovers whose deadline covers the occasion', async () => {
    const portion: StockItem = {
      id: 'chilled',
      subject_kind: 'prepared_portion',
      prepared_recipe_id: 'curry',
      prepared_batch_name: 'Curry',
      storage_location: 'chilled',
      level: { mode: 'exact', quantity: { amount: 2, unit: 'serving' } },
      tracking_mode: 'exact',
      usability_deadline: { date: '2026-08-24' },
      revision: 1,
      created_at: '2026-08-22T09:00:00Z',
      updated_at: '2026-08-22T09:00:00Z',
    };
    mocks.stock = [
      portion,
      { ...portion, id: 'frozen', storage_location: 'frozen', usability_deadline: { date: DAY } },
    ];
    mocks.addGroup.mockResolvedValue({});
    renderPage();
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Not eating this' }));
    expect(screen.getByText('Curry, 2 servings (2 frozen)')).toBeInTheDocument();
    await user.click(screen.getByText('Leftovers'));
    expect(mocks.addGroup).toHaveBeenCalledWith({
      occasionId: 'occasion-1',
      body: {
        components: [{ dish_recipe_id: 'curry', amount: { kind: 'servings', value: 1 } }],
        everyone: false,
        participants: [{ member_id: 'member-1' }],
      },
    });
  });

  it('shows you as out and lets you change your mind', async () => {
    mocks.absent = true;
    mocks.setAttendance.mockResolvedValue({});
    breakfastItems = [];
    renderPage();
    const user = userEvent.setup();

    expect(screen.getByText('Eating elsewhere')).toBeInTheDocument();
    expect(screen.getByText("Tuesday breakfast · you're out on the planner")).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Change' }));

    expect(mocks.setAttendance).toHaveBeenCalledWith({
      occasionId: 'occasion-1',
      memberId: 'member-1',
      attendance: { kind: 'eating', group_id: 'entry-1' },
    });
  });

  it('renders planned and eaten food in the same slot', () => {
    renderPage();

    expect(screen.getByText('Jumbo Oats')).toBeInTheDocument();
    expect(screen.getByText('Whole Milk')).toBeInTheDocument();
  });

  it('ticks each component in a shared meal independently', async () => {
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

    await user.click(screen.getByRole('button', { name: 'Mark Sample Bananas eaten' }));

    expect(mocks.markComponentEaten).toHaveBeenLastCalledWith(
      expect.objectContaining({
        id: 'entry-1',
        componentId: 'component-banana',
        revision: 3,
        body: expect.objectContaining({ amount: siblingItem.amount }),
      }),
    );
    expect(mocks.markComponentEaten).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole('button', { name: 'Mark remaining eaten' })).not.toBeInTheDocument();
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

  it('shows the planned slot time as read-only text', () => {
    renderPage();

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

  it('distinguishes logged food from planned food without a bulk confirmation control', () => {
    breakfastItems = [plannedItem, loggedItem];
    renderPage();

    expect(screen.getByText('Planned')).toBeInTheDocument();
    expect(screen.getByText('Unplanned')).toBeInTheDocument();
    expect(screen.queryByText('Planned meal')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Mark remaining eaten' })).not.toBeInTheDocument();
  });
});

describe('MealPlanPage assumed meals', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.stock = [];
    breakfastItems = [assumedItem];
  });

  it('marks an assumed item so it reads differently from a plain planned one', () => {
    renderPage();

    expect(screen.getByText('Assumed Porridge')).toBeInTheDocument();
    expect(screen.getByText('Assumed')).toBeInTheDocument();
  });

  it('confirms an assumed item through the normal eaten path', async () => {
    mocks.markComponentEaten.mockResolvedValue({});
    renderPage();
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: 'Mark Assumed Porridge eaten' }));

    expect(mocks.markComponentEaten).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'entry-1', componentId: 'component-assumed' }),
    );
  });

  it('offers something else from the not eating this menu', async () => {
    renderPage();
    const user = userEvent.setup();

    expect(screen.queryByRole('button', { name: 'Ate something else' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Not eating this' }));

    expect(screen.getByText('Something else')).toBeInTheDocument();
    expect(screen.getByText('Log what you had')).toBeInTheDocument();
  });
});
