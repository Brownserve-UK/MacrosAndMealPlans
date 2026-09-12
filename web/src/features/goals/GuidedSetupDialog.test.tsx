import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { WeightSummary } from '../../api/client';
import { GuidedSetupDialog } from './GuidedSetupDialog';

const mocks = vi.hoisted(() => ({ preview: vi.fn(), save: vi.fn() }));

vi.mock('../../api/queries', () => ({
  usePreviewCalorieTarget: () => ({ mutateAsync: mocks.preview, isPending: false }),
  useSetGuidedCalorieTarget: () => ({ mutateAsync: mocks.save, isPending: false }),
}));

function renderDialog() {
  render(<GuidedSetupDialog memberId="member-1" summary={{ series: [] } as WeightSummary} onClose={vi.fn()} onManual={vi.fn()} />);
}

describe('GuidedSetupDialog', () => {
  it('takes the member through the questions before showing a recommendation', async () => {
    const user = userEvent.setup();
    mocks.preview.mockResolvedValue({ recommended_kcal: 2150, maintenance_kcal: 2500, adjustment_kcal: -350, floor_kcal: 1200, eased: false });
    renderDialog();
    await user.type(screen.getByLabelText('Date of birth'), '1988-01-01');
    await user.click(screen.getByLabelText('Sex'));
    await user.click(screen.getByRole('option', { name: 'Female' }));
    await user.type(screen.getByLabelText('Height'), '165');
    await user.type(screen.getByLabelText('Current weight'), '70');
    await user.click(screen.getByRole('button', { name: 'Continue' }));
    expect(screen.getByText('Your usual day')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /Lightly active/ }));
    await user.click(screen.getByRole('button', { name: 'Continue' }));
    await user.click(screen.getByRole('button', { name: 'Lose' }));
    await user.type(screen.getByLabelText('Goal weight'), '62');
    await user.click(screen.getByRole('button', { name: 'See my target' }));
    expect(await screen.findByText('2,150 kcal')).toBeInTheDocument();
    expect(screen.getByText('General guidance, not advice from a nutrition professional.')).toBeInTheDocument();
  });

  it('does not ask for a goal weight when maintaining', async () => {
    const user = userEvent.setup();
    renderDialog();
    await user.type(screen.getByLabelText('Date of birth'), '1988-01-01');
    await user.click(screen.getByLabelText('Sex'));
    await user.click(screen.getByRole('option', { name: 'Female' }));
    await user.type(screen.getByLabelText('Height'), '165');
    await user.type(screen.getByLabelText('Current weight'), '70');
    await user.click(screen.getByRole('button', { name: 'Continue' }));
    await user.click(screen.getByRole('button', { name: /Lightly active/ }));
    await user.click(screen.getByRole('button', { name: 'Continue' }));
    await user.click(screen.getByRole('button', { name: 'Maintain' }));
    expect(screen.queryByLabelText('Goal weight')).not.toBeInTheDocument();
  });
});
