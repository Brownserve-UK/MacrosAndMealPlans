import type { ShoppingList, ShoppingRequirement } from '../../api/client';
import type { MealNeed } from './forMeals';

type Count = ShoppingList['counts'][number] & { planned_count: number };
type Requirement = ShoppingRequirement & { for_meals?: MealNeed[] };
type Fixture = Omit<ShoppingList, 'counts' | 'requirements'> & {
  counts: Count[];
  requirements: Requirement[];
};

const ml = (amount: number) => ({ amount, unit: 'ml' as const });
const g = (amount: number) => ({ amount, unit: 'g' as const });

const claim = (planned_on: string, amount: number) => ({
  subject: { kind: 'ingredient' as const, ingredient_id: 'milk' },
  quantity: ml(amount),
  entry_id: 'e1',
  group_name: planned_on === '2026-09-07' ? 'Porridge' : 'Pancakes',
  planned_on,
  slot: 'breakfast' as const,
  assumed: false,
});

const fixture: Fixture = {
  opportunities: [
    { date: '2026-09-05', state: 'normal', revision: 0 },
    { date: '2026-09-12', state: 'normal', revision: 0 },
    { date: '2026-09-19', state: 'normal', revision: 0 },
  ],
  focus: '2026-09-05',
  cadence_configured: true,
  unfinished: [],
  section_order: [
    'fresh_produce',
    'meat_fish',
    'dairy',
    'bakery',
    'frozen',
    'ambient',
    'drinks',
    'household',
    'other',
  ],
  manual: [],
  unplanned: [],
  counts: [
    { date: '2026-09-05', items: 3, planned_count: 2 },
    { date: '2026-09-12', items: 8, planned_count: 0 },
    { date: '2026-09-19', items: 0, planned_count: 0 },
  ],
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
      for_meals: [
        { name: 'Porridge', planned_on: '2026-09-07', slot: 'breakfast' },
        { name: 'Pancakes', planned_on: '2026-09-09', slot: 'breakfast' },
      ],
    },
    {
      subject: { kind: 'ingredient', ingredient_id: 'flour' },
      name: 'Plain Flour',
      quantity: g(500),
      required_by: '2026-09-03',
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
      purchases: [
        {
          id: 'p1',
          state: 'pending',
          purchased_at: '2026-09-05T10:00:00Z',
          revision: 1,
        },
        {
          id: 'p2',
          state: 'pending',
          product_id: 'butter-1',
          quantity: g(500),
          purchased_at: '2026-09-05T10:05:00Z',
          revision: 1,
        },
      ],
    },
  ],
};

export const shoppingList: ShoppingList = fixture;
