import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { PlannerMeal, PlannerWeek } from '../../api/client';
import { HouseholdPlannerPage } from './HouseholdPlannerPage';

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  remove: vi.fn(),
}));

const WEEK_START = '2026-08-24';
const DAY = '2026-08-25';

const meal: PlannerMeal = {
  id: 'meal-1',
  scope: 'household',
  planned_on: DAY,
  planned_time: '18:30',
  slot: 'dinner',
  portioning: 'equal',
  status: 'planned',
  foods: [{
    id: 'c1',
    item_kind: 'product',
    product_id: 'p1',
    item_name: 'Vegetable curry',
    amount: { kind: 'measure', value: 900, unit: 'g' },
    shortage: true,
  }],
  people: [
    { member_id: 'm1', display_name: 'Alex', status: 'planned', can_record: true, allocations: [{ component_id: 'c1', allocated: { kind: 'measure', value: '300', unit: 'g' }, status: 'planned' }] },
    { member_id: 'm2', display_name: 'Morgan', status: 'planned', can_record: true, allocations: [{ component_id: 'c1', allocated: { kind: 'measure', value: '300', unit: 'g' }, status: 'planned' }] },
  ],
  guest_groups: [{ id: 'g1', count: 1, status: 'planned', allocations: [{ component_id: 'c1', allocated: { kind: 'measure', value: '300', unit: 'g' }, status: 'planned' }] }],
  opted_out: [{ member_id: 'm3', created_by: 'u', created_at: '2026-08-24T10:00:00Z' }],
  can_opt_out: false,
  can_join: false,
  capabilities: { can_edit: true, can_delete: true, can_record_guests: true },
  revision: 3,
};

const week: PlannerWeek = { week_start: WEEK_START, week_end: '2026-08-30', meals: [meal] };

vi.mock('@tanstack/react-router', () => ({ useNavigate: () => mocks.navigate }));
vi.mock('../../api/queries', () => ({
  useHouseholdPlannerWeek: () => ({ data: week, isLoading: false, isError: false, refetch: vi.fn() }),
  useDeleteMealPlanEntry: () => ({ mutateAsync: mocks.remove, isPending: false }),
}));
vi.mock('./MealEditorDialog', () => ({ MealEditorDialog: () => <div>Meal editor</div> }));
vi.mock('./MealOutcomeDialog', () => ({ MealOutcomeDialog: () => <div>Outcome review</div> }));

describe('HouseholdPlannerPage', () => {
  beforeEach(() => vi.clearAllMocks());

  it('shows a household meal with attendance, an opted-out chip and a shortage warning', () => {
    render(<HouseholdPlannerPage weekStart={WEEK_START} day={DAY} />);
    expect(screen.getByText('Vegetable curry')).toBeInTheDocument();
    expect(screen.getByText('Alex')).toBeInTheDocument();
    expect(screen.getByText('Opted out')).toBeInTheDocument();
    expect(screen.getByText('1 guest')).toBeInTheDocument();
    expect(screen.getByText(/Not enough servings/)).toBeInTheDocument();
  });

  it('never shows the Snacks section', () => {
    render(<HouseholdPlannerPage weekStart={WEEK_START} day={DAY} />);
    expect(screen.queryByRole('heading', { name: 'Snacks' })).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Dinner' })).toBeInTheDocument();
  });

  it('opens bulk outcome review', async () => {
    render(<HouseholdPlannerPage weekStart={WEEK_START} day={DAY} />);
    await userEvent.setup().click(screen.getByRole('button', { name: 'Record meal' }));
    expect(screen.getByText('Outcome review')).toBeInTheDocument();
  });
});
