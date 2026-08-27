import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useNavigate } from '@tanstack/react-router';
import { useState, type FormEvent } from 'react';
import { ApiError } from '../../api/client';
import { useCreateRecipe } from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';
import {
  RecipeComponentsEditor,
  linesAreValid,
  linesToComponents,
  newLine,
  type ComponentLine,
} from './RecipeComponentsEditor';
import { RecipeFields, type RecipeDraft } from './RecipeFields';

const EMPTY: RecipeDraft = { name: '', servings: '4' };

export function NewRecipeDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const navigate = useNavigate();
  const create = useCreateRecipe();
  const [draft, setDraft] = useState<RecipeDraft>(EMPTY);
  const [lines, setLines] = useState<ComponentLine[]>([newLine()]);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [failure, setFailure] = useState<string | null>(null);

  function handleClose() {
    setDraft(EMPTY);
    setLines([newLine()]);
    setErrors({});
    setFailure(null);
    onClose();
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    const found: Record<string, string> = {};
    if (!draft.name.trim()) found.name = 'Give it a name';
    const servings = Number(draft.servings);
    if (!Number.isInteger(servings) || servings <= 0) found.servings = 'Serves must be a whole number above zero';
    setErrors(found);
    if (Object.keys(found).length > 0) return;

    const components = linesAreValid(lines) ? linesToComponents(lines) : null;
    if (!components) {
      setFailure('Add at least one product, each with an amount.');
      return;
    }

    try {
      const created = await create.mutateAsync({
        name: draft.name.trim(),
        servings,
        components,
      });
      handleClose();
      void navigate({ to: '/recipes/$id', params: { id: created.id } });
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
        <DialogTitle>New recipe</DialogTitle>
        <DialogContent>
          <Stack spacing={3} sx={{ pt: 0.5 }}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}
            <RecipeFields draft={draft} errors={errors} onChange={setDraft} autoFocus />
            <div>
              <Typography variant="h3" sx={{ mb: 1.5 }}>
                Products
              </Typography>
              <RecipeComponentsEditor lines={lines} onChange={setLines} />
            </div>
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={handleClose}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={create.isPending}>
            {create.isPending ? 'Saving…' : 'Create recipe'}
          </Button>
        </DialogActions>
      </form>
    </FormDialog>
  );
}
