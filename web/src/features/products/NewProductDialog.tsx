import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import { useNavigate } from '@tanstack/react-router';
import { useState, type FormEvent } from 'react';
import { ApiError } from '../../api/client';
import { useCreateProduct } from '../../api/queries';
import { draftToNutrition, nutritionToDraft, validateNutritionDraft } from '../../components/NutritionFields';
import { FormDialog } from '../../components/FormDialog';
import { packContextFrom, ProductFields, type ProductDraft } from './ProductFields';

function emptyDraft(): ProductDraft {
  const packAmount = '';
  const packUnit = 'g' as const;
  const servingsPerPack = '';
  return {
    name: '',
    brand: '',
    barcode: '',
    retailer: '',
    section: '',
    packAmount,
    packUnit,
    servingsPerPack,
    nutrition: nutritionToDraft(undefined, packContextFrom({ packAmount, packUnit, servingsPerPack })),
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

export function NewProductDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const navigate = useNavigate();
  const create = useCreateProduct();
  const [draft, setDraft] = useState<ProductDraft>(emptyDraft());
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [failure, setFailure] = useState<string | null>(null);

  function handleClose() {
    setDraft(emptyDraft());
    setErrors({});
    setFailure(null);
    onClose();
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    const found = validate(draft);
    setErrors(found);
    if (Object.keys(found).length > 0) return;

    try {
      const created = await create.mutateAsync(toBody(draft));
      handleClose();
      void navigate({ to: '/products/$id', params: { id: created.id } });
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
        <DialogTitle>New product</DialogTitle>
        <DialogContent>
          <Stack spacing={3} sx={{ pt: 0.5 }}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}
            <ProductFields draft={draft} errors={errors} onChange={setDraft} autoFocus />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={handleClose}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={create.isPending}>
            {create.isPending ? 'Saving…' : 'Create product'}
          </Button>
        </DialogActions>
      </form>
    </FormDialog>
  );
}
