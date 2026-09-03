import type { ShoppingRequirement } from '../../api/client';

export function requirementKey(requirement: ShoppingRequirement): string {
  const subject = requirement.subject;
  return subject.kind === 'ingredient'
    ? `ingredient:${subject.ingredient_id}`
    : `product:${subject.product_id}`;
}
