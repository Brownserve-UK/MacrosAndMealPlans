import type { ShoppingListItem, ShoppingRequirement } from '../../api/client';

export type Listed = { key: string; requirement: ShoppingRequirement };

export type Row =
  | { kind: 'requirement'; key: string; requirement: ShoppingRequirement }
  | { kind: 'manual'; item: ShoppingListItem };

export function isSuggested(requirement: ShoppingRequirement): boolean {
  return requirement.certainty.kind === 'suggested';
}

function suggested(row: Row): boolean {
  return row.kind === 'requirement' && isSuggested(row.requirement);
}

export function groupBySection(
  listed: Listed[],
  manual: ShoppingListItem[] = [],
): Array<[string, Row[]]> {
  const sections = new Map<string, Row[]>();

  const put = (section: string, row: Row) => {
    const bucket = sections.get(section) ?? [];
    bucket.push(row);
    sections.set(section, bucket);
  };

  for (const entry of listed) {
    put(entry.requirement.section, { kind: 'requirement', ...entry });
  }
  for (const item of manual) {
    put(item.section ?? 'other', { kind: 'manual', item });
  }

  for (const bucket of sections.values()) {
    bucket.sort((a, b) => Number(suggested(a)) - Number(suggested(b)));
  }
  return [...sections.entries()];
}
