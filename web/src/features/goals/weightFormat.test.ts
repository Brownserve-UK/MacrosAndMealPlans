import { describe, expect, it } from 'vitest';
import {
  formatRate,
  formatWeight,
  formatWeightChange,
  parseRateInput,
  parseWeightInput,
  parseWeightInputKg,
  rateToInput,
  toInput,
  unitFor,
} from './weightFormat';

describe('formatWeight', () => {
  it('shows kilograms to a tenth, with a space before the unit', () => {
    expect(formatWeight(72.44, 'kilograms')).toBe('72.4 kg');
    expect(formatWeight(80, 'kilograms')).toBe('80 kg');
  });

  it('shows pounds', () => {
    expect(formatWeight(72.575, 'pounds')).toBe('160 lb');
  });

  it('splits stones and pounds', () => {
    expect(formatWeight(72.575, 'stones_pounds')).toBe('11 st 6 lb');
    expect(formatWeight(88.9, 'stones_pounds')).toBe('14 st 0 lb');
  });

  it('carries into the next stone rather than showing fourteen pounds', () => {
    expect(formatWeight(88.891, 'stones_pounds')).toBe('14 st 0 lb');
    expect(formatWeight(88.878, 'stones_pounds')).toBe('13 st 13.9 lb');
  });
});


describe('formatWeightChange', () => {
  it('always carries a sign so a gain and a loss cannot be confused', () => {
    expect(formatWeightChange(-2.6, 'kilograms')).toBe('−2.6 kg');
    expect(formatWeightChange(1.2, 'kilograms')).toBe('+1.2 kg');
  });

  it('says so plainly when nothing has changed', () => {
    expect(formatWeightChange(0, 'kilograms')).toBe('No change');
  });
});

describe('formatRate', () => {
  it('uses pounds a week under either imperial display', () => {
    expect(formatRate(0.5, 'kilograms')).toBe('0.5 kg a week');
    expect(formatRate(0.5, 'pounds')).toBe('1.1 lb a week');
    expect(formatRate(0.5, 'stones_pounds')).toBe('1.1 lb a week');
  });
});

describe('parseWeightInput', () => {
  it('sends kilograms as kilograms', () => {
    expect(parseWeightInput({ primary: '72.4', secondary: '' }, 'kilograms')).toEqual({
      amount: 72.4,
      unit: 'kg',
    });
  });

  it('collapses stones and pounds into plain pounds', () => {
    expect(parseWeightInput({ primary: '11', secondary: '6' }, 'stones_pounds')).toEqual({
      amount: 160,
      unit: 'lb',
    });
  });

  it('treats an empty pounds box as zero pounds', () => {
    expect(parseWeightInput({ primary: '11', secondary: '' }, 'stones_pounds')).toEqual({
      amount: 154,
      unit: 'lb',
    });
  });

  it('refuses nonsense', () => {
    expect(parseWeightInput({ primary: '', secondary: '' }, 'kilograms')).toBeNull();
    expect(parseWeightInput({ primary: 'heavy', secondary: '' }, 'kilograms')).toBeNull();
    expect(parseWeightInput({ primary: '0', secondary: '' }, 'kilograms')).toBeNull();
    expect(parseWeightInput({ primary: '-5', secondary: '' }, 'kilograms')).toBeNull();
  });

  it('refuses fourteen pounds or more, because that is another stone', () => {
    expect(parseWeightInput({ primary: '11', secondary: '14' }, 'stones_pounds')).toBeNull();
  });

  it('converts back to kilograms for anything drawn before saving', () => {
    const kg = parseWeightInputKg({ primary: '11', secondary: '6' }, 'stones_pounds');
    expect(kg).toBeCloseTo(72.57, 1);
  });
});

describe('toInput', () => {
  it('round trips a weight through the form', () => {
    expect(toInput(72.4, 'kilograms')).toEqual({ primary: '72.4', secondary: '' });
    expect(toInput(72.575, 'stones_pounds')).toEqual({ primary: '11', secondary: '6' });
    expect(toInput(null, 'kilograms')).toEqual({ primary: '', secondary: '' });
  });
});

describe('rates', () => {
  it('is one box whatever the display', () => {
    expect(parseRateInput('0.5', 'kilograms')).toEqual({ amount: 0.5, unit: 'kg' });
    expect(parseRateInput('1', 'stones_pounds')).toEqual({ amount: 1, unit: 'lb' });
    expect(parseRateInput('0', 'kilograms')).toBeNull();
    expect(parseRateInput('', 'kilograms')).toBeNull();
  });

  it('fills the box from a stored rate', () => {
    expect(rateToInput(0.5, 'kilograms')).toBe('0.5');
    expect(rateToInput(0.5, 'pounds')).toBe('1.1');
    expect(rateToInput(null, 'kilograms')).toBe('');
  });
});

describe('unitFor', () => {
  it('sends stones as pounds, because there is no stone unit', () => {
    expect(unitFor('kilograms')).toBe('kg');
    expect(unitFor('pounds')).toBe('lb');
    expect(unitFor('stones_pounds')).toBe('lb');
  });
});
