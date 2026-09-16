import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { PlannerShell } from './PlannerShell';

const mocks = vi.hoisted(() => ({ navigate: vi.fn() }));

vi.mock('@tanstack/react-router', () => ({ useNavigate: () => mocks.navigate }));
vi.mock('../../hooks/useHouseholdTimeZone', () => ({ useHouseholdTimeZone: () => 'UTC' }));

const days = Array.from({ length: 7 }, (_, index) => ({
  date: `2026-09-${String(14 + index).padStart(2, '0')}`,
  itemCount: 0,
}));

describe('PlannerShell', () => {
  beforeEach(() => vi.clearAllMocks());

  it('changes week without search state', async () => {
    render(
      <PlannerShell
        weekStart="2026-09-14"
        activeDate="2026-09-15"
        dayCounts={days}
        error={null}
        onDismissError={vi.fn()}
      >
        content
      </PlannerShell>,
    );
    await userEvent.setup().click(screen.getByRole('button', { name: 'Next week' }));
    expect(mocks.navigate).toHaveBeenCalledWith({
      to: '/planner/$weekStart/$day',
      params: { weekStart: '2026-09-21', day: '2026-09-21' },
    });
  });

  it('changes day without search state', async () => {
    render(
      <PlannerShell
        weekStart="2026-09-14"
        activeDate="2026-09-15"
        dayCounts={days}
        error={null}
        onDismissError={vi.fn()}
      >
        content
      </PlannerShell>,
    );
    await userEvent.setup().click(screen.getByRole('button', { name: /Wednesday 16 September/ }));
    expect(mocks.navigate).toHaveBeenCalledWith({
      to: '/planner/$weekStart/$day',
      params: { weekStart: '2026-09-14', day: '2026-09-16' },
    });
  });
});
