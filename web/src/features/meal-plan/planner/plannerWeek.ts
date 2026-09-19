import type { MealPlanComponent, MealSlot } from '../../../api/client';
import { addDays, parseIsoDate } from '../date';
import { labelForSlot } from '../slots';
import type {
  GroupView,
  NewGroup,
  NewGroupComponent,
  OccasionView,
  PickerRow,
  PlannerMember,
  PlannerWeek,
} from './types';

export const SLOT_ORDER: MealSlot[] = ['breakfast', 'lunch', 'dinner', 'snacks'];

export type CellMeta = { text: string; tone: 'quiet' | 'buy' };

export type CellSummary = { name: string; tail: string | null; meta: CellMeta[] };

export function weekDates(weekStart: string): string[] {
  return Array.from({ length: 7 }, (_, index) => addDays(weekStart, index));
}

export function occasionAt(
  week: PlannerWeek | undefined,
  date: string,
  slot: MealSlot,
): OccasionView | null {
  const day = week?.days.find((candidate) => candidate.date === date);
  if (!day) return null;
  const byIndex = day.occasions[SLOT_ORDER.indexOf(slot)];
  if (byIndex && byIndex.slot === slot) return byIndex;
  return day.occasions.find((occasion) => occasion?.slot === slot) ?? null;
}

export function formatMinutes(minutes: number): string {
  if (minutes < 60) return `${minutes} mins`;
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest === 0 ? `${hours} h` : `${hours} h ${rest}`;
}

export function memberName(members: PlannerMember[], id: string): string {
  return members.find((member) => member.id === id)?.name ?? 'Someone';
}

export function firstName(name: string): string {
  return name.split(/\s+/)[0] ?? name;
}

export function initialsOf(member: PlannerMember): string {
  if (member.initials) return member.initials;
  return member.name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((word) => word[0]?.toUpperCase() ?? '')
    .join('');
}

export type JoinedName = { head: string; tail: string | null };

export function joinFoodNames(names: string[]): JoinedName {
  if (names.length === 0) return { head: '', tail: null };
  if (names.length === 1) return { head: names[0]!, tail: null };
  if (names.length === 2) return { head: names[0]!, tail: `& ${names[1]}` };
  if (names.length === 3) return { head: `${names[0]}, ${names[1]}`, tail: `& ${names[2]}` };
  return { head: `${names[0]}, ${names[1]}`, tail: `+${names.length - 2}` };
}

export function groupDisplayName(group: GroupView): JoinedName {
  if (group.label && group.label.trim() !== '') return { head: group.label, tail: null };
  if (group.ad_hoc) return { head: group.name, tail: null };
  if (isLeftoversGroup(group)) return { head: dishLabel(group), tail: null };
  return joinFoodNames(group.components.map((component) => component.item_name));
}

export function groupShortName(group: GroupView): JoinedName {
  if (group.label && group.label.trim() !== '') return { head: group.label, tail: null };
  if (group.ad_hoc) return { head: group.name, tail: null };
  if (isLeftoversGroup(group)) return { head: dishLabel(group), tail: null };
  const names = group.components.map((component) => component.item_name);
  if (names.length <= 1) return { head: names[0] ?? group.name, tail: null };
  return { head: names[0]!, tail: `+${names.length - 1}` };
}

export function joinFoodNamesPlain(names: string[]): string {
  const { head, tail } = joinFoodNames(names);
  return tail ? `${head} ${tail}` : head;
}

export function groupFoodCaption(group: GroupView): string | null {
  if (!group.label || group.label.trim() === '') return null;
  if (group.components.length === 0) return null;
  return joinFoodNamesPlain(group.components.map((component) => component.item_name));
}

export function cellSummary(occasion: OccasionView, members: PlannerMember[]): CellSummary {
  const [first, ...rest] = occasion.groups;
  const meta: CellMeta[] = [];
  if (rest.length > 0) {
    meta.push({ text: rest.length === 1 ? '+1 separate meal' : `+${rest.length} separate meals`, tone: 'quiet' });
  }
  if (occasion.absent_member_ids.length === 1) {
    meta.push({ text: `${firstName(memberName(members, occasion.absent_member_ids[0]!))} out`, tone: 'quiet' });
  } else if (occasion.absent_member_ids.length > 1) {
    meta.push({ text: `${occasion.absent_member_ids.length} out`, tone: 'quiet' });
  }
  const guests = occasion.groups.reduce((total, group) => total + group.guest_count, 0);
  if (guests > 0) meta.push({ text: `+${guests} ${guests === 1 ? 'guest' : 'guests'}`, tone: 'quiet' });
  if (first?.cook_minutes) meta.push({ text: formatMinutes(first.cook_minutes), tone: 'quiet' });
  const toBuy = occasion.groups.reduce((total, group) => total + group.to_buy, 0);
  if (toBuy > 0) meta.push({ text: `${toBuy} to buy`, tone: 'buy' });
  const name = first ? groupShortName(first) : { head: 'Nothing planned', tail: null };
  return { name: name.head, tail: name.tail, meta };
}

export function occasionTitle(occasion: OccasionView): string {
  const weekday = parseIsoDate(occasion.planned_on).toLocaleDateString('en-GB', { weekday: 'long' });
  return `${weekday} ${labelForSlot(occasion.slot).toLowerCase()}`;
}

export function shortDate(date: string): string {
  return parseIsoDate(date).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' });
}

export function dayHeading(date: string): string {
  const parsed = parseIsoDate(date);
  return `${parsed.toLocaleDateString('en-GB', { weekday: 'long' })} ${parsed.getDate()}`;
}

export function shortWeekRange(weekStart: string): string {
  const start = parseIsoDate(weekStart);
  const end = parseIsoDate(addDays(weekStart, 6));
  const sameMonth = start.getMonth() === end.getMonth();
  const startLabel = sameMonth
    ? String(start.getDate())
    : start.toLocaleDateString('en-GB', { day: 'numeric', month: 'short' });
  const endLabel = end.toLocaleDateString('en-GB', { day: 'numeric', month: 'short' });
  return `${startLabel} to ${endLabel}`;
}

export function groupDiners(occasion: OccasionView, group: GroupView, members: PlannerMember[]): PlannerMember[] {
  if (!group.everyone) {
    return group.participants
      .map((participant) => members.find((member) => member.id === participant.member_id))
      .filter((member): member is PlannerMember => member !== undefined);
  }
  const elsewhere = new Set<string>(occasion.absent_member_ids);
  for (const other of occasion.groups) {
    if (other.id === group.id) continue;
    for (const participant of other.participants) elsewhere.add(participant.member_id);
  }
  return members.filter((member) => !elsewhere.has(member.id));
}

export type MemberStatus =
  | { kind: 'eating'; group: GroupView }
  | { kind: 'elsewhere' }
  | { kind: 'unaccounted' };

export function memberStatus(
  occasion: OccasionView,
  member: PlannerMember,
  members: PlannerMember[],
): MemberStatus {
  for (const group of occasion.groups) {
    if (groupDiners(occasion, group, members).some((diner) => diner.id === member.id)) {
      return { kind: 'eating', group };
    }
  }
  if (occasion.absent_member_ids.includes(member.id)) return { kind: 'elsewhere' };
  return { kind: 'unaccounted' };
}

export function memberVariation(group: GroupView, memberId: string): string | null {
  return group.participants.find((participant) => participant.member_id === memberId)?.note ?? null;
}

export function isLeftoversGroup(group: GroupView): boolean {
  return groupKind(group) === 'dish' && group.leftover_servings_available != null;
}

export function dishLabel(group: GroupView): string {
  return isLeftoversGroup(group) ? `Leftovers · ${group.name}` : group.name;
}

export function moveTargets(occasion: OccasionView, status: MemberStatus): GroupView[] {
  if (status.kind === 'eating') return occasion.groups;
  const everyoneGroup = occasion.groups.find((group) => group.everyone);
  if (!everyoneGroup) return occasion.groups;
  return [everyoneGroup, ...occasion.groups.filter((group) => group.id !== everyoneGroup.id)];
}

export function groupKind(group: GroupView): 'recipe' | 'product' | 'saved' | 'dish' | 'ad_hoc' | 'label' {
  if (group.ad_hoc) return 'ad_hoc';
  const first = group.components[0];
  if (!first) return 'label';
  if (first.item_kind === 'recipe') return 'recipe';
  if (first.item_kind === 'dish') return 'dish';
  if (first.item_kind === 'product') return 'product';
  return 'saved';
}

export function conceptFor(group: GroupView): PickerRow['concept'] {
  switch (groupKind(group)) {
    case 'recipe':
      return 'recipe';
    case 'dish':
      return 'dish';
    case 'product':
      return 'food';
    case 'saved':
      return 'saved_meal';
    case 'ad_hoc':
      if (group.ad_hoc === 'eating_out') return 'out';
      if (group.ad_hoc === 'takeaway') return 'takeaway';
      return 'fend';
    default:
      return 'meal';
  }
}

export function groupCaption(group: GroupView): string | null {
  const kind = groupKind(group);
  if (kind === 'dish' && group.leftover_servings_available != null) {
    const servings = group.leftover_servings_available;
    return servings === 1 ? '1 serving in the fridge' : `${servings} servings in the fridge`;
  }
  if (group.cook_minutes) return formatMinutes(group.cook_minutes);
  return null;
}

export function toNewGroupComponent(component: MealPlanComponent): NewGroupComponent | null {
  const amount = component.amount;
  const cooking_servings = component.cooking_servings ?? null;
  switch (component.item_kind) {
    case 'product':
      return { product_id: component.product_id, amount, cooking_servings };
    case 'recipe':
      return { recipe_id: component.recipe_id, amount, cooking_servings };
    case 'dish':
      return { dish_recipe_id: component.dish_recipe_id, amount, cooking_servings };
    case 'ingredient':
      return { ingredient_id: component.ingredient_id, amount, cooking_servings };
    case 'prepared_meal':
      return { prepared_meal_id: component.prepared_meal_id, amount, cooking_servings };
    default:
      return null;
  }
}

export function toExistingGroupComponent(component: MealPlanComponent): NewGroupComponent | null {
  const base = toNewGroupComponent(component);
  return base ? { ...base, id: component.id } : null;
}

export function toNewGroup(group: GroupView): NewGroup {
  return {
    label: group.label,
    ad_hoc: group.ad_hoc,
    components: group.components
      .map(toNewGroupComponent)
      .filter((component): component is NewGroupComponent => component !== null),
    everyone: group.everyone,
    participants: group.everyone
      ? undefined
      : group.participants.map((participant) => ({ member_id: participant.member_id, note: participant.note })),
  };
}
