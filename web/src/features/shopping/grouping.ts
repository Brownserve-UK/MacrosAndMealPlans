import type { ShoppingRequirement } from '../../api/client';
import { sectionRank } from './sections';

export function groupBySection(
  requirements: ShoppingRequirement[],
): Array<[string, ShoppingRequirement[]]> {
  const sections = new Map<string, ShoppingRequirement[]>();
  for (const requirement of requirements) {
    const bucket = sections.get(requirement.section) ?? [];
    bucket.push(requirement);
    sections.set(requirement.section, bucket);
  }
  return [...sections.entries()].sort(
    (a, b) => sectionRank(a[0] as never) - sectionRank(b[0] as never),
  );
}
