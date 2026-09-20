import { describe, expect, it } from 'vitest';
import { group, occasion, MEMBERS } from './fixtures';
import { cellDetail, groupDisplayName, groupFoodCaption, groupShortName, joinFoodNames } from './plannerWeek';
import type { GroupView } from './types';

describe('joinFoodNames', () => {
  it('is just the name for one food', () => {
    expect(joinFoodNames(['Salmon'])).toEqual({ head: 'Salmon', tail: null });
  });

  it('joins two foods with an ampersand', () => {
    expect(joinFoodNames(['Salmon', 'Potatoes'])).toEqual({ head: 'Salmon', tail: '& Potatoes' });
  });

  it('joins three foods with a comma then an ampersand', () => {
    expect(joinFoodNames(['Salmon', 'Potatoes', 'Broccoli'])).toEqual({
      head: 'Salmon, Potatoes',
      tail: '& Broccoli',
    });
  });

  it('collapses four or more foods into a plus count', () => {
    expect(joinFoodNames(['Chilli', 'Rice', 'Garlic bread', 'Sour cream'])).toEqual({
      head: 'Chilli, Rice',
      tail: '+2',
    });
  });
});

function component(item_name: string): GroupView['components'][number] {
  return {
    id: `component-${item_name}`,
    item_kind: 'product',
    product_id: `product-${item_name}`,
    item_name,
    amount: { kind: 'servings', value: 1 },
    nutrition: {},
    quality: 'known',
    preparation: { prepared: { kind: 'servings', value: '1' }, shortage: false },
    status: 'planned',
    subject_status: 'planned',
    position: 0,
    effective_cooking_servings: 1,
    revision: 1,
    needs_cooking: false,
  };
}

function baseGroup(overrides: Partial<GroupView> = {}): GroupView {
  return {
    id: 'group-1',
    name: 'Salmon fillets, potatoes & broccoli',
    label: null,
    ad_hoc: null,
    components: [component('Salmon'), component('Potatoes'), component('Broccoli')],
    everyone: true,
    participants: [],
    guest_count: 0,
    guests: [],
    serves: 1,
    cook_minutes: null,
    to_buy: 0,
    leftover_servings_available: null,
    revision: 1,
    ...overrides,
  };
}

describe('groupDisplayName', () => {
  it('joins the food names when there is no typed label', () => {
    expect(groupDisplayName(baseGroup())).toEqual({ head: 'Salmon, Potatoes', tail: '& Broccoli' });
  });

  it('always wins with a typed label, hiding the food', () => {
    expect(groupDisplayName(baseGroup({ label: 'Fish finger sandwich' }))).toEqual({
      head: 'Fish finger sandwich',
      tail: null,
    });
  });

  it('uses the ad hoc kind label with no tail', () => {
    expect(groupDisplayName(baseGroup({ ad_hoc: 'eating_out', name: 'Eating out' }))).toEqual({
      head: 'Eating out',
      tail: null,
    });
  });
});

describe('groupShortName', () => {
  it('keeps a row to the first food and a count', () => {
    expect(groupShortName(baseGroup())).toEqual({ head: 'Salmon', tail: '+2' });
  });

  it('has no tail for a single food', () => {
    expect(groupShortName(baseGroup({ components: [baseGroup().components[0]!] }))).toEqual({
      head: 'Salmon',
      tail: null,
    });
  });

  it('still lets a typed label win', () => {
    expect(groupShortName(baseGroup({ label: 'Fish finger sandwich' }))).toEqual({
      head: 'Fish finger sandwich',
      tail: null,
    });
  });
});

describe('groupFoodCaption', () => {
  it('is null without a typed label', () => {
    expect(groupFoodCaption(baseGroup())).toBeNull();
  });

  it('surfaces the joined food as the caption once a label is typed', () => {
    expect(groupFoodCaption(baseGroup({ label: 'Fish finger sandwich' }))).toBe('Salmon, Potatoes & Broccoli');
  });
});

describe('cellDetail', () => {
  it('marks everyone as eating when there is one everyone group', () => {
    const detail = cellDetail(occasion(), MEMBERS);
    expect(detail.avatars).toEqual(
      MEMBERS.map((member) => ({
        memberId: member.id,
        name: member.name,
        initials: member.initials,
        state: 'eating',
      })),
    );
    expect(detail.markers).toEqual([
      { kind: 'cook', value: '35 mins', tone: 'brown' },
      { kind: 'buy', value: '2 to buy', tone: 'amber' },
    ]);
  });

  it('marks a member with no group and no absence as unaccounted', () => {
    const curry = occasion({
      groups: [
        group({
          everyone: false,
          participants: [
            { member_id: 'steve', name: 'Steve', note: null },
            { member_id: 'sarah', name: 'Sarah', note: null },
            { member_id: 'emily', name: 'Emily', note: null },
          ],
        }),
      ],
    });
    const detail = cellDetail(curry, MEMBERS);
    expect(detail.avatars.find((avatar) => avatar.memberId === 'jack')).toEqual({
      memberId: 'jack',
      name: 'Jack',
      initials: 'JB',
      state: 'unaccounted',
    });
  });

  it('marks an absent member as elsewhere', () => {
    const detail = cellDetail(occasion({ absent_member_ids: ['jack'] }), MEMBERS);
    expect(detail.avatars.find((avatar) => avatar.memberId === 'jack')).toEqual({
      memberId: 'jack',
      name: 'Jack',
      initials: 'JB',
      state: 'elsewhere',
    });
  });

  it('marks a member on a second group as separate', () => {
    const split = occasion({
      groups: [
        group({ id: 'bol', name: 'Spaghetti bolognese' }),
        group({
          id: 'lasagne',
          name: 'Lasagne',
          everyone: false,
          participants: [{ member_id: 'jack', name: 'Jack', note: null }],
        }),
      ],
    });
    const detail = cellDetail(split, MEMBERS);
    expect(detail.avatars.find((avatar) => avatar.memberId === 'jack')?.state).toBe('separate');
    expect(detail.avatars.find((avatar) => avatar.memberId === 'steve')?.state).toBe('eating');
    expect(detail.markers).toContainEqual({ kind: 'separate', value: '2 meals', tone: 'secondary' });
  });

  it('builds the full marker set from guests, cooking, shopping and absence', () => {
    const busy = occasion({
      groups: [
        group({ id: 'bol', name: 'Spaghetti bolognese', cook_minutes: 40, to_buy: 3, guest_count: 2 }),
        group({ id: 'lasagne', name: 'Lasagne', everyone: false, participants: [], guest_count: 0, to_buy: 0 }),
      ],
      absent_member_ids: ['jack'],
    });
    const detail = cellDetail(busy, MEMBERS);
    expect(detail.markers).toEqual([
      { kind: 'separate', value: '2 meals', tone: 'secondary' },
      { kind: 'cook', value: '40 mins', tone: 'brown' },
      { kind: 'guests', value: '2', tone: 'secondary' },
      { kind: 'buy', value: '3 to buy', tone: 'amber' },
      { kind: 'elsewhere', value: '1', tone: 'secondary' },
    ]);
  });
});
