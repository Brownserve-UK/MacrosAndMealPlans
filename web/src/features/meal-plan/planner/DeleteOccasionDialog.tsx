import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Typography from '@mui/material/Typography';
import { FormDialog } from '../../../components/FormDialog';

export function DeleteOccasionDialog({
  title,
  busy,
  onCancel,
  onDelete,
}: {
  title: string;
  busy: boolean;
  onCancel: () => void;
  onDelete: () => void;
}) {
  return (
    <FormDialog open onClose={busy ? undefined : onCancel} fullWidth maxWidth="xs">
      <DialogTitle>Delete this meal?</DialogTitle>
      <DialogContent>
        <Typography>{title}</Typography>
      </DialogContent>
      <DialogActions>
        <Button onClick={onCancel} disabled={busy}>
          Keep it
        </Button>
        <Button color="error" onClick={onDelete} disabled={busy}>
          Delete
        </Button>
      </DialogActions>
    </FormDialog>
  );
}
