import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { MealTimesPage } from './MealTimesPage';

const mocks = vi.hoisted(() => ({
  updateMealTimes: vi.fn(() => Promise.resolve({})),
  permissions: ['household:write'] as string[],
}));

vi.mock('../../api/queries', () => ({
  useHouseholdSettings: () => ({
    isLoading: false,
    isError: false,
    data: {
      breakfast: '08:00',
      lunch: '12:30',
      dinner: '18:00',
      timezone: 'Etc/UTC',
      assume_eaten_when_time_passes: true,
      revision: 4,
      created_at: '2026-08-27T00:00:00Z',
      updated_at: '2026-08-27T00:00:00Z',
    },
    refetch: vi.fn(),
  }),
  useUpdateHouseholdSettings: () => ({ isPending: false, mutateAsync: mocks.updateMealTimes }),
}));

vi.mock('../../auth/AuthProvider', () => ({
  useAuth: () => ({ principal: { permissions: mocks.permissions } }),
}));

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => <span>{children}</span>,
}));

describe('MealTimesPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.permissions = ['household:write'];
  });

  it('shows the current default meal times', () => {
    render(<MealTimesPage />);
    expect(screen.getByLabelText('Breakfast')).toHaveValue('08:00');
    expect(screen.getByLabelText('Lunch')).toHaveValue('12:30');
    expect(screen.getByLabelText('Dinner')).toHaveValue('18:00');
  });

  it('saves only the changed time with the loaded revision', async () => {
    const user = userEvent.setup();
    render(<MealTimesPage />);

    const save = screen.getByRole('button', { name: 'Save' });
    expect(save).toBeDisabled();

    await user.clear(screen.getByLabelText('Lunch'));
    await user.type(screen.getByLabelText('Lunch'), '13:00');
    await user.click(save);

    expect(mocks.updateMealTimes).toHaveBeenCalledWith({
      revision: 4,
      body: { lunch: '13:00' },
    });
  });

  it('no longer offers a switch for adding everyone to new meals', () => {
    render(<MealTimesPage />);
    expect(screen.queryByLabelText('Add everyone to new household meals')).not.toBeInTheDocument();
    expect(screen.getByLabelText('Assume meals were eaten once their time passes')).toBeInTheDocument();
  });

  it('hides the save control without household:write', () => {
    mocks.permissions = [];
    render(<MealTimesPage />);
    expect(screen.queryByRole('button', { name: 'Save' })).not.toBeInTheDocument();
    expect(screen.getByLabelText('Breakfast')).toBeDisabled();
  });
});
