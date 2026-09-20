import type { MealSlot } from '../../api/client';

export const SLOTS: { value: MealSlot; label: string }[] = [
  { value: 'breakfast', label: 'Breakfast' },
  { value: 'lunch', label: 'Lunch' },
  { value: 'dinner', label: 'Dinner' },
  { value: 'snacks', label: 'Snacks' },
];

export const MAIN_SLOTS = SLOTS.filter((slot) => slot.value !== 'snacks');

export function labelForSlot(slot: MealSlot) {
  return SLOTS.find((candidate) => candidate.value === slot)?.label ?? slot;
}
