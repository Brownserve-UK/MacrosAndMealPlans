import { describe, expect, it } from 'vitest';
import { mealsCaption, plannedCaption } from './forMeals';

describe('which meals an item is for', () => {
  it('names a single meal outright', () => {
    expect(mealsCaption([{ name: 'Spaghetti bolognese', planned_on: '2026-09-17', slot: 'dinner' }])).toBe(
      'Spaghetti bolognese',
    );
  });

  it('names the first meal and counts the rest', () => {
    expect(
      mealsCaption([
        { name: 'Spaghetti bolognese', planned_on: '2026-09-17', slot: 'dinner' },
        { name: 'Soup and bread', planned_on: '2026-09-19', slot: 'lunch' },
        { name: 'Roast chicken', planned_on: '2026-09-20', slot: 'lunch' },
      ]),
    ).toBe('Spaghetti bolognese and 2 more');
  });

  it('says nothing when no meal needs it', () => {
    expect(mealsCaption([])).toBeNull();
  });
});

describe('how much of a trip is planned', () => {
  it('reads naturally for none, one and many', () => {
    expect(plannedCaption(0)).toBe('nothing planned yet');
    expect(plannedCaption(1)).toBe('1 for a planned meal');
    expect(plannedCaption(6)).toBe('6 for planned meals');
  });
});
