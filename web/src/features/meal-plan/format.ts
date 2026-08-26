import type { Amount } from '../../api/client';
import { displayUnit } from '../../components/UnitSelect';

export function formatAmount(amount: Amount): string {
  if (amount.kind === 'measure') return `${amount.value} ${displayUnit(amount.unit)}`;
  if (amount.kind === 'servings') return amount.value === 1 ? '1 serving' : `${amount.value} servings`;
  return amount.value === 1 ? '1 pack' : `${amount.value} packs`;
}
