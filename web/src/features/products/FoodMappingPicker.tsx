import Autocomplete from '@mui/material/Autocomplete';
import TextField from '@mui/material/TextField';
import { useState } from 'react';
import type { Ingredient, PreparedMeal } from '../../api/client';
import { useIngredients, usePreparedMeals } from '../../api/queries';
import { useDebounced } from '../../hooks/useDebounced';

export type FoodMapping =
  | { kind: 'ingredient'; ingredient: Ingredient }
  | { kind: 'prepared_meal'; preparedMeal: PreparedMeal };

type Option = { id: string; name: string; group: 'Ingredients' | 'Prepared meals'; mapping: FoodMapping };

export function FoodMappingPicker({
  value,
  onChange,
  disabled,
}: {
  value: FoodMapping | null;
  onChange: (next: FoodMapping | null) => void;
  disabled?: boolean;
}) {
  const [input, setInput] = useState('');
  const debounced = useDebounced(input, 300);
  const ingredients = useIngredients({ q: debounced || undefined, per_page: 20 });
  const preparedMeals = usePreparedMeals({ q: debounced || undefined, per_page: 20 });

  const options: Option[] = [
    ...(ingredients.data?.items ?? []).map((ingredient) => ({
      id: `ingredient:${ingredient.id}`,
      name: ingredient.name,
      group: 'Ingredients' as const,
      mapping: { kind: 'ingredient' as const, ingredient },
    })),
    ...(preparedMeals.data?.items ?? []).map((preparedMeal) => ({
      id: `prepared-meal:${preparedMeal.id}`,
      name: preparedMeal.name,
      group: 'Prepared meals' as const,
      mapping: { kind: 'prepared_meal' as const, preparedMeal },
    })),
  ];
  const selected =
    options.find((option) =>
      value?.kind === 'ingredient'
        ? option.mapping.kind === 'ingredient' && option.mapping.ingredient.id === value.ingredient.id
        : value?.kind === 'prepared_meal'
          ? option.mapping.kind === 'prepared_meal' && option.mapping.preparedMeal.id === value.preparedMeal.id
          : false,
    ) ?? null;

  return (
    <Autocomplete<Option>
      value={selected}
      onChange={(_, next) => onChange(next?.mapping ?? null)}
      inputValue={input}
      onInputChange={(_, next) => setInput(next)}
      options={options}
      groupBy={(option) => option.group}
      getOptionLabel={(option) => option.name}
      isOptionEqualToValue={(a, b) => a.id === b.id}
      loading={ingredients.isLoading || preparedMeals.isLoading}
      disabled={disabled}
      renderInput={(params) => (
        <TextField
          {...params}
          label="Stands in for"
          placeholder="Search ingredients and prepared meals"
          helperText="Optional. An ingredient or a prepared meal, never both."
        />
      )}
    />
  );
}
