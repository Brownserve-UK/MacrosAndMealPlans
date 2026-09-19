import Autocomplete from '@mui/material/Autocomplete';
import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useMemo, useState } from 'react';
import type { Ingredient, MealTemplate, PreparedMeal, Product, RecipeSummary } from '../../api/client';
import { useIngredients, useMealTemplates, usePreparedMeals, useProducts, useRecipes, useStock } from '../../api/queries';
import { KindChip, type Kind } from '../../components/KindChip';
import { useDebounced } from '../../hooks/useDebounced';
import { parseIsoDate } from './date';

export type Dish = {
  recipeId: string;
  name: string;
  servings: number;
  useBy: string | null;
  chilled: boolean;
};

export type FoodChoice =
  | { kind: 'product'; product: Product }
  | { kind: 'recipe'; recipe: RecipeSummary }
  | { kind: 'dish'; dish: Dish }
  | { kind: 'ingredient'; ingredient: Ingredient }
  | { kind: 'prepared_meal'; preparedMeal: PreparedMeal }
  | { kind: 'saved_meal'; savedMeal: MealTemplate };

type Option = {
  id: string;
  name: string;
  caption: string | null;
  chip: Kind;
  group: 'In the kitchen' | 'Saved meals' | 'Foods' | 'Products' | 'Recipes';
  choice: FoodChoice;
};

function dishCaption(dish: Dish): string {
  const amount = dish.servings === 1 ? '1 serving left' : `${dish.servings} servings left`;
  if (!dish.useBy) return amount;
  const when = parseIsoDate(dish.useBy).toLocaleDateString('en-GB', {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
  });
  return `${amount}, by ${when}`;
}

function foodCount(template: MealTemplate): string {
  const count = template.components.length;
  return count === 1 ? '1 food' : `${count} foods`;
}

export function FoodSearch({
  onPick,
  excludeProductIds,
  excludeRecipeIds,
  excludeDishIds,
  excludeIngredientIds,
  excludePreparedMealIds,
  hideSavedMeals,
  hideFoods,
  hideDishes,
  autoFocus,
}: {
  onPick: (choice: FoodChoice) => void;
  excludeProductIds?: string[];
  excludeRecipeIds?: string[];
  excludeDishIds?: string[];
  excludeIngredientIds?: string[];
  excludePreparedMealIds?: string[];
  hideSavedMeals?: boolean;
  hideFoods?: boolean;
  hideDishes?: boolean;
  autoFocus?: boolean;
}) {
  const [input, setInput] = useState('');
  const debounced = useDebounced(input, 300);
  const products = useProducts({ q: debounced || undefined, per_page: 10 });
  const recipes = useRecipes({ q: debounced || undefined, per_page: 10 });
  const ingredients = useIngredients({ q: debounced || undefined, per_page: 10 });
  const preparedMeals = usePreparedMeals({ q: debounced || undefined, per_page: 10 });
  const savedMeals = useMealTemplates({
    q: debounced || undefined,
    per_page: 10,
  });
  const stock = useStock({ per_page: 200 });

  const dishes = useMemo<Dish[]>(() => {
    const byRecipe = new Map<string, Dish>();
    for (const item of stock.data?.items ?? []) {
      if (item.subject_kind !== 'prepared_portion' || !item.prepared_recipe_id) continue;
      const servings = 'quantity' in item.level ? item.level.quantity.amount : 0;
      if (servings <= 0) continue;
      const useBy = item.usability_deadline?.date ?? null;
      const found = byRecipe.get(item.prepared_recipe_id);
      if (found) {
        found.servings += servings;
        found.chilled = found.chilled || item.storage_location === 'chilled';
        if (useBy && (!found.useBy || useBy < found.useBy)) found.useBy = useBy;
      } else {
        byRecipe.set(item.prepared_recipe_id, {
          recipeId: item.prepared_recipe_id,
          name: item.prepared_batch_name ?? 'Cooked food',
          servings,
          useBy,
          chilled: item.storage_location === 'chilled',
        });
      }
    }
    return [...byRecipe.values()].sort((a, b) => (a.useBy ?? '9999').localeCompare(b.useBy ?? '9999'));
  }, [stock.data]);

  const options = useMemo<Option[]>(() => {
    const skipProducts = new Set(excludeProductIds ?? []);
    const skipRecipes = new Set(excludeRecipeIds ?? []);
    const skipDishes = new Set(excludeDishIds ?? []);
    const skipIngredients = new Set(excludeIngredientIds ?? []);
    const skipPreparedMeals = new Set(excludePreparedMealIds ?? []);
    const needle = debounced.trim().toLowerCase();

    const dishOptions: Option[] = hideDishes
      ? []
      : dishes
      .filter((dish) => !skipDishes.has(dish.recipeId))
      .filter((dish) => !needle || dish.name.toLowerCase().includes(needle))
      .map((dish) => ({
        id: `dish:${dish.recipeId}`,
        name: dish.name,
        caption: dishCaption(dish),
        chip: dish.chilled ? ('fridge' as const) : ('freezer' as const),
        group: 'In the kitchen' as const,
        choice: { kind: 'dish' as const, dish },
      }));
    const savedMealOptions: Option[] = hideSavedMeals
      ? []
      : (savedMeals.data?.items ?? []).map((template) => ({
          id: `saved-meal:${template.id}`,
          name: template.name,
          caption: foodCount(template),
          chip: 'saved_meal' as const,
          group: 'Saved meals' as const,
          choice: { kind: 'saved_meal' as const, savedMeal: template },
        }));
    const ingredientOptions: Option[] = hideFoods
      ? []
      : (ingredients.data?.items ?? [])
          .filter((ingredient) => !skipIngredients.has(ingredient.id))
          .map((ingredient) => ({
            id: `ingredient:${ingredient.id}`,
            name: ingredient.name,
            caption: null,
            chip: 'food' as const,
            group: 'Foods' as const,
            choice: { kind: 'ingredient' as const, ingredient },
          }));
    const preparedMealOptions: Option[] = hideFoods
      ? []
      : (preparedMeals.data?.items ?? [])
          .filter((preparedMeal) => !skipPreparedMeals.has(preparedMeal.id))
          .map((preparedMeal) => ({
            id: `prepared-meal:${preparedMeal.id}`,
            name: preparedMeal.name,
            caption: null,
            chip: 'food' as const,
            group: 'Foods' as const,
            choice: { kind: 'prepared_meal' as const, preparedMeal },
          }));
    const productOptions: Option[] = (products.data?.items ?? [])
      .filter((product) => !skipProducts.has(product.id))
      .map((product) => ({
        id: `product:${product.id}`,
        name: product.name,
        caption: product.brand ?? null,
        chip: 'product' as const,
        group: 'Products' as const,
        choice: { kind: 'product' as const, product },
      }));
    const recipeOptions: Option[] = (recipes.data?.items ?? [])
      .filter((recipe) => !skipRecipes.has(recipe.id))
      .map((recipe) => ({
        id: `recipe:${recipe.id}`,
        name: recipe.name,
        caption: `Serves ${recipe.servings}`,
        chip: 'recipe' as const,
        group: 'Recipes' as const,
        choice: { kind: 'recipe' as const, recipe },
      }));
    return [
      ...dishOptions,
      ...savedMealOptions,
      ...ingredientOptions,
      ...preparedMealOptions,
      ...productOptions,
      ...recipeOptions,
    ];
  }, [
    dishes,
    savedMeals.data,
    hideSavedMeals,
    hideFoods,
    hideDishes,
    ingredients.data,
    preparedMeals.data,
    products.data,
    recipes.data,
    debounced,
    excludeProductIds,
    excludeRecipeIds,
    excludeDishIds,
    excludeIngredientIds,
    excludePreparedMealIds,
  ]);

  const loading =
    products.isLoading || recipes.isLoading || ingredients.isLoading || preparedMeals.isLoading;

  return (
    <Autocomplete<Option>
      value={null}
      onChange={(_, option) => {
        if (option) onPick(option.choice);
        setInput('');
      }}
      inputValue={input}
      onInputChange={(_, next) => setInput(next)}
      options={options}
      groupBy={(option) => option.group}
      getOptionLabel={(option) => option.name}
      isOptionEqualToValue={(a, b) => a.id === b.id}
      filterOptions={(all) => all}
      loading={loading}
      loadingText="Searching"
      noOptionsText={input ? 'Nothing matched' : 'Type to search food, recipes and saved meals'}
      autoHighlight
      blurOnSelect
      clearOnBlur
      renderOption={(props, option) => {
        const { key, ...rest } = props as typeof props & { key: string };
        return (
          <Box component="li" key={key} {...rest}>
            <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center', width: '100%' }}>
              <Stack sx={{ flexGrow: 1, minWidth: 0 }}>
                <Typography variant="body2">{option.name}</Typography>
                {option.caption ? (
                  <Typography variant="caption" color="text.secondary" className="numeral">
                    {option.caption}
                  </Typography>
                ) : null}
              </Stack>
              <KindChip kind={option.chip} />
            </Stack>
          </Box>
        );
      }}
      renderInput={(params) => (
        <TextField {...params} placeholder="Search food, recipes and saved meals" autoFocus={autoFocus} />
      )}
    />
  );
}
