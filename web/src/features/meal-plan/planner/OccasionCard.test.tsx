import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { group, occasion, week } from './fixtures';
import { OccasionCard } from './OccasionCard';

const mocks = vi.hoisted(() => ({
  setAttendance: vi.fn(),
  updateGroup: vi.fn(),
  addGuest: vi.fn(),
  changeGuest: vi.fn(),
  idle: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));
vi.mock('../../../api/queries', () => ({
  useUpdateOccasion: mocks.idle,
  useAddGroup: mocks.idle,
  useUpdateGroup: () => ({ mutateAsync: mocks.updateGroup, isPending: false }),
  useSetAttendance: () => ({ mutateAsync: mocks.setAttendance, isPending: false }),
  useAddPlannerGuest: () => ({ mutateAsync: mocks.addGuest, isPending: false }),
  useChangePlannerGuest: () => ({ mutateAsync: mocks.changeGuest, isPending: false }),
  useRemovePlannerGuest: mocks.idle,
  useSplitPlannerGuests: mocks.idle,
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
    mocks.addGuest.mockResolvedValue(curry);
    mocks.changeGuest.mockResolvedValue(curry);
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

  it('adds a guest with a separate meal', async () => {
    render(<OccasionCard occasion={curry} week={week([curry])} sheet={false} onClose={vi.fn()} onMove={vi.fn()} onCopy={vi.fn()} onDelete={vi.fn()} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Add a guest' }));
    await user.type(screen.getByRole('textbox', { name: 'Something for guest' }), 'Vegetarian dinner');
    await user.click(screen.getByRole('button', { name: /Add “Vegetarian dinner”/ }));
    expect(mocks.addGuest).toHaveBeenCalledWith({
      occasionId: 'occasion-1',
      revision: curry.revision,
      name: null,
      target: { new_group: { label: 'Vegetarian dinner' } },
    });
  });

  it('moves one guest while leaving another on the original meal', async () => {
    const withGuests = occasion({
      groups: [
        group({ id: 'curry', name: 'Curry', guests: [{ id: 'alex', name: 'Alex', note: null, count: 1 }, { id: 'robin', name: 'Robin', note: null, count: 1 }], guest_count: 2 }),
        group({ id: 'pizza', name: 'Pizza', everyone: false, guests: [], guest_count: 0 }),
      ],
    });
    render(<OccasionCard occasion={withGuests} week={week([withGuests])} sheet={false} onClose={vi.fn()} onMove={vi.fn()} onCopy={vi.fn()} onDelete={vi.fn()} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Change Alex' }));
    await user.click(screen.getByRole('button', { name: /^Pizza/ }));
    expect(mocks.changeGuest).toHaveBeenCalledWith({
      occasionId: 'occasion-1', guestId: 'alex', revision: withGuests.revision,
      target: { group_id: 'pizza' },
    });
  });

  it('edits a guest name from the row and adds a variation from the meal picker', async () => {
    const withGuest = occasion({ groups: [group({ id: 'curry', name: 'Curry', guests: [{ id: 'alex', name: 'Alex', note: null, count: 1 }], guest_count: 1 })] });
    render(<OccasionCard occasion={withGuest} week={week([withGuest])} sheet={false} onClose={vi.fn()} onMove={vi.fn()} onCopy={vi.fn()} onDelete={vi.fn()} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Edit name for Alex' }));
    const name = screen.getByRole('textbox', { name: 'Name for Alex' });
    await user.clear(name);
    await user.type(name, 'Morgan{Enter}');
    expect(mocks.changeGuest).toHaveBeenCalledWith({ occasionId: 'occasion-1', guestId: 'alex', revision: withGuest.revision, name: 'Morgan' });
    await user.click(screen.getByRole('button', { name: 'Change Alex' }));
    expect(screen.queryByRole('textbox', { name: 'Guest name (optional)' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Add a variation' }));
    await user.type(screen.getByRole('textbox', { name: 'How Alex has it' }), 'no chilli{Enter}');
    expect(mocks.changeGuest).toHaveBeenCalledWith({ occasionId: 'occasion-1', guestId: 'alex', revision: withGuest.revision, note: 'no chilli' });
  });
});
