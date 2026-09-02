import type { DemandGap } from '../../api/client';

const LABELS: Record<DemandGap, string> = {
  ingredient_has_no_products: 'No products for this ingredient',
  unresolved_recipe_line: 'Unmatched recipe lines',
  recipe_missing: 'A planned recipe is missing',
  product_missing: 'A planned product is missing',
  amount_unresolvable: "An amount can't be worked out",
  incompatible_units: "Units don't match",
};

export function gapLabel(gaps: DemandGap[]): string | null {
  const first = gaps[0];
  return first ? LABELS[first] : null;
}
