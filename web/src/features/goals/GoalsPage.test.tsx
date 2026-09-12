import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { NutritionPlan, WeightRecord, WeightSummary } from '../../api/client';
import { GoalsPage } from './GoalsPage';

const mocks = vi.hoisted(() => ({
  member: vi.fn(),
  plan: vi.fn(),
  summary: vi.fn(),
  records: vi.fn(),
  profile: vi.fn(),
  updateMember: vi.fn(),
}));

vi.mock('../../auth/AuthProvider', () => ({
  useAuth: () => ({ principal: { member_id: 'member-1', username: 'joe', permissions: [] } }),
}));

vi.mock('../../api/queries', () => ({
  useMember: () => mocks.member(),
  useNutritionPlan: () => mocks.plan(),
  useWeightSummary: () => mocks.summary(),
  useWeightRecords: () => mocks.records(),
  useBodyProfile: () => mocks.profile(),
  useUpdateMember: () => ({ mutate: mocks.updateMember, isPending: false }),
  useDeleteWeighIn: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useRecordWeighIn: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useUpdateWeighIn: () => ({ mutateAsync: vi.fn(), isPending: false }),
  usePreviewCalorieTarget: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useSetGuidedCalorieTarget: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useSetManualCalorieTarget: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useHouseholdSettings: () => ({ data: undefined }),
}));

function summary(overrides: Partial<WeightSummary> = {}): WeightSummary {
  return { series: [], ...overrides } as WeightSummary;
}

function record(): WeightRecord {
  return { id: 'weight-1', member_id: 'member-1', weight_kg: 80, recorded_on: '2026-09-12', source: 'manual', revision: 1, created_at: '2026-09-12T08:00:00Z', updated_at: '2026-09-12T08:00:00Z' } as WeightRecord;
}

function renderPage() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  render(<QueryClientProvider client={queryClient}><GoalsPage /></QueryClientProvider>);
}

describe('GoalsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.member.mockReturnValue({ data: { id: 'member-1', weight_display: 'kilograms', revision: 3 }, isLoading: false, isError: false });
    mocks.plan.mockReturnValue({ data: { target: null, calculation: null, calorie_direction: null } satisfies NutritionPlan, isLoading: false, isError: false });
    mocks.summary.mockReturnValue({ data: summary(), isLoading: false, isError: false });
    mocks.records.mockReturnValue({ data: [], isLoading: false, isError: false });
    mocks.profile.mockReturnValue({ data: undefined, isLoading: false, isError: false });
  });

  it('offers guided setup when no target is in force', () => {
    renderPage();
    expect(screen.getByText('We can work out a daily calorie target by asking a few questions.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Start' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Set my own target' })).toBeInTheDocument();
  });

  it('shows the target, its provenance, and the estimated goal date', () => {
    mocks.plan.mockReturnValue({ data: { target: { energy_kcal: 2150 }, calculation: { calculated_on: '2026-09-12', weight_kg: 82.4, maintenance_kcal: 2500, adjustment_kcal: -350, floor_kcal: 1500, eased: false }, calorie_direction: 'at_most' } as NutritionPlan, isLoading: false, isError: false });
    mocks.summary.mockReturnValue({ data: summary({ projection: { status: 'projected', on: '2027-03-12', remaining_kg: 6 } }), isLoading: false, isError: false });
    renderPage();
    expect(screen.getByText('2,150 kcal')).toBeInTheDocument();
    expect(screen.getByText(/Worked out 12 September 2026 from 82.4 kg/)).toBeInTheDocument();
    expect(screen.getByText('12 March 2027')).toBeInTheDocument();
  });

  it('opens the guided setup dialog', async () => {
    const user = userEvent.setup();
    renderPage();
    await user.click(screen.getByRole('button', { name: 'Start' }));
    expect(screen.getByRole('dialog')).toHaveTextContent('About you');
  });

  it('keeps the existing weight history on the goals surface', () => {
    mocks.records.mockReturnValue({ data: [record()], isLoading: false, isError: false });
    renderPage();
    expect(screen.getByText('80 kg')).toBeInTheDocument();
    expect(screen.getByText('12 September 2026')).toBeInTheDocument();
  });
});
