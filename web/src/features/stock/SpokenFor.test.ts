import { describe, expect, it } from 'vitest';
import type { DemandClaim, Unit } from '../../api/client';
import { shareSentence } from './SpokenFor';

const claim = (amount: number, unit: Unit = 'ml'): DemandClaim => ({
  subject: { kind: 'ingredient', ingredient_id: 'milk' },
  quantity: { amount, unit },
  entry_id: 'e1',
  planned_on: '2026-09-07',
  slot: 'breakfast',
  scope: 'member',
  assumed: false,
});

describe('shareSentence', () => {
  it('says nothing when no meal asked for the ingredient', () => {
    expect(shareSentence([], null, ['Other Milk'])).toBeNull();
  });

  it('explains that a product already spoken for lends none of it', () => {
    const sentence = shareSentence([claim(400)], null, ['Other Milk']);
    expect(sentence).toContain('None of that is expected to come from here');
    expect(sentence).toContain('It can come from Other Milk.');
  });

  it('does not offer a remainder when the whole want comes from here', () => {
    const sentence = shareSentence([claim(400)], { amount: 400, unit: 'ml' }, ['Other Milk']);
    expect(sentence).toBe('We expect all of that to come from here.');
  });

  it('splits the want when only part of it comes from here', () => {
    const sentence = shareSentence([claim(400)], { amount: 150, unit: 'ml' }, ['Other Milk']);
    expect(sentence).toBe(
      'We expect 150 ml of that to come from here. The rest can come from Other Milk.',
    );
  });

  it('stays quiet rather than adding up quantities it cannot compare', () => {
    expect(shareSentence([claim(400, 'ml'), claim(1, 'l')], null, [])).toBeNull();
  });
});
