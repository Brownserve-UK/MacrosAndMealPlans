import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { MealEditorDialog } from './MealEditorDialog';

const attendance = vi.hoisted(() => ({ rows: [] as unknown[], isFetching: false }));

vi.mock('../../auth/AuthProvider', () => ({ useAuth: () => ({ principal: { member_id: 'me' } }) }));
vi.mock('../../api/queries', () => ({
  useCreateMealPlanEntry: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useUpdateMealPlanEntry: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useHouseholdSlotAttendance: () => ({ data: attendance.rows, isFetching: attendance.isFetching }),
  useMembers: () => ({
    data: {
      items: [
        { id: 'me', display_name: 'Me' },
        { id: 'morgan', display_name: 'Morgan Sample' },
      ],
    },
  }),
  useProducts: () => ({ data: { items: [] }, isLoading: false }),
  useRecipes: () => ({ data: { items: [] }, isLoading: false }),
}));

describe('MealEditorDialog household roster', () => {
  beforeEach(() => {
    attendance.rows = [];
    attendance.isFetching = false;
  });

  it('disables a member already eating in that slot and shows the reason', () => {
    attendance.rows = [
      { member_id: 'morgan', display_name: 'Morgan Sample', attendance: 'participating', claimed_time: '10:00' },
    ];
    render(
      <MealEditorDialog open mode="household" onClose={vi.fn()} date="2026-09-10" slot="breakfast" meal={null} />,
    );
    const label = screen.getByText('Morgan Sample').closest('label') as HTMLElement;
    expect(label.querySelector('input')).toBeDisabled();
    expect(screen.getByText('Already eating at 10:00')).toBeInTheDocument();
  });

  it('falls back to a generic reason when the clashing meal has no time', () => {
    attendance.rows = [
      { member_id: 'morgan', display_name: 'Morgan Sample', attendance: 'participating', claimed_time: null },
    ];
    render(
      <MealEditorDialog open mode="household" onClose={vi.fn()} date="2026-09-10" slot="breakfast" meal={null} />,
    );
    expect(screen.getByText('Already in another meal')).toBeInTheDocument();
  });

  it('locks the whole roster while attendance is still being fetched', () => {
    attendance.rows = [
      { member_id: 'morgan', display_name: 'Morgan Sample', attendance: 'available', claimed_time: null },
    ];
    attendance.isFetching = true;
    render(
      <MealEditorDialog open mode="household" onClose={vi.fn()} date="2026-09-10" slot="breakfast" meal={null} />,
    );
    expect(screen.getByText("Checking who's free…")).toBeInTheDocument();
    for (const name of ['Me', 'Morgan Sample']) {
      expect((screen.getByText(name).closest('label') as HTMLElement).querySelector('input')).toBeDisabled();
    }
  });
});
