import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { PlannerPage } from './PlannerPage';

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  remember: vi.fn(),
  principal: { member_id: 'me', permissions: ['household:write'] } as {
    member_id: string | null;
    permissions: string[];
  },
}));

vi.mock('@tanstack/react-router', () => ({ useNavigate: () => mocks.navigate }));
vi.mock('../../auth/AuthProvider', () => ({ useAuth: () => ({ principal: mocks.principal }) }));
vi.mock('./usePlannerLens', () => ({ usePlannerLens: () => ({ remember: mocks.remember }) }));
vi.mock('./MineLens', () => ({
  MineLens: ({ showLens, onLensChange }: { showLens: boolean; onLensChange: (lens: 'household') => void }) => (
    <div>
      <span>nutrition panel</span>
      <h2>Snacks</h2>
      {showLens ? <button onClick={() => onLensChange('household')}>Household</button> : null}
    </div>
  ),
}));
vi.mock('./HouseholdLens', () => ({ HouseholdLens: () => <div>Use it up</div> }));

describe('PlannerPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.principal = { member_id: 'me', permissions: ['household:write'] };
  });

  it('renders nutrition and Snacks for Mine', () => {
    render(<PlannerPage weekStart="2026-09-14" day="2026-09-15" requestedLens="mine" />);
    expect(screen.getByText('nutrition panel')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Snacks' })).toBeInTheDocument();
    expect(screen.queryByText('Use it up')).not.toBeInTheDocument();
  });

  it('renders household coordination without Mine content', () => {
    render(<PlannerPage weekStart="2026-09-14" day="2026-09-15" requestedLens="household" />);
    expect(screen.getByText('Use it up')).toBeInTheDocument();
    expect(screen.queryByText('nutrition panel')).not.toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Snacks' })).not.toBeInTheDocument();
  });

  it('navigates between lenses on the Planner route', async () => {
    render(<PlannerPage weekStart="2026-09-14" day="2026-09-15" requestedLens="mine" />);
    await userEvent.setup().click(screen.getByRole('button', { name: 'Household' }));
    expect(mocks.remember).toHaveBeenCalledWith('household');
    expect(mocks.navigate).toHaveBeenCalledWith({
      to: '/planner/$weekStart/$day',
      params: { weekStart: '2026-09-14', day: '2026-09-15' },
      search: { lens: 'household' },
      replace: true,
    });
  });

  it('hides Household and rewrites an unavailable lens to Mine', async () => {
    mocks.principal = { member_id: 'me', permissions: [] };
    render(<PlannerPage weekStart="2026-09-14" day="2026-09-15" requestedLens="household" />);
    expect(screen.getByText('nutrition panel')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Household' })).not.toBeInTheDocument();
    await waitFor(() => expect(mocks.navigate).toHaveBeenCalledWith({
      to: '/planner/$weekStart/$day',
      params: { weekStart: '2026-09-14', day: '2026-09-15' },
      search: { lens: 'mine' },
      replace: true,
    }));
  });

  it('keeps Household available without a linked member', () => {
    mocks.principal = { member_id: null, permissions: ['household:write'] };
    render(<PlannerPage weekStart="2026-09-14" day="2026-09-15" />);
    expect(screen.getByText('Use it up')).toBeInTheDocument();
  });
});
