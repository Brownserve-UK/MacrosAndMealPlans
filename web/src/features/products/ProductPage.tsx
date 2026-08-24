import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Grid from '@mui/material/Grid';
import Paper from '@mui/material/Paper';
import Snackbar from '@mui/material/Snackbar';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import { useState, type FormEvent } from 'react';
import { ApiError, type Ingredient, type Product } from '../../api/client';
import {
  useIngredient,
  useProduct,
  useSetProductArchived,
  useSetProductMapping,
  useUpdateProduct,
} from '../../api/queries';
import { BackLabel } from '../../components/BackLink';
import { ConflictDialog } from '../../components/ConflictDialog';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import {
  draftToNutrition,
  nutritionToDraft,
  validateNutritionDraft,
} from '../../components/NutritionFields';
import { NutritionPanel } from '../../components/NutritionPanel';
import { PageHeader } from '../../components/PageHeader';
import { RecordMenu } from '../../components/RecordMenu';
import { ErrorState, Loading } from '../../components/States';
import { IngredientPicker } from './IngredientPicker';
import { packContextFrom, ProductFields, type ProductDraft } from './ProductFields';

export function ProductPage({ id }: { id: string }) {
  return <EditProduct id={id} />;
}

function draftFrom(product: Product): ProductDraft {
  const packAmount = product.package_quantity ? String(product.package_quantity.amount) : '';
  const packUnit = product.package_quantity?.unit ?? 'g';
  const servingsPerPack = product.servings_per_pack ? String(product.servings_per_pack) : '';
  return {
    name: product.name,
    brand: product.brand ?? '',
    barcode: product.barcode ?? '',
    retailer: product.retailer ?? '',
    section: product.shopping_section ?? '',
    packAmount,
    packUnit,
    servingsPerPack,
    nutrition: nutritionToDraft(
      product.nutrition,
      packContextFrom({ packAmount, packUnit, servingsPerPack }),
    ),
  };
}

function validate(draft: ProductDraft): Record<string, string> {
  const errors = validateNutritionDraft(draft.nutrition, packContextFrom(draft));
  if (!draft.name.trim()) errors.name = 'Give it a name';
  if (draft.packAmount.trim() && Number(draft.packAmount) <= 0) {
    errors.package_quantity = 'Must be more than zero';
  }
  if (draft.servingsPerPack.trim()) {
    if (!draft.packAmount.trim()) {
      errors.servings_per_pack = 'Needs a pack size to divide up';
    } else if (
      !Number.isInteger(Number(draft.servingsPerPack)) ||
      Number(draft.servingsPerPack) <= 0
    ) {
      errors.servings_per_pack = 'Must be a whole number more than zero';
    }
  }
  return errors;
}

function toBody(draft: ProductDraft) {
  return {
    name: draft.name.trim(),
    brand: draft.brand.trim() || null,
    barcode: draft.barcode.trim() || null,
    retailer: draft.retailer.trim() || null,
    shopping_section: draft.section.trim() || null,
    package_quantity: draft.packAmount.trim()
      ? { amount: Number(draft.packAmount), unit: draft.packUnit }
      : null,
    servings_per_pack: draft.servingsPerPack.trim() ? Number(draft.servingsPerPack) : null,
    nutrition: draftToNutrition(draft.nutrition, packContextFrom(draft)),
  };
}

function Frame({
  title,
  subtitle,
  actions,
  aside,
  form,
  banner,
}: {
  title: React.ReactNode;
  subtitle?: string;
  actions?: React.ReactNode;
  aside: React.ReactNode;
  form: React.ReactNode;
  banner?: React.ReactNode;
}) {
  return (
    <>
      <PageHeader
        back={
          <Link to="/products" className="app-link">
            <BackLabel>Products</BackLabel>
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

function EditProduct({ id }: { id: string }) {
  const query = useProduct(id);
  const setMapping = useSetProductMapping();
  const archive = useSetProductArchived();
  const [conflict, setConflict] = useState<ApiError | null>(null);
  const [saved, setSaved] = useState(false);

  const product = query.data;
  const mapped = useIngredient(product?.mapped_ingredient_id ?? '');

  if (query.isLoading) return <Loading label="Loading" />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;
  if (!product) return null;

  async function onMappingChange(next: Ingredient | null) {
    if (!product) return;
    try {
      await setMapping.mutateAsync({
        id: product.id,
        revision: product.revision,
        ingredientId: next?.id ?? null,
      });
    } catch (caught) {
      if (caught instanceof ApiError && caught.isConflict) setConflict(caught);
    }
  }

  async function onToggleArchive() {
    if (!product) return;
    try {
      await archive.mutateAsync({
        id: product.id,
        revision: product.revision,
        archived: !product.archived_at,
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
            <InitialsAvatar name={product.name} size={52} />
            <span>{product.name}</span>
          </Stack>
        }
        subtitle={[product.brand, product.retailer].filter(Boolean).join(' · ') || undefined}
        actions={
          <RecordMenu
            archived={Boolean(product.archived_at)}
            onToggleArchive={onToggleArchive}
            origin={product.provenance.origin}
            locallyEdited={product.provenance.locally_modified}
            updatedAt={product.updated_at}
          />
        }
        banner={
          product.archived_at ? (
            <Alert severity="info" sx={{ mb: 3 }}>
              Archived.
            </Alert>
          ) : null
        }
        aside={
          <Stack spacing={3}>
            <Paper sx={{ p: 3 }}>
              <NutritionPanel
                nutrition={product.nutrition}
                pack={{
                  quantity: product.package_quantity
                    ? {
                        amount: Number(product.package_quantity.amount),
                        unit: product.package_quantity.unit,
                      }
                    : undefined,
                  servings: product.servings_per_pack ?? undefined,
                }}
              />
            </Paper>
            <Paper sx={{ p: 3 }}>
              <Typography variant="h3" sx={{ mb: 2 }}>
                Ingredient
              </Typography>
              <IngredientPicker
                value={mapped.data ?? null}
                onChange={onMappingChange}
                disabled={setMapping.isPending}
              />
            </Paper>
          </Stack>
        }
        form={
          <EditProductForm
            key={`${product.id}:${product.revision}`}
            product={product}
            onSaved={() => setSaved(true)}
            onConflict={setConflict}
          />
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
      <Snackbar open={saved} autoHideDuration={3000} onClose={() => setSaved(false)} message="Saved" />
    </>
  );
}

function EditProductForm({
  product,
  onSaved,
  onConflict,
}: {
  product: Product;
  onSaved: () => void;
  onConflict: (error: ApiError) => void;
}) {
  const update = useUpdateProduct();
  const [draft, setDraft] = useState<ProductDraft>(draftFrom(product));
  const [errors, setErrors] = useState<Record<string, string>>({});

  const dirty = JSON.stringify(draft) !== JSON.stringify(draftFrom(product));

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    const found = validate(draft);
    setErrors(found);
    if (Object.keys(found).length > 0) return;

    try {
      await update.mutateAsync({
        id: product.id,
        revision: product.revision,
        body: toBody(draft),
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
        <ProductFields draft={draft} errors={errors} onChange={setDraft} />
        <Box>
          <Button type="submit" variant="contained" disabled={!dirty || update.isPending}>
            {update.isPending ? 'Saving…' : 'Save changes'}
          </Button>
        </Box>
      </Stack>
    </form>
  );
}
