import FormControlLabel from '@mui/material/FormControlLabel';
import FormHelperText from '@mui/material/FormHelperText';
import Stack from '@mui/material/Stack';
import Switch from '@mui/material/Switch';
import TextField from '@mui/material/TextField';
import type { Unit } from '../../api/client';
import { UnitSelect } from '../../components/UnitSelect';

export type IngredientDraft = { name: string; default_unit: Unit; track_stock: boolean };

export function IngredientFields({
  draft,
  errors,
  onChange,
  autoFocus,
}: {
  draft: IngredientDraft;
  errors: Record<string, string>;
  onChange: (next: IngredientDraft) => void;
  autoFocus?: boolean;
}) {
  return (
    <Stack spacing={2.5}>
      <TextField
        label="Name"
        value={draft.name}
        onChange={(e) => onChange({ ...draft, name: e.target.value })}
        error={Boolean(errors.name)}
        helperText={errors.name}
        autoFocus={autoFocus}
        fullWidth
      />
      <UnitSelect
        label="Measured in"
        value={draft.default_unit}
        onChange={(next) => onChange({ ...draft, default_unit: next })}
      />
      <div>
        <FormControlLabel
          control={
            <Switch
              checked={draft.track_stock}
              onChange={(_, checked) => onChange({ ...draft, track_stock: checked })}
            />
          }
          label="Track stock"
        />
        <FormHelperText>Off means we assume you always have some.</FormHelperText>
      </div>
    </Stack>
  );
}
