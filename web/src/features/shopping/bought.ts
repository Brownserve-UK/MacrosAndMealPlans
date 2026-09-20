import type { Quantity, ShoppingRequirement, Unit } from '../../api/client';
import { formatQuantity } from '../stock/SpokenFor';
import { purchasesOf } from './requirementKey';

const MASS: Partial<Record<Unit, number>> = {
  mg: 0.001,
  g: 1,
  kg: 1000,
  oz: 28.349523125,
  lb: 453.59237,
};

const VOLUME: Partial<Record<Unit, number>> = {
  ml: 1,
  l: 1000,
  tsp: 4.92892159375,
  tbsp: 14.78676478125,
  fl_oz: 28.4130625,
  cup: 240,
};

function scale(unit: Unit): { table: Partial<Record<Unit, number>>; factor: number } | null {
  if (MASS[unit] != null) return { table: MASS, factor: MASS[unit]! };
  if (VOLUME[unit] != null) return { table: VOLUME, factor: VOLUME[unit]! };
  return null;
}

export function convert(quantity: Quantity, to: Unit): number | null {
  if (quantity.unit === to) return quantity.amount;
  const from = scale(quantity.unit);
  const target = scale(to);
  if (!from || !target || from.table !== target.table) return null;
  return (quantity.amount * from.factor) / target.factor;
}

export type Bought = {
  total: Quantity;
  difference: number;
  incomplete: boolean;
};

export function readable(quantity: Quantity): string {
  const bigger: Partial<Record<Unit, Unit>> = { g: 'kg', ml: 'l' };
  const up = bigger[quantity.unit];
  if (up && quantity.amount >= 1000) {
    const converted = convert(quantity, up);
    if (converted != null) return formatQuantity({ amount: converted, unit: up });
  }
  return formatQuantity(quantity);
}

export function boughtSentence(bought: Bought): string {
  const total = readable(bought.total);
  if (bought.incomplete) return `${total} in the trolley, plus something not written down yet`;
  const off = Math.abs(bought.difference);
  if (off < 0.0005) return `${total} in the trolley`;
  const gap = readable({ amount: Math.round(off * 1000) / 1000, unit: bought.total.unit });
  return bought.difference > 0
    ? `${total} in the trolley, ${gap} more than needed`
    : `${total} in the trolley, ${gap} short`;
}

export function boughtSoFar(requirement: ShoppingRequirement): Bought | null {
  const purchases = purchasesOf(requirement);
  if (purchases.length === 0) return null;

  const unit = requirement.quantity?.unit ?? purchases.find((p) => p.quantity)?.quantity?.unit;
  if (!unit) return { total: { amount: 0, unit: 'item' }, difference: 0, incomplete: true };

  let total = 0;
  let incomplete = false;
  for (const purchase of purchases) {
    if (!purchase.quantity) {
      incomplete = true;
      continue;
    }
    const converted = convert(purchase.quantity, unit);
    if (converted == null) {
      incomplete = true;
      continue;
    }
    total += converted;
  }

  return {
    total: { amount: Math.round(total * 1000) / 1000, unit },
    difference: total - (requirement.quantity?.amount ?? 0),
    incomplete,
  };
}
