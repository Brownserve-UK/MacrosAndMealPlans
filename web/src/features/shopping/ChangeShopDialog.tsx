import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import { useState, type FormEvent } from 'react';
import { ApiError } from '../../api/client';
import {
  useMoveShoppingOpportunity,
  useRestoreShoppingOpportunity,
  useSkipShoppingOpportunity,
} from '../../api/queries';
import { ConflictDialog } from '../../components/ConflictDialog';
import { FormDialog } from '../../components/FormDialog';

export function ChangeShopDialog({
  date,
  revision,
  earliest,
  changed,
  oneOff,
  onClose,
}: {
  date: string | null;
  revision: number;
  earliest: string;
  changed: boolean;
  oneOff: boolean;
  onClose: () => void;
}) {
  if (!date) return null;
  return (
    <Body
      key={date}
      date={date}
      revision={revision}
      earliest={earliest}
      changed={changed}
      oneOff={oneOff}
      onClose={onClose}
    />
  );
}

function Body({
  date,
  revision,
  earliest,
  changed,
  oneOff,
  onClose,
}: {
  date: string;
  revision: number;
  earliest: string;
  changed: boolean;
  oneOff: boolean;
  onClose: () => void;
}) {
  const move = useMoveShoppingOpportunity();
  const skip = useSkipShoppingOpportunity();
  const restore = useRestoreShoppingOpportunity();
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

  async function onRestore() {
    setFailure(null);
    try {
      await restore.mutateAsync({ date, revision });
      onClose();
    } catch (caught) {
      if (caught instanceof ApiError && caught.isConflict) setConflict(caught);
      else setFailure(caught instanceof ApiError ? caught.message : 'Could not restore that shop.');
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
          {changed ? (
            <Button disabled={restore.isPending} onClick={() => void onRestore()}>
              {oneOff ? 'Remove extra trip' : 'Restore shop'}
            </Button>
          ) : (
            <Button color="inherit" disabled={skip.isPending} onClick={() => void onSkip()}>
              Skip it
            </Button>
          )}
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
