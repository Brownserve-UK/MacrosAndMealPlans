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
  useRecipes: () => ({ data: { items: [] }, isLoading: false }),
  useMealTemplates: () => ({ data: { items: [] }, isLoading: false }),
  useProducts: () => ({ data: { items: [] }, isLoading: false }),
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

  it('shows Sarah as needing a meal and can mark her as eating elsewhere', async () => {
    render(
      <OccasionCard occasion={curry} week={week([curry])} sheet={false} onClose={vi.fn()} onMove={vi.fn()} onCopy={vi.fn()} onDelete={vi.fn()} />,
    );
    expect(screen.getByRole('heading', { name: 'Tuesday dinner' })).toBeInTheDocument();
    expect(screen.getByText('1 still to sort')).toBeInTheDocument();
    expect(screen.getByText('Needs a meal')).toBeInTheDocument();
    expect(screen.getByText('· mild')).toBeInTheDocument();
    expect(screen.getByText('3 eating, +2 spare', { exact: false })).toBeInTheDocument();

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Change what Sarah is eating' }));
    await user.click(screen.getByRole('button', { name: /^Eating elsewhere/ }));

    expect(mocks.setAttendance).toHaveBeenCalledWith({
      occasionId: 'occasion-1',
      memberId: 'sarah',
      attendance: { kind: 'elsewhere' },
    });
  });

  it('does not list the catalogue until something is typed', async () => {
    render(
      <OccasionCard occasion={curry} week={week([curry])} sheet={false} onClose={vi.fn()} onMove={vi.fn()} onCopy={vi.fn()} onDelete={vi.fn()} />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Change what Sarah is eating' }));

    expect(screen.getByText('This meal')).toBeInTheDocument();
    expect(screen.getByText('Quick picks')).toBeInTheDocument();
  });

  it('never offers Eating out in a person menu', async () => {
    render(
      <OccasionCard occasion={curry} week={week([curry])} sheet={false} onClose={vi.fn()} onMove={vi.fn()} onCopy={vi.fn()} onDelete={vi.fn()} />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Change what Sarah is eating' }));

    expect(screen.queryByText('Eating out')).not.toBeInTheDocument();
  });

  it('hides the This meal section once something is typed', async () => {
    render(
      <OccasionCard occasion={curry} week={week([curry])} sheet={false} onClose={vi.fn()} onMove={vi.fn()} onCopy={vi.fn()} onDelete={vi.fn()} />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Change what Sarah is eating' }));
    expect(screen.getByText('This meal')).toBeInTheDocument();

    await user.type(screen.getByRole('textbox', { name: /Something for Sarah/ }), 'a');

    expect(screen.queryByText('This meal')).not.toBeInTheDocument();
  });

  it('edits a variation inside the picker without a second overlay', async () => {
    render(
      <OccasionCard occasion={curry} week={week([curry])} sheet={false} onClose={vi.fn()} onMove={vi.fn()} onCopy={vi.fn()} onDelete={vi.fn()} />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Change what Emily is eating' }));
    await user.click(screen.getByRole('button', { name: 'Change the variation' }));

    const field = screen.getByLabelText('How Emily has it');
    expect(field).toHaveValue('mild');
    await user.clear(field);
    await user.type(field, 'no chilli{Enter}');

    expect(mocks.setAttendance).toHaveBeenCalledWith({
      occasionId: 'occasion-1',
      memberId: 'emily',
      attendance: { kind: 'eating', group_id: 'curry', note: 'no chilli' },
    });
  });

  it('reads Everyone in the roster summary when every member is on the same dish', () => {
    const everyone = occasion({ groups: [group({ id: 'bol', revision: 7 })] });
    render(
      <OccasionCard occasion={everyone} week={week([everyone])} sheet={false} onClose={vi.fn()} onMove={vi.fn()} onCopy={vi.fn()} onDelete={vi.fn()} />,
    );
    expect(screen.getByText('Everyone')).toBeInTheDocument();
    expect(screen.getAllByText('Spaghetti bolognese').length).toBeGreaterThan(0);
  });

  it('moves a member onto another dish from their row menu', async () => {
    const withLasagne = occasion({
      groups: [
        group({ id: 'bol', name: 'Spaghetti bolognese', revision: 7 }),
        group({
          id: 'lasagne',
          name: 'Lasagne',
          everyone: false,
          participants: [{ member_id: 'jack', name: 'Jack', note: null }],
          serves: 1,
          effective_cooking_servings: 1,
        }),
      ],
    });
    render(
      <OccasionCard
        occasion={withLasagne}
        week={week([withLasagne])}
        sheet={false}
        onClose={vi.fn()}
        onMove={vi.fn()}
        onCopy={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Change what Jack is eating' }));
    await user.click(screen.getByRole('button', { name: /^Spaghetti bolognese/ }));

    expect(mocks.setAttendance).toHaveBeenCalledWith({
      occasionId: 'occasion-1',
      memberId: 'jack',
      attendance: { kind: 'eating', group_id: 'bol' },
    });
  });
});
