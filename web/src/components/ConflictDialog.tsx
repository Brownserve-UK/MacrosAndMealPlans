import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogContentText from '@mui/material/DialogContentText';
import DialogTitle from '@mui/material/DialogTitle';
import type { ApiError } from '../api/client';

export function ConflictDialog({
  error,
  onReload,
  onDismiss,
}: {
  error: ApiError | null;
  onReload: () => void;
  onDismiss: () => void;
}) {
  const open = error?.isConflict ?? false;

  return (
    <Dialog open={open} onClose={onDismiss} maxWidth="sm" fullWidth>
      <DialogTitle>Someone else changed this first</DialogTitle>
      <DialogContent>
        <DialogContentText component="div">
          Saving now would overwrite their change.
        </DialogContentText>

      </DialogContent>
      <DialogActions>
        <Button onClick={onDismiss}>Keep mine</Button>
        <Button onClick={onReload} variant="contained">
          Reload
        </Button>
      </DialogActions>
    </Dialog>
  );
}
