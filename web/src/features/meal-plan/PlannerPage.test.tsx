import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { PlannerWeek } from '../../api/client';
import { PlannerPage } from './PlannerPage';

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  remove: vi.fn(),
}));

const week: PlannerWeek = {
  week_start: '2026-08-24',
  week_end: '2026-08-30',
  meals: [{
    id: 'meal-1',
    scope: 'household',
    planned_on: '2026-08-25',
    planned_time: '18:30',
    slot: 'dinner',
    status: 'planned',
    foods: [{
      id: 'food-1',
      item_kind: 'product',
      product_id: 'product-1',
      item_name: 'Vegetable curry',
      amount: { kind: 'measure', value: 900, unit: 'g' },
      shortage: false,
    }],
    people: [
      { member_id: 'member-1', display_name: 'Alex', status: 'planned', can_record: true, allocations: [{ component_id: 'food-1', allocated: { kind: 'measure', value: '300', unit: 'g' }, status: 'planned' }] },
      { member_id: 'member-2', display_name: 'Morgan', status: 'planned', can_record: true, allocations: [{ component_id: 'food-1', allocated: { kind: 'measure', value: '300', unit: 'g' }, status: 'planned' }] },
    ],
    guest_groups: [{ id: 'guests-1', count: 1, status: 'planned', allocations: [{ component_id: 'food-1', allocated: { kind: 'measure', value: '300', unit: 'g' }, status: 'planned' }] }],
    capabilities: { can_edit: true, can_delete: true, can_record_guests: true },
    revision: 1,
  }],
};

week.meals.push(
  {
    id: 'snack-afternoon',
    scope: 'member',
    member_id: 'member-1',
    owner_name: 'Alex',
    planned_on: '2026-08-25',
    planned_time: '14:00',
    slot: 'snacks',
    status: 'planned',
    foods: [{ id: 'food-chocolate', item_kind: 'product', product_id: 'product-chocolate', item_name: 'Chocolate bar', amount: { kind: 'measure', value: 1, unit: 'item' }, shortage: false }],
    people: [{ member_id: 'member-1', display_name: 'Alex', status: 'planned', can_record: true, allocations: [{ component_id: 'food-chocolate', allocated: { kind: 'measure', value: '1', unit: 'item' }, status: 'planned' }] }],
    guest_groups: [],
    capabilities: { can_edit: true, can_delete: true, can_record_guests: false },
    revision: 1,
  },
  {
    id: 'snack-untimed',
    scope: 'member',
    member_id: 'member-1',
    owner_name: 'Alex',
    planned_on: '2026-08-25',
    planned_time: null,
    slot: 'snacks',
    status: 'planned',
    foods: [{ id: 'food-yoghurt', item_kind: 'product', product_id: 'product-yoghurt', item_name: 'Greek yoghurt', amount: { kind: 'servings', value: 1 }, shortage: false }],
    people: [{ member_id: 'member-1', display_name: 'Alex', status: 'planned', can_record: true, allocations: [{ component_id: 'food-yoghurt', allocated: { kind: 'servings', value: '1' }, status: 'planned' }] }],
    guest_groups: [],
    capabilities: { can_edit: true, can_delete: true, can_record_guests: false },
    revision: 1,
  },
  {
    id: 'snack-morning',
    scope: 'member',
    member_id: 'member-1',
    owner_name: 'Alex',
    planned_on: '2026-08-25',
    planned_time: '10:00',
    slot: 'snacks',
    status: 'planned',
    foods: [{ id: 'food-banana', item_kind: 'product', product_id: 'product-banana', item_name: 'Banana', amount: { kind: 'measure', value: 1, unit: 'item' }, shortage: false }],
    people: [{ member_id: 'member-1', display_name: 'Alex', status: 'planned', can_record: true, allocations: [{ component_id: 'food-banana', allocated: { kind: 'measure', value: '1', unit: 'item' }, status: 'planned' }] }],
    guest_groups: [],
    capabilities: { can_edit: true, can_delete: true, can_record_guests: false },
    revision: 1,
  },
);

vi.mock('@tanstack/react-router', () => ({ useNavigate: () => mocks.navigate }));
vi.mock('../../auth/AuthProvider', () => ({
  useAuth: () => ({ principal: { member_id: 'member-1' } }),
}));
vi.mock('../../api/queries', () => ({
  usePlannerWeek: () => ({ data: week, isLoading: false, isError: false, refetch: vi.fn() }),
  useDeleteMealPlanEntry: () => ({ mutateAsync: mocks.remove, isPending: false }),
}));
vi.mock('./MealEditorDialog', () => ({ MealEditorDialog: () => <div>Meal editor</div> }));
vi.mock('./MealOutcomeDialog', () => ({ MealOutcomeDialog: () => <div>Outcome review</div> }));

describe('PlannerPage', () => {
  beforeEach(() => vi.clearAllMocks());

  it('shows one meal card with totals and a plain-language attendance summary', () => {
    render(<PlannerPage weekStart="2026-08-24" day="2026-08-25" />);

    expect(screen.getByText('Vegetable curry')).toBeInTheDocument();
    expect(screen.getByText('900 g')).toBeInTheDocument();
    expect(screen.getByText('Alex, Morgan and 1 guest')).toBeInTheDocument();
    expect(screen.queryByText('Eating')).not.toBeInTheDocument();
    expect(screen.queryByText(/left over/i)).not.toBeInTheDocument();
  });

  it('opens one meal-level outcome review', async () => {
    render(<PlannerPage weekStart="2026-08-24" day="2026-08-25" />);
    await userEvent.setup().click(screen.getAllByRole('button', { name: 'Mark as eaten' })[0]!);
    expect(screen.getByText('Outcome review')).toBeInTheDocument();
  });

  it('uses the established week navigator with day activity counts', () => {
    render(<PlannerPage weekStart="2026-08-24" day="2026-08-25" />);

    expect(screen.getByRole('button', { name: 'Previous week' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Next week' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Tuesday 25 August, 4 items' })).toHaveAttribute('aria-pressed', 'true');
  });

  it('shows timed and untimed Snacks in one chronological list', () => {
    render(<PlannerPage weekStart="2026-08-24" day="2026-08-25" />);

    const banana = screen.getByText('Banana');
    const chocolate = screen.getByText('Chocolate bar');
    const yoghurt = screen.getByText('Greek yoghurt');
    expect(screen.getByText('10:00 · 1 item')).toBeInTheDocument();
    expect(screen.getByText('14:00 · 1 item')).toBeInTheDocument();
    expect(banana.compareDocumentPosition(chocolate) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(chocolate.compareDocumentPosition(yoghurt) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });
});
