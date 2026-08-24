import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { MaybeNumber } from './Unknown';

describe('unknown values', () => {
  it('renders a dash rather than zero when a number is missing', () => {
    render(<MaybeNumber value={null} suffix="g" />);
    expect(screen.getByLabelText('Not known')).toBeInTheDocument();
    expect(screen.queryByText('0')).not.toBeInTheDocument();
  });

  it('renders an actual zero as zero', () => {
    render(<MaybeNumber value={0} suffix="g" />);
    expect(screen.getByText('0')).toBeInTheDocument();
    expect(screen.queryByLabelText('Not known')).not.toBeInTheDocument();
  });

  it('formats a decimal for reading', () => {
    render(<MaybeNumber value={64.53} />);
    expect(screen.getByText('64.5')).toBeInTheDocument();
  });

});
