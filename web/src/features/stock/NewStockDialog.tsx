import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import { useState, type FormEvent } from 'react';
import { ApiError } from '../../api/client';
import { useCreateStockItem } from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';
import {
  draftToLevel,
  emptyStockDraft,
  StockFields,
  validateStockDraft,
  type StockDraft,
} from './StockFields';

export function NewStockDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const create = useCreateStockItem();
  const [draft, setDraft] = useState<StockDraft>(emptyStockDraft());
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [failure, setFailure] = useState<string | null>(null);

  function handleClose() {
    setDraft(emptyStockDraft());
    setErrors({});
    setFailure(null);
    onClose();
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    const found = validateStockDraft(draft);
    setErrors(found);
    if (Object.keys(found).length > 0) return;

    try {
      await create.mutateAsync({
        product_id: draft.product!.id,
        level: draftToLevel(draft),
        storage_location: draft.storageLocation,
        note: draft.note.trim() || null,
      });
      handleClose();
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
        <DialogTitle>Add stock</DialogTitle>
        <DialogContent>
          <Stack spacing={3} sx={{ pt: 0.5 }}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}
            <StockFields draft={draft} errors={errors} onChange={setDraft} />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={handleClose}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={create.isPending}>
            {create.isPending ? 'Saving…' : 'Add stock'}
          </Button>
        </DialogActions>
      </form>
    </FormDialog>
  );
}
