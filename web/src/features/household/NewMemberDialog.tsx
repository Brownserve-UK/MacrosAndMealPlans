import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import { useNavigate } from '@tanstack/react-router';
import { useState, type FormEvent } from 'react';
import { ApiError } from '../../api/client';
import { useCreateMember } from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';

export function NewMemberDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const navigate = useNavigate();
  const create = useCreateMember();
  const [name, setName] = useState('');
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [failure, setFailure] = useState<string | null>(null);

  function handleClose() {
    setName('');
    setErrors({});
    setFailure(null);
    onClose();
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    if (!name.trim()) {
      setErrors({ display_name: 'Give them a name' });
      return;
    }
    try {
      const created = await create.mutateAsync({ display_name: name.trim() });
      handleClose();
      void navigate({ to: '/household/$id', params: { id: created.id } });
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
        <DialogTitle>Add someone</DialogTitle>
        <DialogContent>
          <Stack spacing={3} sx={{ pt: 0.5 }}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}
            <TextField
              label="Name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              error={Boolean(errors.display_name)}
              helperText={errors.display_name}
              autoFocus
              fullWidth
              required
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={handleClose}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={create.isPending}>
            {create.isPending ? 'Saving…' : 'Add'}
          </Button>
        </DialogActions>
      </form>
    </FormDialog>
  );
}
