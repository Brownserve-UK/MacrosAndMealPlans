import { useMemo } from 'react';
import type { MealTemplate, MealTemplateComponent } from '../../../api/client';
import { useMealTemplates, useProducts, useRecipes, useStock } from '../../../api/queries';
import { useDebounced } from '../../../hooks/useDebounced';
import { formatMinutes } from './plannerWeek';
import type { NewGroupComponent, PickerRow } from './types';

export type FridgeDish = { recipeId: string; name: string; servings: number; useBy: string | null };

export function templateComponents(template: MealTemplate): NewGroupComponent[] {
  return template.components
    .map((component) => refToComponent(component))
    .filter((component): component is NewGroupComponent => component !== null);
}

function refToComponent(component: MealTemplateComponent): NewGroupComponent | null {
  const amount = component.amount;
  switch (component.item_kind) {
    case 'product':
      return { product_id: component.product_id, amount };
    case 'recipe':
      return { recipe_id: component.recipe_id, amount };
    case 'dish':
      return { dish_recipe_id: component.dish_recipe_id, amount };
    case 'ingredient':
      return { ingredient_id: component.ingredient_id, amount };
    case 'prepared_meal':
      return { prepared_meal_id: component.prepared_meal_id, amount };
    default:
      return null;
  }
}

export function servingsInFridge(dish: FridgeDish): string {
  return dish.servings === 1 ? '1 serving in the fridge' : `${dish.servings} servings in the fridge`;
}

export function leftoversRow(dish: FridgeDish, servings = dish.servings): PickerRow {
  return {
    id: `leftovers:${dish.recipeId}`,
    title: 'Leftovers',
    caption: `${dish.name}, ${servingsInFridge(dish)}`,
    concept: 'dish',
    section: 'quick',
    group: {
      components: [{ dish_recipe_id: dish.recipeId, amount: { kind: 'servings', value: servings } }],
    },
  };
}

export function useFridgeDishes(): FridgeDish[] {
  const stock = useStock({ per_page: 200 });
  return useMemo(() => {
    const byRecipe = new Map<string, FridgeDish>();
    for (const item of stock.data?.items ?? []) {
      if (item.subject_kind !== 'prepared_portion' || !item.prepared_recipe_id) continue;
      const servings = 'quantity' in item.level ? item.level.quantity.amount : 0;
      if (servings <= 0) continue;
      const useBy = item.usability_deadline?.date ?? null;
      const found = byRecipe.get(item.prepared_recipe_id);
      if (found) {
        found.servings += servings;
        if (useBy && (!found.useBy || useBy < found.useBy)) found.useBy = useBy;
      } else {
        byRecipe.set(item.prepared_recipe_id, {
          recipeId: item.prepared_recipe_id,
          name: item.prepared_batch_name ?? 'Cooked food',
          servings,
          useBy,
        });
      }
    }
    return [...byRecipe.values()].sort((a, b) => (a.useBy ?? '9999').localeCompare(b.useBy ?? '9999'));
  }, [stock.data]);
}

export function usePickerRows(query: string): { rows: PickerRow[]; loading: boolean } {
  const debounced = useDebounced(query.trim(), 250);
  const recipes = useRecipes({ q: debounced || undefined, per_page: 6 });
  const savedMeals = useMealTemplates({ q: debounced || undefined, per_page: 6 });
  const products = useProducts({ q: debounced || undefined, per_page: 6 });
  const dishes = useFridgeDishes();

  const rows = useMemo<PickerRow[]>(() => {
    const matches: PickerRow[] = [];
    if (debounced !== '') {
      for (const recipe of recipes.data?.items ?? []) {
        const minutes = (recipe.preparation_minutes ?? 0) + (recipe.cooking_minutes ?? 0);
        matches.push({
          id: `recipe:${recipe.id}`,
          title: recipe.name,
          caption: minutes > 0 ? `Recipe · ${formatMinutes(minutes)}` : 'Recipe',
          concept: 'recipe',
          section: 'matches',
          group: { components: [{ recipe_id: recipe.id, amount: { kind: 'servings', value: recipe.servings } }] },
        });
      }
      for (const template of savedMeals.data?.items ?? []) {
        matches.push({
          id: `saved:${template.id}`,
          title: template.name,
          caption: 'Saved meal',
          concept: 'saved_meal',
          section: 'matches',
          group: { label: template.name, components: templateComponents(template) },
        });
      }
      for (const product of products.data?.items ?? []) {
        matches.push({
          id: `product:${product.id}`,
          title: product.name,
          caption: product.brand ? `Product · ${product.brand}` : 'Product',
          concept: 'food',
          section: 'matches',
          group: { components: [{ product_id: product.id, amount: { kind: 'packs', value: 1 } }] },
        });
      }
    }
    const quick: PickerRow[] = [
      ...dishes.slice(0, 3).map((dish) => leftoversRow(dish)),
      {
        id: 'quick:eating-out',
        title: 'Eating out',
        caption: null,
        concept: 'out',
        section: 'quick',
        group: { label: 'Eating out', ad_hoc: 'eating_out' },
      },
      {
        id: 'quick:takeaway',
        title: 'Takeaway',
        caption: null,
        concept: 'takeaway',
        section: 'quick',
        group: { label: 'Takeaway', ad_hoc: 'takeaway' },
      },
      {
        id: 'quick:fend',
        title: 'Fend for yourself',
        caption: null,
        concept: 'fend',
        section: 'quick',
        group: { label: 'Fend for yourself', ad_hoc: 'fend_for_yourself' },
      },
    ];
    return [...matches, ...quick];
  }, [debounced, recipes.data, savedMeals.data, products.data, dishes]);

  return { rows, loading: recipes.isLoading || savedMeals.isLoading || products.isLoading };
}
