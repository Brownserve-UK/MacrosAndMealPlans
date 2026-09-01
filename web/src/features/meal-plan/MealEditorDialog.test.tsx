import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { MealEditorDialog } from './MealEditorDialog';

const attendance = vi.hoisted(() => ({ rows: [] as unknown[], isFetching: false }));

vi.mock('../../auth/AuthProvider', () => ({ useAuth: () => ({ principal: { member_id: 'me' } }) }));
vi.mock('../../api/queries', () => ({
  useCreateMealPlanEntry: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useUpdateMealPlanEntry: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useMealTimes: () => ({ data: { breakfast: '08:00', lunch: '12:30', dinner: '18:00' } }),
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

vi.mock('./FoodSearch', () => ({
  FoodSearch: ({ onPick }: { onPick: (choice: unknown) => void }) => (
    <button
      type="button"
      onClick={() => onPick({
        kind: 'product',
        product: {
          id: 'food',
          name: 'Test food',
          nutrition: { basis: { amount: 100, unit: 'g' } },
          package_quantity: null,
        },
      })}
    >
      Add test food
    </button>
  ),
}));

async function showPeople() {
  const user = userEvent.setup();
  await user.click(screen.getByRole('button', { name: 'Add test food' }));
  await user.click(screen.getByRole('button', { name: 'Continue' }));
}

describe('MealEditorDialog household roster', () => {
  beforeEach(() => {
    attendance.rows = [];
    attendance.isFetching = false;
  });

  it('disables a member already eating in that slot and shows the reason', async () => {
    attendance.rows = [
      { member_id: 'morgan', display_name: 'Morgan Sample', attendance: 'participating', claimed_time: '10:00' },
    ];
    render(
      <MealEditorDialog open mode="household" onClose={vi.fn()} date="2026-09-10" slot="breakfast" meal={null} />,
    );
    await showPeople();
    const label = screen.getByText('Morgan Sample').closest('label') as HTMLElement;
    expect(label.querySelector('input')).toBeDisabled();
    expect(screen.getByText('Already eating at 10:00')).toBeInTheDocument();
  });

  it('falls back to a generic reason when the clashing meal has no time', async () => {
    attendance.rows = [
      { member_id: 'morgan', display_name: 'Morgan Sample', attendance: 'participating', claimed_time: null },
    ];
    render(
      <MealEditorDialog open mode="household" onClose={vi.fn()} date="2026-09-10" slot="breakfast" meal={null} />,
    );
    await showPeople();
    expect(screen.getByText('Already in another meal')).toBeInTheDocument();
  });

  it('locks the whole roster while attendance is still being fetched', async () => {
    attendance.rows = [
      { member_id: 'morgan', display_name: 'Morgan Sample', attendance: 'available', claimed_time: null },
    ];
    attendance.isFetching = true;
    render(
      <MealEditorDialog open mode="household" onClose={vi.fn()} date="2026-09-10" slot="breakfast" meal={null} />,
    );
    await showPeople();
    expect(screen.getByText("Checking who's free…")).toBeInTheDocument();
    for (const name of ['Me', 'Morgan Sample']) {
      expect((screen.getByText(name).closest('label') as HTMLElement).querySelector('input')).toBeDisabled();
    }
  });

  it('keeps date and meal fixed while showing the configured time', async () => {
    render(
      <MealEditorDialog open mode="member" onClose={vi.fn()} date="2026-09-10" slot="breakfast" meal={null} />,
    );
    expect(screen.queryByLabelText('Date')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Meal')).not.toBeInTheDocument();
    await waitFor(() => expect(screen.getByLabelText('Time')).toHaveValue('08:00'));
    expect(screen.getByText('Thursday 10 September')).toBeInTheDocument();
  });
});
