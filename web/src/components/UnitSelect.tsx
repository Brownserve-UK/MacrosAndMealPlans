import Autocomplete from '@mui/material/Autocomplete';
import TextField from '@mui/material/TextField';
import type { Unit, UnitInfo } from '../api/client';
import { useUnits } from '../api/queries';

const GROUP_ORDER: Record<string, number> = { mass: 0, volume: 1, count: 2 };
const GROUP_LABELS: Record<string, string> = { mass: 'Weight', volume: 'Volume', count: 'Count' };

export function displayUnit(code: string): string {
  return code.replace(/_/g, ' ');
}

export function UnitSelect({
  label,
  value,
  onChange,
  sx,
  error,
  helperText,
}: {
  label: string;
  value: Unit | '';
  onChange: (next: Unit) => void;
  sx?: object;
  error?: boolean;
  helperText?: string;
}) {
  const units = useUnits();
  const options = [...(units.data ?? [])].sort(
    (a, b) => (GROUP_ORDER[a.dimension] ?? 99) - (GROUP_ORDER[b.dimension] ?? 99),
  );
  const selected = options.find((u) => u.code === value) ?? null;

  return (
    <Autocomplete<UnitInfo>
      options={options}
      value={selected}
      onChange={(_, next) => {
        if (next) onChange(next.code as Unit);
      }}
      groupBy={(option) => GROUP_LABELS[option.dimension] ?? option.dimension}
      getOptionLabel={(option) => `${option.label} (${displayUnit(option.code)})`}
      getOptionKey={(option) => option.code}
      isOptionEqualToValue={(option, val) => option.code === val.code}
      fullWidth
      sx={sx}
      slotProps={{ listbox: { sx: { maxHeight: 280 } } }}
      renderInput={(params) => (
        <TextField {...params} label={label} error={error} helperText={helperText} />
      )}
    />
  );
}
