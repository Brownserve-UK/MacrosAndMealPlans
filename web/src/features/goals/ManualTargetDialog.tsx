import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState, type FormEvent } from 'react';
import { ApiError, type NutritionTarget } from '../../api/client';
import { useSetManualCalorieTarget } from '../../api/queries';
import { ConflictDialog } from '../../components/ConflictDialog';
import { FormDialog } from '../../components/FormDialog';

export function ManualTargetDialog({
  memberId,
  floorKcal,
  target,
  onClose,
}: {
  memberId: string;
  floorKcal?: number | null;
  target?: NutritionTarget | null;
  onClose: () => void;
}) {
  const save = useSetManualCalorieTarget();
  const [values, setValues] = useState({
    energy_kcal: target?.energy_kcal == null ? '' : String(target.energy_kcal),
    protein_g: target?.protein_g == null ? '' : String(target.protein_g),
    carbohydrate_g: target?.carbohydrate_g == null ? '' : String(target.carbohydrate_g),
    fat_g: target?.fat_g == null ? '' : String(target.fat_g),
  });
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [conflict, setConflict] = useState<ApiError | null>(null);
  const parsed = Number(values.energy_kcal);
  const belowFloor = Number.isFinite(parsed) && parsed > 0 && floorKcal != null && parsed < floorKcal;

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    const next: Record<string, string> = {};
    for (const [field, label] of [
      ['energy_kcal', 'Enter a daily calorie target'],
      ['protein_g', 'Enter a protein target'],
      ['carbohydrate_g', 'Enter a carbohydrate target'],
      ['fat_g', 'Enter a fat target'],
    ] as const) {
      if (!Number.isFinite(Number(values[field])) || Number(values[field]) <= 0) next[field] = label;
    }
    setErrors(next);
    setFormError(null);
    if (Object.keys(next).length > 0) return;

    try {
      await save.mutateAsync({
        id: memberId,
        body: {
          energy_kcal: parsed,
          protein_g: Number(values.protein_g),
          carbohydrate_g: Number(values.carbohydrate_g),
          fat_g: Number(values.fat_g),
        },
      });
      onClose();
    } catch (caught) {
      if (caught instanceof ApiError) {
        if (caught.isConflict) setConflict(caught);
        else setFormError(Object.values(caught.fieldErrors)[0] ?? caught.message);
      } else {
        setFormError('Something went wrong.');
      }
    }
  }

  return (
    <FormDialog open onClose={onClose} fullWidth maxWidth="xs">
      <form onSubmit={onSubmit}>
        <DialogTitle>Set your own target</DialogTitle>
        <DialogContent dividers>
          <Stack spacing={2.5}>
            <Typography variant="body2" color="text.secondary">
              Use a target from a nutrition professional or one you already know.
            </Typography>
            {([
              ['energy_kcal', 'Daily calories', 'kcal'],
              ['protein_g', 'Protein', 'g'],
              ['carbohydrate_g', 'Carbs', 'g'],
              ['fat_g', 'Fat', 'g'],
            ] as const).map(([field, label, unit], index) => (
              <TextField
                key={field}
                autoFocus={index === 0}
                label={label}
                value={values[field]}
                onChange={(event) => setValues((current) => ({ ...current, [field]: event.target.value }))}
                error={Boolean(errors[field])}
                helperText={errors[field]}
                inputMode="decimal"
                slotProps={{ input: { endAdornment: <Typography variant="caption" color="text.secondary">{unit}</Typography> } }}
                fullWidth
              />
            ))}
            {belowFloor ? (
              <Alert severity="warning">
                This is below the usual safety floor of{' '}
                <span className="numeral">{floorKcal?.toLocaleString('en-GB')} kcal</span>.
              </Alert>
            ) : null}
            {formError ? <Alert severity="error">{formError}</Alert> : null}
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={onClose}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={save.isPending}>
            {save.isPending ? 'Saving…' : 'Use these targets'}
          </Button>
        </DialogActions>
      </form>
      <ConflictDialog
        error={conflict}
        onReload={() => {
          setConflict(null);
          onClose();
        }}
        onDismiss={() => setConflict(null)}
      />
    </FormDialog>
  );
}
