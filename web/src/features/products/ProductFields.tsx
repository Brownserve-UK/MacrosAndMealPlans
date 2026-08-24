import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import type { Unit } from '../../api/client';
import { UnitSelect } from '../../components/UnitSelect';
import {
  NutritionFields,
  type NutritionDraft,
  type PackContext,
} from '../../components/NutritionFields';

export type ProductDraft = {
  name: string;
  brand: string;
  barcode: string;
  retailer: string;
  section: string;
  packAmount: string;
  packUnit: Unit;
  servingsPerPack: string;
  nutrition: NutritionDraft;
};

export function packContextFrom(
  draft: Pick<ProductDraft, 'packAmount' | 'packUnit' | 'servingsPerPack'>,
): PackContext {
  const amount = Number(draft.packAmount);
  const servings = Number(draft.servingsPerPack);
  return {
    quantity:
      draft.packAmount.trim() && !Number.isNaN(amount)
        ? { amount, unit: draft.packUnit }
        : undefined,
    servings: draft.servingsPerPack.trim() && !Number.isNaN(servings) ? servings : undefined,
  };
}

const TEXT: Array<{ key: keyof ProductDraft; label: string; span: number; errorKey?: string }> = [
  { key: 'name', label: 'Name', span: 12 },
  { key: 'brand', label: 'Brand', span: 6 },
  { key: 'retailer', label: 'Shop', span: 6 },
  { key: 'barcode', label: 'Barcode', span: 6 },
  { key: 'section', label: 'Aisle', span: 6, errorKey: 'shopping_section' },
];

export function ProductFields({
  draft,
  errors,
  onChange,
  autoFocus,
}: {
  draft: ProductDraft;
  errors: Record<string, string>;
  onChange: (next: ProductDraft) => void;
  autoFocus?: boolean;
}) {
  const set = <K extends keyof ProductDraft>(key: K, value: ProductDraft[K]) =>
    onChange({ ...draft, [key]: value });

  const pack = packContextFrom(draft);

  return (
    <Stack spacing={3.5}>
      <Grid container spacing={2}>
        {TEXT.map((field) => {
          const errorKey = field.errorKey ?? (field.key as string);
          return (
            <Grid key={field.key as string} size={{ xs: 12, sm: field.span }}>
              <TextField
                label={field.label}
                value={draft[field.key] as string}
                onChange={(e) => set(field.key, e.target.value as never)}
                error={Boolean(errors[errorKey])}
                helperText={errors[errorKey]}
                autoFocus={autoFocus && field.key === 'name'}
                fullWidth
              />
            </Grid>
          );
        })}

        <Grid size={{ xs: 6, sm: 6 }}>
          <TextField
            label="Pack size"
            value={draft.packAmount}
            onChange={(e) => set('packAmount', e.target.value)}
            error={Boolean(errors.package_quantity)}
            helperText={errors.package_quantity}
            inputMode="decimal"
            fullWidth
          />
        </Grid>
        <Grid size={{ xs: 6, sm: 6 }}>
          <UnitSelect
            label="Pack unit"
            value={draft.packUnit}
            onChange={(packUnit) => set('packUnit', packUnit)}
          />
        </Grid>

        {draft.packAmount.trim() ? (
          <Grid size={{ xs: 12, sm: 6 }}>
            <TextField
              label="Servings per pack"
              value={draft.servingsPerPack}
              onChange={(e) => set('servingsPerPack', e.target.value)}
              error={Boolean(errors.servings_per_pack)}
              helperText={errors.servings_per_pack ?? 'Optional'}
              placeholder="—"
              inputMode="numeric"
              fullWidth
            />
          </Grid>
        ) : null}
      </Grid>

      <NutritionFields
        draft={draft.nutrition}
        errors={errors}
        pack={pack}
        onChange={(nutrition) => set('nutrition', nutrition)}
      />
    </Stack>
  );
}
