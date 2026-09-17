import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { group, occasion, week } from './fixtures';
import { OccasionCard } from './OccasionCard';

const mocks = vi.hoisted(() => ({
  setAttendance: vi.fn(),
  updateGroup: vi.fn(),
  idle: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));
vi.mock('../../../api/queries', () => ({
  useUpdateOccasion: mocks.idle,
  useAddGroup: mocks.idle,
  useUpdateGroup: () => ({ mutateAsync: mocks.updateGroup, isPending: false }),
  useSetAttendance: () => ({ mutateAsync: mocks.setAttendance, isPending: false }),
  useStock: () => ({ data: { items: [] }, isLoading: false }),
}));

const curry = occasion({
  planned_on: '2026-09-15',
  effective_time: '17:30',
  groups: [
    group({
      id: 'curry',
      name: 'Chicken curry',
      everyone: false,
      participants: [
        { member_id: 'steve', name: 'Steve', note: null },
        { member_id: 'emily', name: 'Emily', note: 'mild' },
        { member_id: 'jack', name: 'Jack', note: null },
      ],
      serves: 3,
      effective_cooking_servings: 5,
      cooking_servings: 5,
      to_buy: 0,
    }),
  ],
  unaccounted_member_ids: ['sarah'],
});

describe('OccasionCard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.setAttendance.mockResolvedValue(curry);
    mocks.updateGroup.mockResolvedValue(curry.groups[0]);
  });

  it('shows the needs-a-meal slab for an unticked member and can mark them out', async () => {
    render(
      <OccasionCard occasion={curry} week={week([curry])} sheet={false} onClose={vi.fn()} onMove={vi.fn()} onCopy={vi.fn()} onDelete={vi.fn()} />,
    );
    expect(screen.getByRole('heading', { name: 'Tuesday dinner' })).toBeInTheDocument();
    expect(screen.getByText('Sarah needs a meal')).toBeInTheDocument();
    expect(screen.getByText('· mild')).toBeInTheDocument();
    expect(screen.getByText('(+2 for leftovers)', { exact: false })).toBeInTheDocument();
    await userEvent.setup().click(screen.getByRole('button', { name: /Eating elsewhere/ }));
    expect(mocks.setAttendance).toHaveBeenCalledWith({
      occasionId: 'occasion-1',
      memberId: 'sarah',
      attendance: { kind: 'elsewhere' },
    });
  });

  it('unticks a member from an everyone group by writing the explicit list', async () => {
    const everyone = occasion({ groups: [group({ id: 'bol', revision: 7 })] });
    render(
      <OccasionCard occasion={everyone} week={week([everyone])} sheet={false} onClose={vi.fn()} onMove={vi.fn()} onCopy={vi.fn()} onDelete={vi.fn()} />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Everyone' }));
    await user.click(screen.getByRole('button', { name: 'Sarah' }));
    await user.click(screen.getByRole('menuitem', { name: 'Not eating this' }));
    expect(mocks.updateGroup).toHaveBeenCalledWith({
      id: 'bol',
      body: {
        everyone: false,
        participants: [
          { member_id: 'steve', note: null },
          { member_id: 'emily', note: null },
          { member_id: 'jack', note: null },
        ],
        revision: 7,
      },
    });
  });
});
