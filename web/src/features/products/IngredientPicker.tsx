import Autocomplete from '@mui/material/Autocomplete';
import TextField from '@mui/material/TextField';
import { useState } from 'react';
import type { Ingredient } from '../../api/client';
import { useIngredients } from '../../api/queries';
import { useDebounced } from '../../hooks/useDebounced';

export function IngredientPicker({
  value,
  onChange,
  disabled,
}: {
  value: Ingredient | null;
  onChange: (next: Ingredient | null) => void;
  disabled?: boolean;
}) {
  const [input, setInput] = useState('');
  const debounced = useDebounced(input, 300);
  const query = useIngredients({ q: debounced || undefined, per_page: 20 });

  return (
    <Autocomplete<Ingredient>
      value={value}
      onChange={(_, next) => onChange(next)}
      inputValue={input}
      onInputChange={(_, next) => setInput(next)}
      options={query.data?.items ?? []}
      getOptionLabel={(option) => option.name}
      isOptionEqualToValue={(a, b) => a.id === b.id}
      loading={query.isLoading}
      disabled={disabled}
      renderInput={(params) => (
        <TextField
          {...params}
          label="Ingredient"
          placeholder="Search"
          helperText="Optional."
        />
      )}
    />
  );
}
