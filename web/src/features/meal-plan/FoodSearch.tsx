import Autocomplete from '@mui/material/Autocomplete';
import TextField from '@mui/material/TextField';
import { useMemo, useState } from 'react';
import type { Product, RecipeSummary } from '../../api/client';
import { useProducts, useRecipes } from '../../api/queries';
import { useDebounced } from '../../hooks/useDebounced';

export type FoodChoice =
  | { kind: 'product'; product: Product }
  | { kind: 'recipe'; recipe: RecipeSummary };

type Option = {
  id: string;
  name: string;
  group: 'Products' | 'Recipes';
  choice: FoodChoice;
};

export function FoodSearch({
  onPick,
  excludeProductIds,
  excludeRecipeIds,
  autoFocus,
}: {
  onPick: (choice: FoodChoice) => void;
  excludeProductIds?: string[];
  excludeRecipeIds?: string[];
  autoFocus?: boolean;
}) {
  const [input, setInput] = useState('');
  const debounced = useDebounced(input, 300);
  const products = useProducts({ q: debounced || undefined, per_page: 10 });
  const recipes = useRecipes({ q: debounced || undefined, per_page: 10 });

  const options = useMemo<Option[]>(() => {
    const skipProducts = new Set(excludeProductIds ?? []);
    const skipRecipes = new Set(excludeRecipeIds ?? []);
    const productOptions: Option[] = (products.data?.items ?? [])
      .filter((product) => !skipProducts.has(product.id))
      .map((product) => ({ id: `product:${product.id}`, name: product.name, group: 'Products', choice: { kind: 'product', product } }));
    const recipeOptions: Option[] = (recipes.data?.items ?? [])
      .filter((recipe) => !skipRecipes.has(recipe.id))
      .map((recipe) => ({ id: `recipe:${recipe.id}`, name: recipe.name, group: 'Recipes', choice: { kind: 'recipe', recipe } }));
    return [...productOptions, ...recipeOptions];
  }, [products.data, recipes.data, excludeProductIds, excludeRecipeIds]);

  const loading = products.isLoading || recipes.isLoading;

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
      noOptionsText={input ? 'Nothing matched' : 'Type to search food and recipes'}
      autoHighlight
      blurOnSelect
      clearOnBlur
      renderInput={(params) => (
        <TextField {...params} placeholder="Search food or recipes" autoFocus={autoFocus} />
      )}
    />
  );
}
