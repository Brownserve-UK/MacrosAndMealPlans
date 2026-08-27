import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Grid from '@mui/material/Grid';
import Paper from '@mui/material/Paper';
import Snackbar from '@mui/material/Snackbar';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import { useEffect, useState, type FormEvent } from 'react';
import { ApiError, type Recipe, type RecipeNutrition } from '../../api/client';
import {
  useRecipe,
  useRecipeNutrition,
  useRecipeNutritionPreview,
  useSetRecipeArchived,
  useUpdateRecipe,
} from '../../api/queries';
import { BackLabel } from '../../components/BackLink';
import { ConflictDialog } from '../../components/ConflictDialog';
import { PageHeader } from '../../components/PageHeader';
import { RecordMenu } from '../../components/RecordMenu';
import { ErrorState, Loading } from '../../components/States';
import { useDebounced } from '../../hooks/useDebounced';
import {
  RecipeComponentsEditor,
  linesAreValid,
  linesFromComponents,
  linesToComponents,
  type ComponentLine,
} from './RecipeComponentsEditor';
import { RecipeFields, type RecipeDraft } from './RecipeFields';

export function RecipePage({ id }: { id: string }) {
  const query = useRecipe(id);
  const archive = useSetRecipeArchived();
  const preview = useRecipeNutritionPreview();
  const [conflict, setConflict] = useState<ApiError | null>(null);
  const [saved, setSaved] = useState(false);

  if (query.isLoading) return <Loading label="Loading" />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;

  const recipe = query.data;
  if (!recipe) return null;

  async function onToggleArchive() {
    if (!recipe) return;
    try {
      await archive.mutateAsync({
        id: recipe.id,
        revision: recipe.revision,
        archived: !recipe.archived_at,
      });
    } catch (caught) {
      if (caught instanceof ApiError && caught.isConflict) setConflict(caught);
    }
  }

  return (
    <>
      <PageHeader
        back={
          <Link to="/recipes" className="app-link">
            <BackLabel>Recipes</BackLabel>
          </Link>
        }
        title={recipe.name}
        subtitle={`Serves ${recipe.servings}`}
        actions={
          <RecordMenu
            archived={Boolean(recipe.archived_at)}
            onToggleArchive={onToggleArchive}
            updatedAt={recipe.updated_at}
          />
        }
      />
      {recipe.archived_at ? (
        <Alert severity="info" sx={{ mb: 3 }}>
          Archived.
        </Alert>
      ) : null}

      <Grid container spacing={3}>
        <Grid size={{ xs: 12, md: 7 }}>
          <Paper sx={{ p: 3 }}>
            <EditRecipeForm
              key={`${recipe.id}:${recipe.revision}`}
              recipe={recipe}
              preview={preview}
              onSaved={() => setSaved(true)}
              onConflict={setConflict}
            />
          </Paper>
        </Grid>
        <Grid size={{ xs: 12, md: 5 }}>
          <NutritionPanel recipeId={recipe.id} preview={preview} />
        </Grid>
      </Grid>

      <ConflictDialog
        error={conflict}
        onDismiss={() => setConflict(null)}
        onReload={() => {
          setConflict(null);
          void query.refetch();
        }}
      />
      <Snackbar open={saved} autoHideDuration={3000} onClose={() => setSaved(false)} message="Saved" />
    </>
  );
}

function EditRecipeForm({
  recipe,
  preview,
  onSaved,
  onConflict,
}: {
  recipe: Recipe;
  preview: ReturnType<typeof useRecipeNutritionPreview>;
  onSaved: () => void;
  onConflict: (error: ApiError) => void;
}) {
  const update = useUpdateRecipe();
  const [draft, setDraft] = useState<RecipeDraft>({
    name: recipe.name,
    servings: String(recipe.servings),
  });
  const [lines, setLines] = useState<ComponentLine[]>(() => linesFromComponents(recipe.components));
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [failure, setFailure] = useState<string | null>(null);

  const servings = Number(draft.servings);
  const valid = linesAreValid(lines) && Number.isInteger(servings) && servings > 0;
  const previewKey = useDebounced(JSON.stringify({ servings, lines }), 400);

  useEffect(() => {
    if (!valid) return;
    const components = linesToComponents(lines);
    if (!components) return;
    preview.mutate({ servings, components });
    // We deliberately re-run only when the debounced draft changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [previewKey]);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    const found: Record<string, string> = {};
    if (!draft.name.trim()) found.name = 'Give it a name';
    if (!Number.isInteger(servings) || servings <= 0) found.servings = 'Serves must be a whole number above zero';
    setErrors(found);
    if (Object.keys(found).length > 0) return;

    const components = linesAreValid(lines) ? linesToComponents(lines) : null;
    if (!components) {
      setFailure('Add at least one product, each with an amount.');
      return;
    }

    try {
      await update.mutateAsync({
        id: recipe.id,
        revision: recipe.revision,
        body: { name: draft.name.trim(), servings, components },
      });
      onSaved();
    } catch (caught) {
      if (caught instanceof ApiError) {
        if (caught.isConflict) onConflict(caught);
        else setErrors(caught.fieldErrors);
      } else setFailure('Could not save.');
    }
  }

  return (
    <form onSubmit={onSubmit}>
      <Stack spacing={3}>
        {failure ? <Alert severity="error">{failure}</Alert> : null}
        <RecipeFields draft={draft} errors={errors} onChange={setDraft} />
        <Box>
          <Typography variant="h3" sx={{ mb: 1.5 }}>
            Products
          </Typography>
          <RecipeComponentsEditor lines={lines} onChange={setLines} />
        </Box>
        <Box>
          <Button type="submit" variant="contained" disabled={update.isPending}>
            {update.isPending ? 'Saving…' : 'Save changes'}
          </Button>
        </Box>
      </Stack>
    </form>
  );
}

function NutritionPanel({
  recipeId,
  preview,
}: {
  recipeId: string;
  preview: ReturnType<typeof useRecipeNutritionPreview>;
}) {
  const saved = useRecipeNutrition(recipeId);
  // The live preview (driven by the edit form) wins once it has produced a value.
  const data = preview.data ?? saved.data;

  return (
    <Paper sx={{ p: 3 }}>
      <Typography variant="h3" sx={{ mb: 0.5 }}>
        Nutrition
      </Typography>
      <Typography variant="body2" color="text.secondary" sx={{ mb: 2.5 }}>
        Per serving
      </Typography>
      {data ? <NutritionRows data={data} /> : <Loading label="Working it out" />}
    </Paper>
  );
}

const ROWS: { key: 'energy_kcal' | 'protein_g' | 'carbohydrate_g' | 'fat_g'; label: string; unit: string }[] = [
  { key: 'energy_kcal', label: 'Energy', unit: 'kcal' },
  { key: 'protein_g', label: 'Protein', unit: 'g' },
  { key: 'carbohydrate_g', label: 'Carbohydrate', unit: 'g' },
  { key: 'fat_g', label: 'Fat', unit: 'g' },
];

function NutritionRows({ data }: { data: RecipeNutrition }) {
  const facts = data.nutrition;
  return (
    <Stack spacing={1.5}>
      {ROWS.map((row) => {
        const value = facts[row.key];
        return (
          <Stack key={row.key} direction="row" sx={{ justifyContent: 'space-between' }}>
            <Typography variant="body2" color="text.secondary">
              {row.label}
            </Typography>
            <Typography variant="body2" className="numeral">
              {value == null ? '—' : `${Math.round(value * 10) / 10} ${row.unit}`}
            </Typography>
          </Stack>
        );
      })}
      {data.quality !== 'known' ? (
        <Alert severity="warning" sx={{ mt: 1 }}>
          {data.quality === 'unknown'
            ? 'None of these products have nutrition data yet, so this is unknown.'
            : 'Some products are missing nutrition data, so this is only partial.'}
        </Alert>
      ) : null}
    </Stack>
  );
}
