import { QueryClient } from '@tanstack/react-query';
import { describe, expect, it } from 'vitest';
import {
  catalogueKeys,
  householdKeys,
  mealPlanKeys,
  nutritionTargetKeys,
  recipeKeys,
  settingsKeys,
  shoppingKeys,
  stockKeys,
  weightKeys,
} from './keys';

type Key = readonly unknown[];

function invalidatedBy(prefix: Key, cached: Record<string, Key>): string[] {
  const qc = new QueryClient();
  for (const key of Object.values(cached)) qc.setQueryData(key, 'cached');
  void qc.invalidateQueries({ queryKey: prefix });
  return Object.entries(cached)
    .filter(([, key]) => qc.getQueryState(key)?.isInvalidated)
    .map(([name]) => name)
    .sort();
}

describe('query key shapes', () => {
  it('keeps the wire keys the cache was built on', () => {
    expect(settingsKeys.units()).toEqual(['units']);
    expect(settingsKeys.mealTimes()).toEqual(['mealTimes']);
    expect(catalogueKeys.ingredientList({ q: 'oat' })).toEqual(['ingredients', { q: 'oat' }]);
    expect(catalogueKeys.ingredientProducts('i1')).toEqual(['ingredient', 'i1', 'products']);
    expect(householdKeys.memberAccess('m1')).toEqual(['member', 'm1', 'access']);
    expect(mealPlanKeys.myWeek('2026-09-07')).toEqual(['mealPlanWeek', '2026-09-07']);
    expect(mealPlanKeys.householdWeek('2026-09-07')).toEqual(['plannerWeek', '2026-09-07']);
    expect(mealPlanKeys.plannerWeek('2026-09-07')).toEqual(['planner', '2026-09-07']);
    expect(recipeKeys.photo('r1', 'hero', 3)).toEqual(['recipe', 'r1', 'photo', 'hero', 3]);
    expect(stockKeys.availability('p1')).toEqual(['stock', 'availability', 'p1', null]);
    expect(shoppingKeys.list('2026-09-07')).toEqual(['shopping', 'requirements', '2026-09-07']);
    expect(weightKeys.summary('m1')).toEqual(['weight', 'summary', 'm1']);
  });

  it('gives an absent argument a stable slot so two calls agree', () => {
    expect(stockKeys.availability()).toEqual(['stock', 'availability', undefined, null]);
    expect(shoppingKeys.purchases()).toEqual(['shopping', 'purchases', undefined]);
  });

  it('keeps a demand window distinct from the unbounded query', () => {
    const bounded = stockKeys.availability(undefined, { from: '2026-09-20', to: '2026-09-26' });
    expect(bounded).toEqual(['stock', 'availability', undefined, { from: '2026-09-20', to: '2026-09-26' }]);
    expect(bounded).not.toEqual(stockKeys.availability());
  });

  it('starts every key with the prefix its family is invalidated by', () => {
    const families: [Key, Key[]][] = [
      [catalogueKeys.ingredients(), [catalogueKeys.ingredientList({})]],
      [
        catalogueKeys.ingredient(),
        [catalogueKeys.ingredientDetail('i1'), catalogueKeys.ingredientProducts('i1')],
      ],
      [catalogueKeys.products(), [catalogueKeys.productList({})]],
      [catalogueKeys.product(), [catalogueKeys.productDetail('p1')]],
      [householdKeys.members(), [householdKeys.memberList({})]],
      [
        householdKeys.member(),
        [householdKeys.memberDetail('m1'), householdKeys.memberAccess('m1')],
      ],
      [householdKeys.users(), [householdKeys.userList({})]],
      [householdKeys.user(), [householdKeys.userDetail('u1')]],
      [mealPlanKeys.myWeeks(), [mealPlanKeys.myWeek('2026-09-07')]],
      [mealPlanKeys.householdWeeks(), [mealPlanKeys.householdWeek('2026-09-07')]],
      [mealPlanKeys.plannerWeeks(), [mealPlanKeys.plannerWeek('2026-09-07')]],
      [mealPlanKeys.entries(), [mealPlanKeys.entry('e1')]],
      [nutritionTargetKeys.all(), [nutritionTargetKeys.forMember('m1')]],
      [weightKeys.all(), [weightKeys.summary('m1'), weightKeys.records('m1'), weightKeys.goal('m1')]],
      [recipeKeys.recipes(), [recipeKeys.recipeList({})]],
      [
        recipeKeys.recipe(),
        [recipeKeys.recipeDetail('r1'), recipeKeys.nutrition('r1'), recipeKeys.photo('r1', 'card', 0)],
      ],
      [
        stockKeys.all(),
        [stockKeys.list({}), stockKeys.item('s1'), stockKeys.events('s1'), stockKeys.availability()],
      ],
      [
        shoppingKeys.all(),
        [
          shoppingKeys.list(),
          shoppingKeys.opportunities(),
          shoppingKeys.cadence(),
          shoppingKeys.purchases(),
        ],
      ],
    ];

    for (const [prefix, members] of families) {
      for (const key of members) {
        expect(key.slice(0, prefix.length)).toEqual([...prefix]);
      }
    }
  });
});

describe('invalidation targets', () => {
  const mealPlanCache = {
    myWeek: mealPlanKeys.myWeek('2026-09-07'),
    householdWeek: mealPlanKeys.householdWeek('2026-09-07'),
    needsReview: mealPlanKeys.needsReview(),
    plannerWeek: mealPlanKeys.plannerWeek('2026-09-07'),
    unrelated: stockKeys.list({}),
  };

  it('separates the planner weeks', () => {
    expect(invalidatedBy(mealPlanKeys.myWeeks(), mealPlanCache)).toEqual(['myWeek']);
    expect(invalidatedBy(mealPlanKeys.householdWeeks(), mealPlanCache)).toEqual(['householdWeek']);
    expect(invalidatedBy(mealPlanKeys.plannerWeeks(), mealPlanCache)).toEqual(['plannerWeek']);
  });

  it('leaves the rest of the cache alone', () => {
    expect(invalidatedBy(mealPlanKeys.needsReview(), mealPlanCache)).toEqual(['needsReview']);
  });

  it('clears every list a catalogue edit can change', () => {
    const cache = {
      ingredientList: catalogueKeys.ingredientList({ q: 'oat' }),
      otherIngredientList: catalogueKeys.ingredientList({ include_archived: true }),
      ingredientDetail: catalogueKeys.ingredientDetail('i1'),
      productList: catalogueKeys.productList({}),
    };
    expect(invalidatedBy(catalogueKeys.ingredients(), cache)).toEqual([
      'ingredientList',
      'otherIngredientList',
    ]);
    expect(invalidatedBy(catalogueKeys.ingredient(), cache)).toEqual(['ingredientDetail']);
  });

  it('empties the cupboard views together, since a purchase changes both', () => {
    const cache = {
      stockList: stockKeys.list({}),
      stockItem: stockKeys.item('s1'),
      stockEvents: stockKeys.events('s1'),
      availability: stockKeys.availability('p1'),
      shoppingList: shoppingKeys.list(),
      purchases: shoppingKeys.purchases('pending'),
    };
    expect(invalidatedBy(stockKeys.all(), cache)).toEqual([
      'availability',
      'stockEvents',
      'stockItem',
      'stockList',
    ]);
    expect(invalidatedBy(shoppingKeys.all(), cache)).toEqual(['purchases', 'shoppingList']);
  });

  it('scopes a weigh-in to the member it belongs to', () => {
    const cache = {
      mine: weightKeys.summary('m1'),
      myRecords: weightKeys.records('m1'),
      theirs: weightKeys.summary('m2'),
    };
    expect(invalidatedBy(weightKeys.summary('m1'), cache)).toEqual(['mine']);
    expect(invalidatedBy(weightKeys.all(), cache)).toEqual(['mine', 'myRecords', 'theirs']);
  });
});
