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
  useRecipeNutrition: () => ({ data: { nutrition: { energy_kcal: 345 } } }),
}));

vi.mock('./FoodSearch', () => ({
  FoodSearch: ({ onPick }: { onPick: (choice: unknown) => void }) => (
    <>
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
      <button
        type="button"
        onClick={() => onPick({ kind: 'recipe', recipe: { id: 'curry', name: 'Test curry' } })}
      >
        Add test recipe
      </button>
    </>
  ),
}));

async function addFood() {
  const user = userEvent.setup();
  await user.click(screen.getByRole('button', { name: 'Add test food' }));
  return user;
}

function personChip(name: string): HTMLElement {
  return screen.getByText(name).closest('.MuiChip-root') as HTMLElement;
}

describe('MealEditorDialog household roster', () => {
  beforeEach(() => {
    attendance.rows = [];
    attendance.isFetching = false;
  });

  it('cannot pick a member already eating in that slot, and says why on hover', async () => {
    attendance.rows = [
      { member_id: 'morgan', display_name: 'Morgan Sample', attendance: 'participating', claimed_time: '10:00' },
    ];
    render(
      <MealEditorDialog open mode="household" onClose={vi.fn()} date="2026-09-10" slot="breakfast" meal={null} />,
    );
    const user = await addFood();
    const chip = personChip('Morgan Sample');
    expect(chip).toHaveAttribute('aria-disabled', 'true');

    await user.click(chip);
    expect(screen.queryByText('Cooking 1 serving')).not.toBeInTheDocument();

    await user.hover(chip);
    expect(await screen.findByRole('tooltip')).toHaveTextContent('Already eating at 10:00');
  });

  it('falls back to a generic reason when the clashing meal has no time', async () => {
    attendance.rows = [
      { member_id: 'morgan', display_name: 'Morgan Sample', attendance: 'participating', claimed_time: null },
    ];
    render(
      <MealEditorDialog open mode="household" onClose={vi.fn()} date="2026-09-10" slot="breakfast" meal={null} />,
    );
    const user = await addFood();
    await user.hover(personChip('Morgan Sample'));
    expect(await screen.findByRole('tooltip')).toHaveTextContent('Already in another meal');
  });

  it('locks the whole roster while attendance is still being fetched', async () => {
    attendance.rows = [
      { member_id: 'morgan', display_name: 'Morgan Sample', attendance: 'available', claimed_time: null },
    ];
    attendance.isFetching = true;
    render(
      <MealEditorDialog open mode="household" onClose={vi.fn()} date="2026-09-10" slot="breakfast" meal={null} />,
    );
    await addFood();
    expect(screen.getByText("Checking who's free…")).toBeInTheDocument();
    for (const name of ['Me', 'Morgan Sample']) {
      expect(personChip(name)).toHaveClass('Mui-disabled');
    }
  });

  it('derives the servings from who is eating, and cook extra adds to them', async () => {
    attendance.rows = [
      { member_id: 'morgan', display_name: 'Morgan Sample', attendance: 'available', claimed_time: null },
    ];
    render(
      <MealEditorDialog open mode="household" onClose={vi.fn()} date="2026-09-10" slot="breakfast" meal={null} />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Add test recipe' }));

    expect(screen.getByRole('button', { name: 'Plan meal' })).toBeDisabled();

    await user.click(screen.getByText('Me'));
    await user.click(screen.getByText('Morgan Sample'));
    expect(screen.getByText('Cooking 2 servings')).toBeInTheDocument();
    expect(screen.getByText('One each, nothing left over.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Plan meal' })).toBeEnabled();

    await user.click(screen.getByRole('button', { name: 'Cook more extra' }));
    await user.click(screen.getByRole('button', { name: 'Cook more extra' }));
    expect(screen.getByText('Cooking 4 servings')).toBeInTheDocument();
    expect(screen.getByText('2 now, 2 kept for later.')).toBeInTheDocument();
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
