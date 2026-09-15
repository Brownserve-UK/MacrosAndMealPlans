import Autocomplete from '@mui/material/Autocomplete';
import TextField from '@mui/material/TextField';
import { useState } from 'react';
import type { RecipeSummary } from '../../api/client';
import { useRecipes } from '../../api/queries';
import { useDebounced } from '../../hooks/useDebounced';

export function RecipePicker({
  value,
  onChange,
  disabled,
  autoFocus,
  error,
  helperText,
}: {
  value: RecipeSummary | null;
  onChange: (next: RecipeSummary | null) => void;
  disabled?: boolean;
  autoFocus?: boolean;
  error?: boolean;
  helperText?: string;
}) {
  const [input, setInput] = useState('');
  const debounced = useDebounced(input, 300);
  const query = useRecipes({ q: debounced || undefined, per_page: 20 });
  const options = query.data?.items ?? [];

  return (
    <Autocomplete<RecipeSummary>
      value={value}
      onChange={(_, next) => onChange(next)}
      inputValue={input}
      onInputChange={(_, next) => setInput(next)}
      options={options}
      getOptionLabel={(option) => option.name}
      isOptionEqualToValue={(a, b) => a.id === b.id}
      loading={query.isLoading}
      loadingText="Loading recipes"
      noOptionsText={input ? 'No recipes found' : 'No recipes available'}
      autoHighlight
      disabled={disabled}
      renderInput={(params) => (
        <TextField
          {...params}
          label="Recipe"
          placeholder="Search recipes"
          autoFocus={autoFocus}
          error={error}
          helperText={helperText}
        />
      )}
    />
  );
}
