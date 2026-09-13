import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { NutritionPlan, WeightRecord, WeightSummary } from '../../api/client';
import { GoalsPage } from './GoalsPage';

const mocks = vi.hoisted(() => ({ member: vi.fn(), plan: vi.fn(), summary: vi.fn(), records: vi.fn(), profile: vi.fn(), updateMember: vi.fn() }));
vi.mock('../../auth/AuthProvider', () => ({ useAuth: () => ({ principal: { member_id: 'member-1', username: 'joe', permissions: [] } }) }));
vi.mock('../../api/queries', () => ({ useMember: () => mocks.member(), useNutritionPlan: () => mocks.plan(), useWeightSummary: () => mocks.summary(), useWeightRecords: () => mocks.records(), useBodyProfile: () => mocks.profile(), useUpdateMember: () => ({ mutate: mocks.updateMember }), useDeleteWeighIn: () => ({ mutateAsync: vi.fn(), isPending: false }), useRecordWeighIn: () => ({ mutateAsync: vi.fn(), isPending: false }), useUpdateWeighIn: () => ({ mutateAsync: vi.fn(), isPending: false }), usePreviewCalorieTarget: () => ({ mutateAsync: vi.fn(), isPending: false }), useSetGuidedCalorieTarget: () => ({ mutateAsync: vi.fn(), isPending: false }), useSetManualCalorieTarget: () => ({ mutateAsync: vi.fn(), isPending: false }), useHouseholdSettings: () => ({ data: undefined }) }));

const summary = (overrides: Partial<WeightSummary> = {}) => ({ series: [], ...overrides }) as WeightSummary;
const record = (id: string, on: string, kg: number) => ({ id, member_id: 'member-1', weight_kg: kg, recorded_on: on, source: 'manual', revision: 1, created_at: `${on}T08:00:00Z`, updated_at: `${on}T08:00:00Z` }) as WeightRecord;
function renderPage() { const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } }); render(<QueryClientProvider client={client}><GoalsPage /></QueryClientProvider>); }

describe('GoalsPage', () => {
  beforeEach(() => { vi.clearAllMocks(); mocks.member.mockReturnValue({ data: { id: 'member-1', weight_display: 'kilograms', revision: 3 }, isLoading: false, isError: false }); mocks.plan.mockReturnValue({ data: { target: null, calculation: null } satisfies NutritionPlan, isLoading: false, isError: false }); mocks.summary.mockReturnValue({ data: summary(), isLoading: false, isError: false }); mocks.records.mockReturnValue({ data: [], isLoading: false, isError: false }); mocks.profile.mockReturnValue({ data: undefined }); });
  it('offers one primary first-time action and manual entry', () => { renderPage(); expect(screen.getByRole('button', { name: 'Get started' })).toBeInTheDocument(); expect(screen.getByRole('button', { name: 'Set my own numbers' })).toBeInTheDocument(); });
  it('shows calorie and macro targets in the hero', () => { mocks.plan.mockReturnValue({ data: { target: { energy_kcal: 2150, protein_g: 135, carbohydrate_g: 250, fat_g: 65 }, calculation: null } as NutritionPlan, isLoading: false, isError: false }); renderPage(); expect(screen.getByText('2,150 kcal')).toBeInTheDocument(); expect(screen.getByText('135 g')).toBeInTheDocument(); expect(screen.getByRole('button', { name: 'Review targets' })).toBeInTheDocument(); });
  it('opens the redesigned wizard at the goal question', async () => { const user = userEvent.setup(); renderPage(); await user.click(screen.getByRole('button', { name: 'Get started' })); expect(screen.getByRole('dialog')).toHaveTextContent('What is your goal?'); });
  it('caps recent weigh-ins and opens the complete list', async () => { const user = userEvent.setup(); const records = [record('4', '2026-09-12', 79), record('3', '2026-09-05', 80), record('2', '2026-08-29', 81), record('1', '2026-08-22', 82)]; mocks.records.mockReturnValue({ data: records, isLoading: false, isError: false }); mocks.summary.mockReturnValue({ data: summary({ latest: records[0], series: [...records].reverse().map((item) => ({ on: item.recorded_on, weight_kg: item.weight_kg })) }), isLoading: false, isError: false }); renderPage(); expect(screen.queryByText('22 August 2026')).not.toBeInTheDocument(); await user.click(screen.getByRole('button', { name: 'See all weigh-ins' })); expect(screen.getByText('22 August 2026')).toBeInTheDocument(); });
});
