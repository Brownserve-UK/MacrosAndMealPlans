import type { ShoppingSection } from '../../api/client';

export const SECTION_ORDER: ShoppingSection[] = [
  'fresh_produce',
  'meat_fish',
  'dairy',
  'bakery',
  'frozen',
  'ambient',
  'drinks',
  'household',
  'other',
];

const LABELS: Record<ShoppingSection, string> = {
  fresh_produce: 'Fresh produce',
  meat_fish: 'Meat & fish',
  dairy: 'Dairy',
  bakery: 'Bakery',
  frozen: 'Frozen',
  ambient: 'Ambient',
  drinks: 'Drinks',
  household: 'Household',
  other: 'Other',
};

export function sectionLabel(section: ShoppingSection): string {
  return LABELS[section] ?? 'Other';
}

export function sectionRank(section: ShoppingSection): number {
  const index = SECTION_ORDER.indexOf(section);
  return index === -1 ? SECTION_ORDER.length : index;
}
