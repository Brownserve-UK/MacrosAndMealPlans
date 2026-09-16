import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { GuidedMealDialog } from './GuidedMealDialog';

const mocks = vi.hoisted(() => ({
  create: vi.fn(),
  manager: true,
  attendance: [] as unknown[],
}));

vi.mock('../../auth/AuthProvider', () => ({
  useAuth: () => ({
    principal: {
      member_id: 'me',
      permissions: mocks.manager ? ['household:write'] : [],
    },
  }),
}));

vi.mock('../../api/queries', () => ({
  useCreateMealPlanEntry: () => ({ mutateAsync: mocks.create, isPending: false }),
  useHouseholdSettings: () => ({
    data: { breakfast: '08:00', lunch: '12:30', dinner: '18:00', timezone: 'Etc/UTC' },
  }),
  useHouseholdSlotAttendance: () => ({ data: mocks.attendance, isFetching: false }),
  useMembers: () => ({
    data: {
      items: [
        { id: 'me', display_name: 'Sam Brown' },
        { id: 'morgan', display_name: 'Morgan Lee' },
      ],
    },
  }),
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

function renderDialog() {
  render(
    <GuidedMealDialog
      open
      onClose={vi.fn()}
      date="2026-09-16"
      slot="dinner"
    />,
  );
}

async function continueToFood(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'Continue' }));
  expect(await screen.findByText('What are you eating?')).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Add test food' }));
  await user.click(screen.getByRole('button', { name: 'Plan meal' }));
}

describe('GuidedMealDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.manager = true;
    mocks.attendance = [];
    mocks.create.mockResolvedValue(undefined);
  });

  it('takes a manager through attendance and creates a personal meal for just them', async () => {
    const user = userEvent.setup();
    renderDialog();

    expect(screen.getByText("Who's eating?")).toBeInTheDocument();
    expect(screen.getByLabelText('Progress')).toBeInTheDocument();
    expect(document.querySelector('[aria-label="Back"]')).not.toBeVisible();
    expect(screen.getByText('Dinner · 18:00')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Just me/ })).toHaveAttribute('aria-pressed', 'true');

    await continueToFood(user);

    expect(mocks.create).toHaveBeenCalledWith(expect.objectContaining({
      household: false,
      member_id: 'me',
      participants: undefined,
      guest_count: 0,
    }));
  });

  it('creates a personal meal for one other member', async () => {
    const user = userEvent.setup();
    renderDialog();
    await user.click(screen.getByRole('button', { name: /Just me/ }));
    await user.click(screen.getByRole('button', { name: /Morgan Lee/ }));

    await continueToFood(user);

    expect(mocks.create).toHaveBeenCalledWith(expect.objectContaining({
      household: false,
      member_id: 'morgan',
      participants: undefined,
      guest_count: 0,
    }));
  });

  it('creates a household meal for multiple members or guests', async () => {
    const user = userEvent.setup();
    renderDialog();
    await user.click(screen.getByRole('button', { name: /Morgan Lee/ }));
    await user.click(screen.getByRole('button', { name: 'Add guest' }));

    await continueToFood(user);

    expect(mocks.create).toHaveBeenCalledWith(expect.objectContaining({
      household: true,
      member_id: null,
      participants: [
        { member_id: 'me', allocations: [] },
        { member_id: 'morgan', allocations: [] },
      ],
      guest_count: 1,
    }));
  });

  it('shows blocked members as disabled choices with the existing reason', async () => {
    mocks.attendance = [{
      member_id: 'morgan',
      display_name: 'Morgan Lee',
      attendance: 'participating',
      claimed_time: '18:30',
    }];
    const user = userEvent.setup();
    renderDialog();
    const choice = screen.getByRole('button', { name: /Morgan Lee/ });
    expect(choice).toHaveAttribute('aria-disabled', 'true');
    await user.hover(choice.parentElement as HTMLElement);
    expect(await screen.findByRole('tooltip')).toHaveTextContent('Already eating at 18:30');
  });

  it('takes a plain member straight to the single food screen', async () => {
    mocks.manager = false;
    const user = userEvent.setup();
    renderDialog();

    expect(screen.getByText('What are you eating?')).toBeInTheDocument();
    expect(screen.queryByText("Who's eating?")).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Progress')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Back' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Close' })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Add test food' }));
    await user.click(screen.getByRole('button', { name: 'Plan meal' }));
    await waitFor(() => expect(mocks.create).toHaveBeenCalledWith(expect.objectContaining({
      household: false,
      member_id: 'me',
    })));
  });
});
