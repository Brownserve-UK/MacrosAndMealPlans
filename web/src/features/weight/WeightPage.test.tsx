import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { WeightGoal, WeightRecord, WeightSummary } from '../../api/client';
import { WeightPage } from './WeightPage';

const mocks = vi.hoisted(() => ({
  summary: vi.fn(),
  records: vi.fn(),
  member: vi.fn(),
  updateMember: vi.fn(),
}));

vi.mock('../../auth/AuthProvider', () => ({
  useAuth: () => ({ principal: { member_id: 'member-1', username: 'joe', permissions: [] } }),
}));

vi.mock('../../api/queries', () => ({
  useWeightSummary: () => mocks.summary(),
  useWeightRecords: () => mocks.records(),
  useHouseholdSettings: () => ({ data: undefined }),
  useMember: () => mocks.member(),
  useUpdateMember: () => ({ mutate: mocks.updateMember, isPending: false }),
  useDeleteWeighIn: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useRecordWeighIn: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useUpdateWeighIn: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useSetWeightGoal: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useUpdateWeightGoal: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useClearWeightGoal: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));

function record(id: string, on: string, kg: number): WeightRecord {
  return {
    id,
    member_id: 'member-1',
    weight_kg: kg,
    recorded_on: on,
    source: 'manual',
    revision: 1,
    created_at: '2026-09-01T08:00:00Z',
    updated_at: '2026-09-01T08:00:00Z',
  } as WeightRecord;
}

function goal(): WeightGoal {
  return {
    id: 'goal-1',
    member_id: 'member-1',
    objective: 'lose',
    starting_weight_kg: 82,
    target_weight_kg: 76,
    planned_rate_kg_per_week: 0.5,
    started_on: '2026-08-01',
    revision: 1,
    created_at: '2026-08-01T08:00:00Z',
    updated_at: '2026-08-01T08:00:00Z',
  } as WeightGoal;
}

function summary(overrides: Partial<WeightSummary> = {}): WeightSummary {
  return { series: [], ...overrides } as WeightSummary;
}

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  render(
    <QueryClientProvider client={queryClient}>
      <WeightPage />
    </QueryClientProvider>,
  );
}

describe('WeightPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.member.mockReturnValue({
      data: { id: 'member-1', weight_display: 'kilograms', revision: 3 },
      isLoading: false,
      isError: false,
    });
    mocks.records.mockReturnValue({ data: [], isLoading: false, isError: false });
    mocks.summary.mockReturnValue({ data: summary(), isLoading: false, isError: false });
  });

  it('invites a first weigh-in when there is nothing at all', () => {
    renderPage();

    expect(screen.getByText('No weight recorded')).toBeInTheDocument();
    expect(screen.getByText('Nothing recorded yet')).toBeInTheDocument();
  });

  it('shows the latest weight, the change and the projected date', () => {
    mocks.summary.mockReturnValue({
      data: summary({
        latest: record('w1', '2026-09-03', 80),
        goal: goal(),
        projection: { status: 'projected', on: '2026-10-01', remaining_kg: 4 },
        change_since_start_kg: -2,
        series: [
          { on: '2026-08-01', weight_kg: 82 },
          { on: '2026-09-03', weight_kg: 80 },
        ],
      }),
      isLoading: false,
      isError: false,
    });
    renderPage();

    expect(screen.getByText('80 kg')).toBeInTheDocument();
    expect(screen.getByText('76 kg')).toBeInTheDocument();
    expect(screen.getByText('−2 kg')).toBeInTheDocument();
    expect(screen.getByText('1 October 2026')).toBeInTheDocument();
    expect(screen.getByText('0.5 kg a week')).toBeInTheDocument();
  });

  it('says the goal is reached rather than showing a date', () => {
    mocks.summary.mockReturnValue({
      data: summary({
        latest: record('w1', '2026-09-03', 76),
        goal: goal(),
        projection: { status: 'reached' },
        change_since_start_kg: -6,
      }),
      isLoading: false,
      isError: false,
    });
    renderPage();

    expect(screen.getByText('Reached')).toBeInTheDocument();
  });

  it('has no end date for a goal it cannot project', () => {
    mocks.summary.mockReturnValue({
      data: summary({
        latest: record('w1', '2026-09-03', 80),
        goal: { ...goal(), objective: 'maintain', target_weight_kg: null, planned_rate_kg_per_week: null },
        projection: { status: 'steady' },
        change_since_start_kg: -2,
      }),
      isLoading: false,
      isError: false,
    });
    renderPage();

    expect(screen.getByText('No end date')).toBeInTheDocument();
  });

  it('renders every weigh-in in the history, including two on one day', () => {
    mocks.records.mockReturnValue({
      data: [
        record('w2', '2026-09-03', 80.9),
        record('w1', '2026-09-03', 80.2),
        record('w0', '2026-09-01', 81),
      ],
      isLoading: false,
      isError: false,
    });
    renderPage();

    expect(screen.getByText('80.9 kg')).toBeInTheDocument();
    expect(screen.getByText('80.2 kg')).toBeInTheDocument();
    expect(screen.getByText('81 kg')).toBeInTheDocument();
  });

  it('shows the member their weights in the units they picked', () => {
    mocks.member.mockReturnValue({
      data: { id: 'member-1', weight_display: 'stones_pounds', revision: 3 },
      isLoading: false,
      isError: false,
    });
    mocks.records.mockReturnValue({
      data: [record('w1', '2026-09-03', 72.575)],
      isLoading: false,
      isError: false,
    });
    renderPage();

    expect(screen.getByText('11 st 6 lb')).toBeInTheDocument();
  });

  it('saves the display preference on the member when it is switched', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByRole('button', { name: 'st + lb' }));

    expect(mocks.updateMember).toHaveBeenCalledWith({
      id: 'member-1',
      revision: 3,
      body: { weight_display: 'stones_pounds' },
    });
  });

  it('opens the weigh-in dialog', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByRole('button', { name: 'Add weigh-in' }));

    expect(screen.getByRole('dialog')).toHaveTextContent('Add weigh-in');
  });

  it('opens the goal dialog', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByRole('button', { name: 'Set a goal' }));

    expect(screen.getByRole('dialog')).toHaveTextContent('Set a goal');
  });
});
