import type { Purchase, ShoppingRequirement } from '../../api/client';

export function requirementKey(requirement: ShoppingRequirement): string {
  const subject = requirement.subject;
  const on =
    requirement.assignment.kind === 'opportunity' ? requirement.assignment.date : requirement.assignment.kind;
  if (subject.kind === 'ingredient') return `ingredient:${subject.ingredient_id}@${on}`;
  if (subject.kind === 'prepared_meal') return `prepared_meal:${subject.prepared_meal_id}@${on}`;
  if (subject.kind === 'prepared_portion') return `prepared:${subject.prepared_batch_id}@${on}`;
  if (subject.kind === 'cooked_food') return `cooked:${subject.recipe_id}@${on}`;
  return `product:${subject.product_id}@${on}`;
}

export function purchasesOf(requirement: ShoppingRequirement): Purchase[] {
  return requirement.purchases ?? [];
}
