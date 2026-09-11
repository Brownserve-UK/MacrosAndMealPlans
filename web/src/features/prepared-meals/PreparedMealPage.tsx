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
import { ApiError, type PreparedMeal } from '../../api/client';
import {
  usePreparedMeal,
  usePreparedMealProducts,
  useSetPreparedMealArchived,
  useUpdatePreparedMeal,
} from '../../api/queries';
import { BackLabel } from '../../components/BackLink';
import { ConflictDialog } from '../../components/ConflictDialog';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { RecordListShell, RecordRow } from '../../components/RecordList';
import { PageHeader } from '../../components/PageHeader';
import { RecordMenu } from '../../components/RecordMenu';
import { ErrorState, Loading } from '../../components/States';
import { IngredientFields, type IngredientDraft } from '../ingredients/IngredientFields';
import { NewProductDialog } from '../products/NewProductDialog';

export function PreparedMealPage({ id }: { id: string }) {
  return <EditPreparedMeal id={id} />;
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
          <Link to="/foods" className="app-link">
            <BackLabel>Foods</BackLabel>
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
  const products = usePreparedMealProducts(id);
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

function EditPreparedMeal({ id }: { id: string }) {
  const query = usePreparedMeal(id);
  const archive = useSetPreparedMealArchived();
  const [conflict, setConflict] = useState<ApiError | null>(null);
  const [saved, setSaved] = useState(false);
  const [addProductOpen, setAddProductOpen] = useState(false);

  if (query.isLoading) return <Loading label="Loading" />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;

  const preparedMeal = query.data;
  if (!preparedMeal) return null;

  async function onToggleArchive() {
    if (!preparedMeal) return;
    try {
      await archive.mutateAsync({
        id: preparedMeal.id,
        revision: preparedMeal.revision,
        archived: !preparedMeal.archived_at,
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
            <InitialsAvatar name={preparedMeal.name} size={52} />
            <span>{preparedMeal.name}</span>
          </Stack>
        }
        actions={
          <RecordMenu
            archived={Boolean(preparedMeal.archived_at)}
            onToggleArchive={onToggleArchive}
            origin={preparedMeal.provenance.origin}
            locallyEdited={preparedMeal.provenance.locally_modified}
            updatedAt={preparedMeal.updated_at}
          />
        }
        banner={
          preparedMeal.archived_at ? (
            <Alert severity="info" sx={{ mb: 3 }}>
              Archived.
            </Alert>
          ) : null
        }
        form={
          <EditPreparedMealForm
            key={`${preparedMeal.id}:${preparedMeal.revision}`}
            preparedMeal={preparedMeal}
            onSaved={() => setSaved(true)}
            onConflict={setConflict}
          />
        }
        aside={<ProductsPanel id={preparedMeal.id} onAddProduct={() => setAddProductOpen(true)} />}
      />

      <ConflictDialog
        error={conflict}
        onDismiss={() => setConflict(null)}
        onReload={() => {
          setConflict(null);
          void query.refetch();
        }}
      />
      <NewProductDialog
        open={addProductOpen}
        onClose={() => setAddProductOpen(false)}
        mappedPreparedMealId={preparedMeal.id}
      />
      <Snackbar open={saved} autoHideDuration={3000} onClose={() => setSaved(false)} message="Saved" />
    </>
  );
}

function EditPreparedMealForm({
  preparedMeal,
  onSaved,
  onConflict,
}: {
  preparedMeal: PreparedMeal;
  onSaved: () => void;
  onConflict: (error: ApiError) => void;
}) {
  const update = useUpdatePreparedMeal();
  const [draft, setDraft] = useState<IngredientDraft>({
    name: preparedMeal.name,
    default_unit: preparedMeal.default_unit,
    track_stock: preparedMeal.track_stock ?? true,
  });
  const [errors, setErrors] = useState<Record<string, string>>({});

  const dirty =
    draft.name !== preparedMeal.name ||
    draft.default_unit !== preparedMeal.default_unit ||
    draft.track_stock !== (preparedMeal.track_stock ?? true);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    if (!draft.name.trim()) {
      setErrors({ name: 'Give it a name' });
      return;
    }
    try {
      await update.mutateAsync({
        id: preparedMeal.id,
        revision: preparedMeal.revision,
        body: {
          name: draft.name.trim(),
          default_unit: draft.default_unit,
          track_stock: draft.track_stock,
        },
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
