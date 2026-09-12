import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState, type FormEvent } from 'react';
import { ApiError } from '../../api/client';
import { useSetManualCalorieTarget } from '../../api/queries';
import { ConflictDialog } from '../../components/ConflictDialog';
import { FormDialog } from '../../components/FormDialog';

export function ManualTargetDialog({
  memberId,
  floorKcal,
  onClose,
}: {
  memberId: string;
  floorKcal?: number | null;
  onClose: () => void;
}) {
  const save = useSetManualCalorieTarget();
  const [energyKcal, setEnergyKcal] = useState('');
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [conflict, setConflict] = useState<ApiError | null>(null);
  const parsed = Number(energyKcal);
  const belowFloor = Number.isFinite(parsed) && parsed > 0 && floorKcal != null && parsed < floorKcal;

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    const next: Record<string, string> = {};
    if (!Number.isFinite(parsed) || parsed <= 0) next.energy_kcal = 'Enter a daily calorie target';
    setErrors(next);
    setFormError(null);
    if (Object.keys(next).length > 0) return;

    try {
      await save.mutateAsync({ id: memberId, body: { energy_kcal: parsed } });
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
            <TextField
              autoFocus
              label="Daily calories"
              value={energyKcal}
              onChange={(event) => setEnergyKcal(event.target.value)}
              error={Boolean(errors.energy_kcal)}
              helperText={errors.energy_kcal}
              inputMode="numeric"
              slotProps={{
                input: {
                  endAdornment: (
                    <Typography variant="caption" color="text.secondary">
                      kcal
                    </Typography>
                  ),
                },
              }}
              fullWidth
            />
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
            {save.isPending ? 'Saving…' : 'Use this target'}
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
