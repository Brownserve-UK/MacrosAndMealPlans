import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { PlannerLens } from './PlannerLens';

describe('PlannerLens', () => {
  it('reports a controlled lens change', async () => {
    const onChange = vi.fn();
    render(<PlannerLens lens="mine" onChange={onChange} show />);
    await userEvent.setup().click(screen.getByRole('button', { name: 'Household' }));
    expect(onChange).toHaveBeenCalledWith('household');
  });

  it('renders nothing when only one lens is available', () => {
    render(<PlannerLens lens="mine" onChange={vi.fn()} show={false} />);
    expect(screen.queryByRole('button', { name: 'Mine' })).not.toBeInTheDocument();
  });
});
