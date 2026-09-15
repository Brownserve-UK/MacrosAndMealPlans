import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useNavigate } from '@tanstack/react-router';
import { useState, type FormEvent } from 'react';
import { ApiError, type MealPlanEntry } from '../../api/client';
import { useCreateMealTemplateFromEntry } from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';
import { formatAmount } from './format';

function eligibleComponents(entry: MealPlanEntry) {
  return entry.components.filter((component) => component.item_kind !== 'dish');
}

export function SaveForReuseDialog({
  open,
  onClose,
  entry,
}: {
  open: boolean;
  onClose: () => void;
  entry: MealPlanEntry;
}) {
  const eligible = eligibleComponents(entry);
  const create = useCreateMealTemplateFromEntry();
  const navigate = useNavigate();
  const [name, setName] = useState(() => eligible.map((component) => component.item_name).join(', '));
  const [error, setError] = useState<string | null>(null);

  function handleClose() {
    if (create.isPending) return;
    setError(null);
    onClose();
  }

  async function save(event: FormEvent) {
    event.preventDefault();
    if (!name.trim()) {
      setError('Give this saved meal a name.');
      return;
    }
    try {
      const created = await create.mutateAsync({ entryId: entry.id, name: name.trim() });
      onClose();
      void navigate({ to: '/saved-meals/$id', params: { id: created.id } });
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not save this meal.');
    }
  }

  return (
    <FormDialog open={open} onClose={handleClose} maxWidth="sm" fullWidth>
      <form onSubmit={save}>
        <DialogTitle>Save for reuse</DialogTitle>
        <DialogContent>
          <Stack spacing={2.5} sx={{ pt: 0.5 }}>
            {error ? <Alert severity="error">{error}</Alert> : null}
            <TextField label="Name" value={name} onChange={(event) => setName(event.target.value)} autoFocus fullWidth />
            <Stack spacing={0.75}>
              {eligible.map((component) => (
                <Typography key={component.id} variant="body2" color="text.secondary">
                  {formatAmount(component.amount)} {component.item_name}
                </Typography>
              ))}
              {eligible.length < entry.components.length ? (
                <Typography variant="caption" color="text.secondary">
                  Cooked food already in the house won't be saved.
                </Typography>
              ) : null}
            </Stack>
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={handleClose} disabled={create.isPending}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={create.isPending}>
            {create.isPending ? 'Saving…' : 'Save'}
          </Button>
        </DialogActions>
      </form>
    </FormDialog>
  );
}
