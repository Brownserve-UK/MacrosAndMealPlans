import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import { useNavigate } from '@tanstack/react-router';
import { useState, type FormEvent } from 'react';
import { ApiError } from '../../api/client';
import { useCreateIngredient } from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';
import { IngredientFields, type IngredientDraft } from './IngredientFields';

const EMPTY: IngredientDraft = { name: '', default_unit: 'g', track_stock: true };

export function NewIngredientDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const navigate = useNavigate();
  const create = useCreateIngredient();
  const [draft, setDraft] = useState<IngredientDraft>(EMPTY);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [failure, setFailure] = useState<string | null>(null);

  function handleClose() {
    setDraft(EMPTY);
    setErrors({});
    setFailure(null);
    onClose();
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    if (!draft.name.trim()) {
      setErrors({ name: 'Give it a name' });
      return;
    }
    try {
      const created = await create.mutateAsync({
        name: draft.name.trim(),
        default_unit: draft.default_unit,
        track_stock: draft.track_stock,
      });
      handleClose();
      void navigate({ to: '/ingredients/$id', params: { id: created.id } });
    } catch (caught) {
      if (caught instanceof ApiError) {
        const fields = caught.fieldErrors;
        if (Object.keys(fields).length > 0) setErrors(fields);
        else setFailure(caught.message);
      } else setFailure('Could not save.');
    }
  }

  return (
    <FormDialog open={open} onClose={handleClose} maxWidth="sm" fullWidth>
      <form onSubmit={onSubmit}>
        <DialogTitle>New ingredient</DialogTitle>
        <DialogContent>
          <Stack spacing={3} sx={{ pt: 0.5 }}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}
            <IngredientFields draft={draft} errors={errors} onChange={setDraft} autoFocus />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={handleClose}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={create.isPending}>
            {create.isPending ? 'Saving…' : 'Create ingredient'}
          </Button>
        </DialogActions>
      </form>
    </FormDialog>
  );
}
