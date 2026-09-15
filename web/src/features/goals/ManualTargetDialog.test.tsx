import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ManualTargetDialog } from './ManualTargetDialog';

const mocks = vi.hoisted(() => ({ save: vi.fn() }));
vi.mock('../../api/queries', () => ({ useSetManualCalorieTarget: () => ({ mutateAsync: mocks.save, isPending: false }) }));

describe('ManualTargetDialog', () => {
  beforeEach(() => { vi.clearAllMocks(); mocks.save.mockResolvedValue({}); });

  it('sets calorie and macro targets together', async () => {
    const user = userEvent.setup();
    render(<ManualTargetDialog memberId="member-1" onClose={vi.fn()} />);

    await user.type(screen.getByLabelText('Daily calories'), '2100');
    await user.type(screen.getByLabelText('Protein'), '140');
    await user.type(screen.getByLabelText('Carbs'), '245');
    await user.type(screen.getByLabelText('Fat'), '65');
    await user.click(screen.getByRole('button', { name: 'Use these targets' }));

    expect(mocks.save).toHaveBeenCalledWith({
      id: 'member-1',
      body: { energy_kcal: 2100, protein_g: 140, carbohydrate_g: 245, fat_g: 65 },
    });
  });
});
