import DeleteIcon from '@mui/icons-material/DeleteOutlineOutlined';
import EditIcon from '@mui/icons-material/EditOutlined';
import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link, useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { ApiError } from '../../api/client';
import { useDeleteMealTemplate, useMealTemplate } from '../../api/queries';
import { BackLabel } from '../../components/BackLink';
import { IconTile } from '../../components/IconTile';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { formatAmount } from '../meal-plan/format';
import { SavedMealEditorDialog } from './SavedMealEditorDialog';

export function SavedMealPage({ id }: { id: string }) {
  const query = useMealTemplate(id);
  const remove = useDeleteMealTemplate();
  const navigate = useNavigate();
  const [editing, setEditing] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (query.isLoading) return <Loading label="Loading saved meal" />;
  if (query.isError || !query.data) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;

  const template = query.data;

  async function onDelete() {
    try {
      await remove.mutateAsync({ id: template.id, revision: template.revision });
      void navigate({ to: '/saved-meals' });
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not delete this saved meal.');
      setDeleting(false);
    }
  }

  return (
    <>
      <PageHeader
        back={
          <Link to="/saved-meals" className="app-link">
            <BackLabel>Saved meals</BackLabel>
          </Link>
        }
        title={
          <Stack direction="row" spacing={2} sx={{ alignItems: 'center' }}>
            <IconTile concept="meal" />
            <span>{template.name}</span>
          </Stack>
        }
        actions={
          <Stack direction="row" spacing={1}>
            <Button startIcon={<EditIcon />} onClick={() => setEditing(true)}>Edit</Button>
            <Button color="error" startIcon={<DeleteIcon />} onClick={() => setDeleting(true)}>Delete</Button>
          </Stack>
        }
      />

      {error ? <Typography color="error.main" sx={{ mb: 2 }}>{error}</Typography> : null}

      <Stack spacing={1.5}>
        {template.components.map((component) => (
          <Stack
            key={component.id}
            direction="row"
            spacing={2}
            sx={{
              justifyContent: 'space-between',
              alignItems: 'center',
              px: 2.25,
              py: 1.5,
              border: '1px solid',
              borderColor: 'divider',
              borderRadius: 2,
            }}
          >
            <Typography>{component.item_name}</Typography>
            <Typography variant="body2" color="text.secondary" className="numeral">
              {formatAmount(component.amount)}
            </Typography>
          </Stack>
        ))}
      </Stack>

      <SavedMealEditorDialog open={editing} onClose={() => setEditing(false)} template={template} />

      <Dialog open={deleting} onClose={() => setDeleting(false)} maxWidth="xs" fullWidth>
        <DialogTitle>Delete "{template.name}"?</DialogTitle>
        <DialogContent>
          <Typography color="text.secondary">
            This only removes the saved meal. Meals already planned from it are unaffected.
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDeleting(false)} disabled={remove.isPending}>Cancel</Button>
          <Button color="error" variant="contained" onClick={() => void onDelete()} disabled={remove.isPending}>
            {remove.isPending ? 'Deleting…' : 'Delete'}
          </Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
