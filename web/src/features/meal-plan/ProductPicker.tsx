import Autocomplete from '@mui/material/Autocomplete';
import TextField from '@mui/material/TextField';
import { useState } from 'react';
import type { Product } from '../../api/client';
import { useProducts } from '../../api/queries';
import { useDebounced } from '../../hooks/useDebounced';

export function ProductPicker({
  value,
  onChange,
  disabled,
  autoFocus,
  error,
  helperText,
  excludeIds,
  unmappedOnly,
}: {
  value: Product | null;
  onChange: (next: Product | null) => void;
  disabled?: boolean;
  autoFocus?: boolean;
  error?: boolean;
  helperText?: string;
  excludeIds?: string[];
  unmappedOnly?: boolean;
}) {
  const [input, setInput] = useState('');
  const debounced = useDebounced(input, 300);
  const query = useProducts({ q: debounced || undefined, per_page: 20, unmapped: unmappedOnly });
  const excluded = new Set(excludeIds ?? []);
  const options = (query.data?.items ?? []).filter(
    (product) => product.id === value?.id || !excluded.has(product.id),
  );

  return (
    <Autocomplete<Product>
      value={value}
      onChange={(_, next) => onChange(next)}
      inputValue={input}
      onInputChange={(_, next) => setInput(next)}
      options={options}
      getOptionLabel={(option) => option.name}
      isOptionEqualToValue={(a, b) => a.id === b.id}
      loading={query.isLoading}
      loadingText="Loading products"
      noOptionsText={input ? 'No products found' : 'No products available'}
      autoHighlight
      disabled={disabled}
      renderInput={(params) => (
        <TextField
          {...params}
          label="Product"
          placeholder="Search products"
          autoFocus={autoFocus}
          error={error}
          helperText={helperText}
        />
      )}
    />
  );
}
