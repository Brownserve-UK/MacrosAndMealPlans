import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { occasion, week } from './fixtures';
import { PlannerPage } from './PlannerPage';
import type { PlannerWeek } from './types';

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  create: vi.fn(),
  remove: vi.fn(),
  week: undefined as PlannerWeek | undefined,
  idle: () => ({ mutateAsync: vi.fn(), isPending: false }),
  noItems: () => ({ data: { items: [] }, isLoading: false }),
}));

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mocks.navigate,
  Link: ({ children }: { children: React.ReactNode }) => <a>{children}</a>,
}));
vi.mock('@mui/material/useMediaQuery', () => ({ default: () => true }));
vi.mock('../../../hooks/useHouseholdTimeZone', () => ({ useHouseholdTimeZone: () => 'UTC' }));
vi.mock('../../../api/queries', () => ({
  usePlannerWeek: () => ({ data: mocks.week, isLoading: false, isError: false, refetch: vi.fn() }),
  useCreateOccasion: () => ({ mutateAsync: mocks.create, isPending: false }),
  useDeleteOccasion: () => ({ mutateAsync: mocks.remove, isPending: false }),
  useMoveOccasion: mocks.idle,
  useCopyOccasion: mocks.idle,
  useCopyWeek: mocks.idle,
  useAddGroup: mocks.idle,
  useAddPlannerGuest: mocks.idle,
  useChangePlannerGuest: mocks.idle,
  useRemovePlannerGuest: mocks.idle,
  useSplitPlannerGuests: mocks.idle,
  useUpdateOccasion: mocks.idle,
  useUpdateGroup: mocks.idle,
  useSetAttendance: mocks.idle,
  useRecipes: mocks.noItems,
  useMealTemplates: mocks.noItems,
  useProducts: mocks.noItems,
  useStock: mocks.noItems,
}));

describe('PlannerPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.create.mockResolvedValue(occasion());
    mocks.remove.mockResolvedValue(undefined);
    mocks.week = week([occasion(), occasion({ id: 'occasion-2', planned_on: '2026-09-19', slot: 'dinner', groups: [] })]);
  });

  it('renders the week as a grid of cells with one quiet meta line', () => {
    render(<PlannerPage weekStart="2026-09-14" />);
    expect(screen.getByRole('grid', { name: 'Week plan' })).toBeInTheDocument();
    expect(screen.getByText('Spaghetti bolognese')).toBeInTheDocument();
    expect(screen.getByText('35 mins')).toBeInTheDocument();
    expect(screen.getByText('2 to buy')).toBeInTheDocument();
    expect(screen.getAllByRole('columnheader')).toHaveLength(7);
  });

  it('commits a name-only meal from the type-to-add picker', async () => {
    render(<PlannerPage weekStart="2026-09-14" />);
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /Plan lunch on 17 Sep/ }));
    const input = screen.getByRole('textbox', { name: 'What are you eating?' });
    await user.type(input, 'Pizza');
    expect(screen.getByText('Add “Pizza”')).toBeInTheDocument();
    await user.keyboard('{Enter}');
    expect(mocks.create).toHaveBeenCalledWith({
      planned_on: '2026-09-17',
      slot: 'lunch',
      group: { label: 'Pizza' },
    });
  });

  it('asks before deleting a meal from the card', async () => {
    render(<PlannerPage weekStart="2026-09-14" />);
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /Dinner on 17 Sep/ }));
    const card = screen.getByRole('dialog');
    expect(within(card).getByRole('heading', { name: 'Thursday dinner' })).toBeInTheDocument();
    await user.click(within(card).getByRole('button', { name: 'Delete' }));
    const confirm = screen.getByRole('dialog', { name: 'Delete this meal?' });
    expect(mocks.remove).not.toHaveBeenCalled();
    await user.click(within(confirm).getByRole('button', { name: 'Delete' }));
    expect(mocks.remove).toHaveBeenCalledWith({ id: 'occasion-1', revision: 3 });
  });
});
