import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Typography from '@mui/material/Typography';

export function DeleteMealDialog({
  open,
  description,
  busy,
  onCancel,
  onDelete,
}: {
  open: boolean;
  description: string;
  busy: boolean;
  onCancel: () => void;
  onDelete: () => void;
}) {
  return (
    <Dialog open={open} onClose={busy ? undefined : onCancel}>
      <DialogTitle>Delete this meal?</DialogTitle>
      <DialogContent><Typography>{description}</Typography></DialogContent>
      <DialogActions>
        <Button onClick={onCancel} disabled={busy}>Cancel</Button>
        <Button color="error" variant="contained" onClick={onDelete} disabled={busy}>Delete meal</Button>
      </DialogActions>
    </Dialog>
  );
}
