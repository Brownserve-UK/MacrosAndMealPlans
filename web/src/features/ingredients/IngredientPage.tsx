import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Grid from '@mui/material/Grid';
import Paper from '@mui/material/Paper';
import Snackbar from '@mui/material/Snackbar';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import { useState, type FormEvent } from 'react';
import { ApiError, type Ingredient } from '../../api/client';
import {
  useIngredient,
  useIngredientProducts,
  useSetIngredientArchived,
  useUpdateIngredient,
} from '../../api/queries';
import { BackLabel } from '../../components/BackLink';
import { ConflictDialog } from '../../components/ConflictDialog';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { RecordListShell, RecordRow } from '../../components/RecordList';
import { PageHeader } from '../../components/PageHeader';
import { RecordMenu } from '../../components/RecordMenu';
import { ErrorState, Loading } from '../../components/States';
import { NewProductDialog } from '../products/NewProductDialog';
import { IngredientFields, type IngredientDraft } from './IngredientFields';

export function IngredientPage({ id }: { id: string }) {
  return <EditIngredient id={id} />;
}

function Frame({
  title,
  subtitle,
  actions,
  form,
  aside,
  banner,
}: {
  title: React.ReactNode;
  subtitle?: string;
  actions?: React.ReactNode;
  form: React.ReactNode;
  aside: React.ReactNode;
  banner?: React.ReactNode;
}) {
  return (
    <>
      <PageHeader
        back={
          <Link to="/ingredients" className="app-link">
            <BackLabel>Ingredients</BackLabel>
          </Link>
        }
        title={title}
        subtitle={subtitle}
        actions={actions}
      />
      {banner}
      <Grid container spacing={3}>
        <Grid size={{ xs: 12, md: 5 }}>{aside}</Grid>
        <Grid size={{ xs: 12, md: 7 }}>
          <Paper sx={{ p: 3 }}>{form}</Paper>
        </Grid>
      </Grid>
    </>
  );
}

function ProductsPanel({ id, onAddProduct }: { id: string; onAddProduct: () => void }) {
  const products = useIngredientProducts(id);
  const items = products.data?.items ?? [];

  return (
    <Paper sx={{ p: 3 }}>
      <Typography variant="h3" sx={{ mb: 2.5 }}>
        Products
      </Typography>

      {items.length > 0 ? (
        <RecordListShell>
          {items.map((product) => (
            <Link key={product.id} to="/products/$id" params={{ id: product.id }}>
              <RecordRow
                name={product.name}
                detail={[product.brand, product.retailer].filter(Boolean).join(' · ') || '—'}
                trailing={
                  product.nutrition?.energy_kcal != null ? (
                    <Typography className="numeral" variant="body2" color="text.secondary">
                      {product.nutrition.energy_kcal} kcal
                    </Typography>
                  ) : (
                    <Chip size="small" variant="outlined" label="No nutrition" />
                  )
                }
              />
            </Link>
          ))}
        </RecordListShell>
      ) : (
        <Box sx={{ py: 3, textAlign: 'center' }}>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            No products linked yet.
          </Typography>
          <Button onClick={onAddProduct} variant="outlined" size="small">
            Add a product
          </Button>
        </Box>
      )}
    </Paper>
  );
}

function EditIngredient({ id }: { id: string }) {
  const query = useIngredient(id);
  const archive = useSetIngredientArchived();
  const [conflict, setConflict] = useState<ApiError | null>(null);
  const [saved, setSaved] = useState(false);
  const [addProductOpen, setAddProductOpen] = useState(false);

  if (query.isLoading) return <Loading label="Loading" />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;

  const ingredient = query.data;
  if (!ingredient) return null;

  async function onToggleArchive() {
    if (!ingredient) return;
    try {
      await archive.mutateAsync({
        id: ingredient.id,
        revision: ingredient.revision,
        archived: !ingredient.archived_at,
      });
    } catch (caught) {
      if (caught instanceof ApiError && caught.isConflict) setConflict(caught);
    }
  }

  return (
    <>
      <Frame
        title={
          <Stack direction="row" spacing={2} sx={{ alignItems: 'center' }}>
            <InitialsAvatar name={ingredient.name} size={52} />
            <span>{ingredient.name}</span>
          </Stack>
        }
        actions={
          <RecordMenu
            archived={Boolean(ingredient.archived_at)}
            onToggleArchive={onToggleArchive}
            origin={ingredient.provenance.origin}
            locallyEdited={ingredient.provenance.locally_modified}
            updatedAt={ingredient.updated_at}
          />
        }
        banner={
          ingredient.archived_at ? (
            <Alert severity="info" sx={{ mb: 3 }}>
              Archived.
            </Alert>
          ) : null
        }
        form={
          <EditIngredientForm
            key={`${ingredient.id}:${ingredient.revision}`}
            ingredient={ingredient}
            onSaved={() => setSaved(true)}
            onConflict={setConflict}
          />
        }
        aside={
          <ProductsPanel id={ingredient.id} onAddProduct={() => setAddProductOpen(true)} />
        }
      />

      <ConflictDialog
        error={conflict}
        onDismiss={() => setConflict(null)}
        onReload={() => {
          setConflict(null);
          void query.refetch();
        }}
      />
      <NewProductDialog open={addProductOpen} onClose={() => setAddProductOpen(false)} />
      <Snackbar open={saved} autoHideDuration={3000} onClose={() => setSaved(false)} message="Saved" />
    </>
  );
}

function EditIngredientForm({
  ingredient,
  onSaved,
  onConflict,
}: {
  ingredient: Ingredient;
  onSaved: () => void;
  onConflict: (error: ApiError) => void;
}) {
  const update = useUpdateIngredient();
  const [draft, setDraft] = useState<IngredientDraft>({
    name: ingredient.name,
    default_unit: ingredient.default_unit,
  });
  const [errors, setErrors] = useState<Record<string, string>>({});

  const dirty =
    draft.name !== ingredient.name || draft.default_unit !== ingredient.default_unit;

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    if (!draft.name.trim()) {
      setErrors({ name: 'Give it a name' });
      return;
    }
    try {
      await update.mutateAsync({
        id: ingredient.id,
        revision: ingredient.revision,
        body: { name: draft.name.trim(), default_unit: draft.default_unit },
      });
      onSaved();
    } catch (caught) {
      if (caught instanceof ApiError) {
        if (caught.isConflict) onConflict(caught);
        else setErrors(caught.fieldErrors);
      }
    }
  }

  return (
    <form onSubmit={onSubmit}>
      <Stack spacing={3}>
        <IngredientFields draft={draft} errors={errors} onChange={setDraft} />
        <Box>
          <Button type="submit" variant="contained" disabled={!dirty || update.isPending}>
            {update.isPending ? 'Saving…' : 'Save changes'}
          </Button>
        </Box>
      </Stack>
    </form>
  );
}
