import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import { useState, type FormEvent } from 'react';
import { ApiError } from '../../api/client';
import { useMoveShoppingOpportunity, useSkipShoppingOpportunity } from '../../api/queries';
import { ConflictDialog } from '../../components/ConflictDialog';
import { FormDialog } from '../../components/FormDialog';

export function ChangeShopDialog({
  date,
  revision,
  earliest,
  onClose,
}: {
  date: string | null;
  revision: number;
  earliest: string;
  onClose: () => void;
}) {
  if (!date) return null;
  return <Body key={date} date={date} revision={revision} earliest={earliest} onClose={onClose} />;
}

function Body({
  date,
  revision,
  earliest,
  onClose,
}: {
  date: string;
  revision: number;
  earliest: string;
  onClose: () => void;
}) {
  const move = useMoveShoppingOpportunity();
  const skip = useSkipShoppingOpportunity();
  const [to, setTo] = useState(date);
  const [failure, setFailure] = useState<string | null>(null);
  const [conflict, setConflict] = useState<ApiError | null>(null);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    try {
      await move.mutateAsync({ date, to, revision });
      onClose();
    } catch (caught) {
      if (caught instanceof ApiError && caught.isConflict) setConflict(caught);
      else setFailure(caught instanceof ApiError ? caught.message : 'Could not move that shop.');
    }
  }

  async function onSkip() {
    setFailure(null);
    try {
      await skip.mutateAsync({ date, revision });
      onClose();
    } catch (caught) {
      if (caught instanceof ApiError && caught.isConflict) setConflict(caught);
      else setFailure(caught instanceof ApiError ? caught.message : 'Could not skip that shop.');
    }
  }

  return (
    <FormDialog open onClose={onClose} maxWidth="xs" fullWidth>
      <form onSubmit={onSubmit}>
        <DialogTitle>Change this shop</DialogTitle>
        <DialogContent>
          <Stack spacing={2.5} sx={{ pt: 0.5 }}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}
            <TextField
              type="date"
              label="Shop on"
              value={to}
              onChange={(e) => setTo(e.target.value)}
              slotProps={{ htmlInput: { min: earliest } }}
              fullWidth
            />
          </Stack>
        </DialogContent>
        <DialogActions sx={{ justifyContent: 'space-between' }}>
          <Button color="inherit" disabled={skip.isPending} onClick={() => void onSkip()}>
            Skip it
          </Button>
          <Stack direction="row" spacing={1}>
            <Button onClick={onClose}>Cancel</Button>
            <Button type="submit" variant="contained" disabled={move.isPending || to === date}>
              Move
            </Button>
          </Stack>
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
