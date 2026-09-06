import type { Purchase, ShoppingRequirement } from '../../api/client';

export function requirementKey(requirement: ShoppingRequirement): string {
  const subject = requirement.subject;
  if (subject.kind === 'ingredient') return `ingredient:${subject.ingredient_id}`;
  if (subject.kind === 'prepared_portion') return `prepared:${subject.prepared_batch_id}`;
  return `product:${subject.product_id}`;
}

export function purchasesOf(requirement: ShoppingRequirement): Purchase[] {
  return requirement.purchases ?? [];
}
