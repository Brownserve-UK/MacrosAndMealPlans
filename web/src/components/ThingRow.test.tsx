import Button from '@mui/material/Button';
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { KindChip } from './KindChip';
import { ThingRow } from './ThingRow';

describe('the row', () => {
  it('shows a title, a caption and one action', () => {
    render(
      <ThingRow
        concept="dish"
        title="Chilli"
        caption="In the freezer, 4 servings left"
        chip={<KindChip kind="dish" />}
        action={<Button>Plan it</Button>}
      />,
    );

    expect(screen.getByText('Chilli')).toBeInTheDocument();
    expect(screen.getByText('In the freezer, 4 servings left')).toBeInTheDocument();
    expect(screen.getByText('Cooked food')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Plan it' })).toBeInTheDocument();
  });

  it('always leads with an icon, so kinds are told apart before reading', () => {
    const { container } = render(<ThingRow concept="cook" title="Chilli" />);
    expect(container.querySelector('svg')).toBeInTheDocument();
  });

  it('leaves the caption out rather than rendering an empty line', () => {
    const { container } = render(<ThingRow concept="meal" title="Chicken and Rice" />);
    expect(container.querySelectorAll('p')).toHaveLength(0);
  });

  it('names a kind the same way wherever it appears', () => {
    render(
      <>
        <KindChip kind="recipe" />
        <KindChip kind="fridge" />
      </>,
    );
    expect(screen.getByText('Recipe')).toBeInTheDocument();
    expect(screen.getByText('Fridge')).toBeInTheDocument();
  });
});
