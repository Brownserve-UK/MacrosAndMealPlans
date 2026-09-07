import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import { useState, type FormEvent } from 'react';
import { ApiError } from '../../api/client';
import { useAddShoppingOpportunity } from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';
import { todayIso } from '../meal-plan/date';

export function AddShopDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const add = useAddShoppingOpportunity();
  const [date, setDate] = useState(todayIso());
  const [failure, setFailure] = useState<string | null>(null);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    try {
      await add.mutateAsync({ date });
      onClose();
    } catch (caught) {
      setFailure(caught instanceof ApiError ? caught.message : 'Could not add that shop.');
    }
  }

  return (
    <FormDialog open={open} onClose={onClose} maxWidth="xs" fullWidth>
      <form onSubmit={onSubmit}>
        <DialogTitle>Add a shop</DialogTitle>
        <DialogContent>
          <Stack spacing={2.5} sx={{ pt: 0.5 }}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}
            <TextField
              type="date"
              label="Shop on"
              value={date}
              onChange={(e) => setDate(e.target.value)}
              slotProps={{ htmlInput: { min: todayIso() } }}
              fullWidth
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={onClose}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={add.isPending}>
            Add
          </Button>
        </DialogActions>
      </form>
    </FormDialog>
  );
}
