import type { ShoppingList } from '../../api/client';

const ml = (amount: number) => ({ amount, unit: 'ml' as const });
const g = (amount: number) => ({ amount, unit: 'g' as const });

const claim = (planned_on: string, amount: number) => ({
  subject: { kind: 'ingredient' as const, ingredient_id: 'milk' },
  quantity: ml(amount),
  entry_id: 'e1',
  planned_on,
  slot: 'breakfast' as const,
  scope: 'member' as const,
  assumed: false,
});

export const shoppingList: ShoppingList = {
  opportunities: [
    { date: '2026-09-05', state: 'normal' },
    { date: '2026-09-12', state: 'normal' },
  ],
  focus: '2026-09-05',
  cadence_configured: true,
  requirements: [
    {
      subject: { kind: 'ingredient', ingredient_id: 'milk' },
      name: 'Whole Milk',
      quantity: ml(600),
      required_by: '2026-09-07',
      use_by_at_least: '2026-09-09',
      section: 'dairy',
      certainty: { kind: 'definite' },
      assignment: { kind: 'opportunity', date: '2026-09-05' },
      claims: [claim('2026-09-07', 300), claim('2026-09-09', 300)],
      gaps: [],
    },
    {
      subject: { kind: 'ingredient', ingredient_id: 'flour' },
      name: 'Plain Flour',
      quantity: g(500),
      section: 'ambient',
      certainty: { kind: 'definite' },
      assignment: { kind: 'needs_earlier_opportunity' },
      claims: [],
      gaps: [],
    },
    {
      subject: { kind: 'ingredient', ingredient_id: 'butter' },
      name: 'Butter',
      quantity: g(250),
      section: 'dairy',
      certainty: { kind: 'definite' },
      assignment: { kind: 'opportunity', date: '2026-09-05' },
      claims: [],
      gaps: [],
      purchase: {
        id: 'p1',
        state: 'pending',
        purchased_at: '2026-09-05T10:00:00Z',
        revision: 1,
      },
    },
  ],
};
