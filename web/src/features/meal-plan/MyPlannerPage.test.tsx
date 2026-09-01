import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MealPlanEntry, MealPlanWeek } from '../../api/client';
import { MyPlannerPage } from './MyPlannerPage';

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  remove: vi.fn(),
  optOut: vi.fn(),
  rejoin: vi.fn(),
}));

function iso(date: Date) {
  return date.toISOString().slice(0, 10);
}
function startOfWeek(date: Date) {
  const copy = new Date(date);
  const weekday = (copy.getUTCDay() + 6) % 7;
  copy.setUTCDate(copy.getUTCDate() - weekday);
  return copy;
}

const TODAY = new Date();
const WEEK_START = iso(startOfWeek(TODAY));
const DAY = iso(TODAY);
const nutrition = { nutrition: {}, unknown_count: 0, partial_count: 0 };

function baseEntry(overrides: Partial<MealPlanEntry>): MealPlanEntry {
  return {
    id: 'entry',
    scope: 'member',
    member_id: 'me',
    subject_member_id: 'me',
    participants: [],
    guest_groups: [],
    opted_out: [],
    planned_on: DAY,
    planned_time: '18:30',
    slot: 'dinner',
    portioning: 'equal',
    status: 'planned',
    components: [{
      id: 'c1',
      item_kind: 'product',
      product_id: 'p1',
      item_name: 'Pasta bake',
      amount: { kind: 'measure', value: 300, unit: 'g' },
      position: 0,
      nutrition: {},
      quality: 'known',
      status: 'planned',
      subject_status: 'planned',
      preparation: { prepared: { kind: 'measure', value: '300', unit: 'g' }, shortage: false },
      revision: 1,
    }],
    planned: nutrition,
    needs_attention: false,
    created_by: 'u',
    updated_by: 'u',
    revision: 2,
    created_at: '2026-08-24T10:00:00Z',
    updated_at: '2026-08-24T10:00:00Z',
    ...overrides,
  } as MealPlanEntry;
}

let entries: MealPlanEntry[] = [];

function week(): MealPlanWeek {
  const days = Array.from({ length: 7 }, (_, index) => {
    const date = new Date(`${WEEK_START}T00:00:00Z`);
    date.setUTCDate(date.getUTCDate() + index);
    const dayIso = iso(date);
    return {
      date: dayIso,
      entries: dayIso === DAY ? entries : [],
      slots: ['breakfast', 'lunch', 'dinner', 'snacks'].map((slot) => ({ slot: slot as 'dinner', items: [], nutrition })),
      actual: nutrition,
      remaining_planned: nutrition,
      projected: nutrition,
    };
  });
  return {
    member_id: 'me',
    week_start: WEEK_START,
    week_end: days[days.length - 1]?.date ?? WEEK_START,
    days,
    actual: nutrition,
    remaining_planned: nutrition,
    projected: nutrition,
  };
}

function snackEntry(id: string, time: string | null, foodName: string): MealPlanEntry {
  const base = baseEntry({ id, slot: 'snacks', planned_time: time });
  return { ...base, components: [{ ...base.components[0], id: `${id}-c`, item_name: foodName }] } as MealPlanEntry;
}

vi.mock('@tanstack/react-router', () => ({ useNavigate: () => mocks.navigate }));
vi.mock('../../auth/AuthProvider', () => ({ useAuth: () => ({ principal: { member_id: 'me' } }) }));
vi.mock('../../api/queries', () => ({
  useMealPlanWeek: () => ({ data: week(), isLoading: false, isError: false, refetch: vi.fn() }),
  useMeta: () => ({ data: { nutrient_directions: {} } }),
  useDeleteMealPlanEntry: () => ({ mutateAsync: mocks.remove, isPending: false }),
  useOptOutOfMeal: () => ({ mutateAsync: mocks.optOut, isPending: false }),
  useRejoinMeal: () => ({ mutateAsync: mocks.rejoin, isPending: false }),
}));
vi.mock('./NutritionSummary', () => ({ DayWeekNutrition: () => <div>nutrition panel</div> }));
vi.mock('./MealEditorDialog', () => ({ MealEditorDialog: () => <div>Meal editor</div> }));

function dinnerSection() {
  return screen.getByRole('heading', { name: 'Dinner' }).closest('section') as HTMLElement;
}

describe('MyPlannerPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    entries = [];
  });

  it('offers Add food on a filled personal slot, never Add meal', () => {
    entries = [baseEntry({ id: 'mine', scope: 'member' })];
    render(<MyPlannerPage weekStart={WEEK_START} day={DAY} />);
    const dinner = within(dinnerSection());
    expect(dinner.getByText('Pasta bake')).toBeInTheDocument();
    expect(dinner.getByRole('button', { name: 'Add food' })).toBeInTheDocument();
    expect(dinner.getByRole('button', { name: 'Edit meal' })).toBeInTheDocument();
    expect(dinner.getByRole('button', { name: 'Delete' })).toBeInTheDocument();
    expect(dinner.queryByRole('button', { name: /Add meal/ })).not.toBeInTheDocument();
    expect(dinner.queryByRole('button', { name: /Opt out/ })).not.toBeInTheDocument();
  });

  it('shows an empty slot as a single Plan action', () => {
    render(<MyPlannerPage weekStart={WEEK_START} day={DAY} />);
    const lunch = within(screen.getByRole('heading', { name: 'Lunch' }).closest('section') as HTMLElement);
    expect(lunch.getByRole('button', { name: 'Plan lunch' })).toBeInTheDocument();
  });

  it('shows a household meal read-only with only an opt-out action', async () => {
    entries = [baseEntry({
      id: 'shared',
      scope: 'household',
      member_id: null,
      participants: [{
        member_id: 'me',
        display_name: 'Me',
        status: 'planned',
        allocations: [{ component_id: 'c1', allocated: { kind: 'measure', value: '150', unit: 'g' }, status: 'planned' }],
        nutrition,
      }],
    })];
    render(<MyPlannerPage weekStart={WEEK_START} day={DAY} />);
    const dinner = within(dinnerSection());
    expect(dinner.getByText('Household meal')).toBeInTheDocument();
    expect(dinner.queryByRole('button', { name: 'Edit meal' })).not.toBeInTheDocument();
    expect(dinner.queryByRole('button', { name: /Plan dinner/ })).not.toBeInTheDocument();
    await userEvent.setup().click(dinner.getByRole('button', { name: 'Opt out to plan your own' }));
    expect(mocks.optOut).toHaveBeenCalledWith({ id: 'shared', revision: 2 });
  });

  it('frees the slot with Join and Plan once a household meal is opted out of', async () => {
    entries = [baseEntry({
      id: 'shared',
      scope: 'household',
      member_id: null,
      opted_out: [{ member_id: 'me', created_by: 'u', created_at: '2026-08-24T10:00:00Z' }],
    })];
    render(<MyPlannerPage weekStart={WEEK_START} day={DAY} />);
    const dinner = within(dinnerSection());
    expect(dinner.getByText('Opted out')).toBeInTheDocument();
    expect(dinner.queryByText('Pasta bake')).not.toBeInTheDocument();
    expect(dinner.getByRole('button', { name: 'Plan dinner' })).toBeInTheDocument();
    await userEvent.setup().click(dinner.getByRole('button', { name: 'Join meal' }));
    expect(mocks.rejoin).toHaveBeenCalledWith({ id: 'shared', revision: 2 });
  });

  it('orders snack occurrences by time with the untimed one last', () => {
    entries = [
      snackEntry('s-late', '14:00', 'Afternoon crackers'),
      snackEntry('s-none', null, 'Anytime apple'),
      snackEntry('s-early', '10:00', 'Morning banana'),
    ];
    render(<MyPlannerPage weekStart={WEEK_START} day={DAY} />);
    const snacks = within(screen.getByRole('heading', { name: 'Snacks' }).closest('section') as HTMLElement);
    const foods = snacks.getAllByText(/banana|crackers|apple/i).map((node) => node.textContent);
    expect(foods).toEqual(['Morning banana', 'Afternoon crackers', 'Anytime apple']);
    expect(snacks.getByText('No set time')).toBeInTheDocument();
  });

  it('groups foods from one snack occurrence without an internal divider', () => {
    const snack = snackEntry('s-none', null, 'Greek yoghurt');
    const firstComponent = snack.components[0];
    if (!firstComponent) throw new Error('Expected a snack component');
    snack.components = [
      firstComponent,
      { ...firstComponent, id: 's-none-apple', item_name: 'Apples' },
    ];
    entries = [snack];

    render(<MyPlannerPage weekStart={WEEK_START} day={DAY} />);

    const occurrence = within(screen.getByRole('group', { name: 'Untimed snack' }));
    expect(occurrence.getByText('Greek yoghurt')).toBeInTheDocument();
    expect(occurrence.getByText('Apples')).toBeInTheDocument();
    expect(occurrence.queryByRole('separator')).not.toBeInTheDocument();
    expect(occurrence.getAllByRole('button', { name: 'Snack actions' })).toHaveLength(1);
  });

  it('offers only free main meals plus a snack from the page action', async () => {
    entries = [baseEntry({ id: 'mine', scope: 'member', slot: 'dinner' })];
    render(<MyPlannerPage weekStart={WEEK_START} day={DAY} />);

    await userEvent.setup().click(screen.getByRole('button', { name: 'Plan meal' }));

    expect(screen.getByRole('menuitem', { name: 'Breakfast' })).toBeInTheDocument();
    expect(screen.getByRole('menuitem', { name: 'Lunch' })).toBeInTheDocument();
    expect(screen.getByRole('menuitem', { name: 'Snack' })).toBeInTheDocument();
    expect(screen.queryByRole('menuitem', { name: 'Dinner' })).not.toBeInTheDocument();
  });
});
