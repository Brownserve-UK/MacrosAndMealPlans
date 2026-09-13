import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { WeightSummary } from '../../api/client';
import { GuidedSetupDialog } from './GuidedSetupDialog';

const mocks = vi.hoisted(() => ({ preview: vi.fn(), save: vi.fn() }));
vi.mock('../../api/queries', () => ({ usePreviewCalorieTarget: () => ({ mutateAsync: mocks.preview, isPending: false }), useSetGuidedCalorieTarget: () => ({ mutateAsync: mocks.save, isPending: false }) }));

const recommendation = { calculation: { recommended_kcal: 2150, maintenance_kcal: 2500, adjustment_kcal: -350, floor_kcal: 1200, eased: false }, macros: { protein_g: 125, carbohydrate_g: 260, fat_g: 65 }, estimated_goal_date: '2026-12-20' };
function renderDialog() { render(<GuidedSetupDialog memberId="member-1" summary={{ series: [] } as WeightSummary} onClose={vi.fn()} onManual={vi.fn()} />); }
async function fillAbout(user: ReturnType<typeof userEvent.setup>) { await user.type(await screen.findByLabelText('Date of birth'), '1988-01-01'); await user.click(screen.getByLabelText('Sex')); await user.click(screen.getByRole('option', { name: 'Female' })); await user.type(screen.getByLabelText('Height'), '165'); await user.type(screen.getByLabelText('Current weight'), '70'); await user.click(screen.getByRole('button', { name: 'Continue' })); }

describe('GuidedSetupDialog', () => {
  beforeEach(() => { vi.clearAllMocks(); mocks.preview.mockResolvedValue(recommendation); });
  it('uses the approved question order and shows calorie and macro targets', async () => {
    const user = userEvent.setup(); renderDialog();
    await user.click(screen.getByRole('button', { name: /Lose weight/ }));
    expect(await screen.findByText('What matters most?')).toBeInTheDocument();
    await user.click(await screen.findByRole('button', { name: /Build muscle/ }));
    await fillAbout(user);
    await user.click(await screen.findByRole('button', { name: /Lightly active/ }));
    expect(await screen.findByText('Goal weight & pace')).toBeInTheDocument();
    expect(screen.getByText('0.5 kg/week · 1.1 lb/week')).toBeInTheDocument();
    await user.type(screen.getByLabelText('Goal weight'), '62');
    await user.click(screen.getByRole('button', { name: 'See my targets' }));
    expect(await screen.findByText('2,150 kcal')).toBeInTheDocument();
    expect(screen.getByText('125 g')).toBeInTheDocument();
    expect(mocks.preview).toHaveBeenCalledWith(expect.objectContaining({ body: expect.objectContaining({ objective: 'lose', emphasis: 'muscle' }) }));
  });

  it('skips goal weight and pace when maintaining', async () => {
    const user = userEvent.setup(); renderDialog();
    await user.click(screen.getByRole('button', { name: /Maintain weight/ }));
    await user.click(await screen.findByRole('button', { name: /General & balanced/ }));
    await fillAbout(user);
    await user.click(await screen.findByRole('button', { name: /Lightly active/ }));
    expect(await screen.findByText('Your targets')).toBeInTheDocument();
    expect(screen.queryByLabelText('Goal weight')).not.toBeInTheDocument();
    expect(mocks.preview).toHaveBeenCalledWith(expect.objectContaining({ body: expect.objectContaining({ target_weight: null, pace: null }) }));
  });
});
