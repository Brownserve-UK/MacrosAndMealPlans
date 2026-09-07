import type { ShoppingRequirement } from '../../api/client';
import { sectionRank } from './sections';

export function isSuggested(requirement: ShoppingRequirement): boolean {
  return requirement.certainty.kind === 'suggested';
}

export function groupBySection(
  requirements: ShoppingRequirement[],
  order?: string[],
): Array<[string, ShoppingRequirement[]]> {
  const sections = new Map<string, ShoppingRequirement[]>();
  for (const requirement of requirements) {
    const bucket = sections.get(requirement.section) ?? [];
    bucket.push(requirement);
    sections.set(requirement.section, bucket);
  }
  for (const bucket of sections.values()) {
    bucket.sort((a, b) => Number(isSuggested(a)) - Number(isSuggested(b)));
  }
  const rank = (section: string) =>
    order ? (order.indexOf(section) === -1 ? order.length : order.indexOf(section))
          : sectionRank(section as never);
  return [...sections.entries()].sort((a, b) => rank(a[0]) - rank(b[0]));
}
