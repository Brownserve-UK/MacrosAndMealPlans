import Grid from '@mui/material/Grid';
import Link from '@mui/material/Link';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import type { Nutrition, Unit } from '../api/client';
import { displayUnit, UnitSelect } from './UnitSelect';

type Field = { key: keyof Nutrition; label: string; unit: string };

const ESSENTIAL: Field[] = [
  { key: 'energy_kcal', label: 'Calories', unit: 'kcal' },
  { key: 'protein_g', label: 'Protein', unit: 'g' },
  { key: 'carbohydrate_g', label: 'Carbs', unit: 'g' },
  { key: 'fat_g', label: 'Fat', unit: 'g' },
];

const DETAILED: Field[] = [
  { key: 'sugar_g', label: 'Sugars', unit: 'g' },
  { key: 'saturated_fat_g', label: 'Saturates', unit: 'g' },
  { key: 'fibre_g', label: 'Fibre', unit: 'g' },
  { key: 'salt_g', label: 'Salt', unit: 'g' },
  { key: 'cholesterol_mg', label: 'Cholesterol', unit: 'mg' },
];

const FIELDS: Field[] = [...ESSENTIAL, ...DETAILED];

export type NutritionDraft = {
  [nutrient: string]: string | Unit | boolean | undefined;
  basisAmount?: string;
  basisUnit?: Unit;
  basisCustom?: boolean;
};

export type PackContext = {
  quantity?: { amount: number; unit: Unit };
  servings?: number;
};

const MASS_UNITS: readonly Unit[] = ['mg', 'g', 'kg', 'oz', 'lb'];
const VOLUME_UNITS: readonly Unit[] = ['ml', 'l', 'tsp', 'tbsp', 'fl_oz', 'cup'];

export function dimensionOf(unit: Unit): 'mass' | 'volume' | 'count' {
  if (MASS_UNITS.includes(unit)) return 'mass';
  if (VOLUME_UNITS.includes(unit)) return 'volume';
  return 'count';
}

export function derivedBasis(pack: PackContext): { amount: number; unit: Unit } {
  if (pack.quantity && pack.servings) {
    return { amount: pack.quantity.amount / pack.servings, unit: pack.quantity.unit };
  }
  const dimension = pack.quantity ? dimensionOf(pack.quantity.unit) : 'mass';
  if (dimension === 'volume') return { amount: 100, unit: 'ml' as Unit };
  if (dimension === 'count') return { amount: 1, unit: pack.quantity!.unit };
  return { amount: 100, unit: 'g' as Unit };
}

export function isBasisEditable(pack: PackContext): boolean {
  if (pack.servings) return false;
  const dimension = pack.quantity ? dimensionOf(pack.quantity.unit) : 'mass';
  return dimension !== 'count';
}

export function nutritionToDraft(
  nutrition: Nutrition | undefined,
  pack: PackContext,
): NutritionDraft {
  const derived = derivedBasis(pack);
  const editable = isBasisEditable(pack);
  const stored = nutrition?.basis;
  const custom = stored
    ? editable
      ? stored.unit !== derived.unit
      : Number(stored.amount) !== derived.amount || stored.unit !== derived.unit
    : false;

  const draft: NutritionDraft = {
    basisAmount: stored ? String(stored.amount) : String(derived.amount),
    basisUnit: stored?.unit ?? derived.unit,
    basisCustom: custom,
  };
  for (const { key } of FIELDS) {
    const value = nutrition?.[key];
    draft[key as string] = value === null || value === undefined ? '' : String(value);
  }
  return draft;
}

export function draftToNutrition(draft: NutritionDraft, pack: PackContext): Nutrition {
  const nutrition: Record<string, unknown> = {};
  const derived = derivedBasis(pack);

  if (draft.basisCustom) {
    const amount = String(draft.basisAmount ?? '').trim();
    nutrition.basis =
      amount && draft.basisUnit ? { amount: Number(amount), unit: draft.basisUnit } : null;
  } else if (isBasisEditable(pack)) {
    const amount = String(draft.basisAmount ?? '').trim();
    nutrition.basis = amount ? { amount: Number(amount), unit: derived.unit } : null;
  } else {
    nutrition.basis = derived;
  }

  for (const { key } of FIELDS) {
    const raw = String(draft[key as string] ?? '').trim();
    nutrition[key as string] = raw === '' ? null : Number(raw);
  }
  return nutrition as Nutrition;
}

export function validateNutritionDraft(
  draft: NutritionDraft,
  pack: PackContext,
): Record<string, string> {
  const errors: Record<string, string> = {};
  let anyValue = false;

  for (const { key } of FIELDS) {
    const raw = String(draft[key as string] ?? '').trim();
    if (raw === '') continue;
    anyValue = true;
    const parsed = Number(raw);
    if (Number.isNaN(parsed)) errors[key as string] = 'Must be a number';
    else if (parsed < 0) errors[key as string] = 'Must not be negative';
  }

  if (draft.basisCustom || isBasisEditable(pack)) {
    const amount = String(draft.basisAmount ?? '').trim();
    if (anyValue && (!amount || (draft.basisCustom && !draft.basisUnit))) {
      errors.basis = 'Needed';
    } else if (amount && Number(amount) <= 0) {
      errors.basis = 'Must be more than zero';
    }
  }
  return errors;
}

function NumberField({
  field,
  draft,
  errors,
  set,
}: {
  field: Field;
  draft: NutritionDraft;
  errors: Record<string, string>;
  set: (key: string, value: string) => void;
}) {
  const key = field.key as string;
  return (
    <TextField
      label={field.label}
      value={String(draft[key] ?? '')}
      onChange={(e) => set(key, e.target.value)}
      error={Boolean(errors[key])}
      helperText={errors[key]}
      placeholder="—"
      inputMode="decimal"
      slotProps={{
        input: {
          endAdornment: (
            <Typography variant="caption" color="text.secondary">
              {field.unit}
            </Typography>
          ),
        },
      }}
      fullWidth
    />
  );
}

function BasisRow({
  draft,
  errors,
  pack,
  onChange,
}: {
  draft: NutritionDraft;
  errors: Record<string, string>;
  pack: PackContext;
  onChange: (next: NutritionDraft) => void;
}) {
  const derived = derivedBasis(pack);

  if (draft.basisCustom) {
    return (
      <Stack spacing={1}>
        <Typography variant="body2" color="text.secondary">
          Custom basis
        </Typography>
        <Grid container spacing={2}>
          <Grid size={{ xs: 6, sm: 4 }}>
            <TextField
              label="Amount"
              value={draft.basisAmount ?? ''}
              onChange={(e) => onChange({ ...draft, basisAmount: e.target.value })}
              error={Boolean(errors.basis)}
              helperText={errors.basis}
              inputMode="decimal"
              fullWidth
            />
          </Grid>
          <Grid size={{ xs: 6, sm: 4 }}>
            <UnitSelect
              label="Unit"
              value={draft.basisUnit ?? ''}
              onChange={(basisUnit) => onChange({ ...draft, basisUnit })}
              error={Boolean(errors.basis)}
              helperText={errors.basis}
            />
          </Grid>
        </Grid>
      </Stack>
    );
  }

  if (isBasisEditable(pack)) {
    return (
      <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
        <Typography variant="body2" color="text.secondary">
          Nutrition per
        </Typography>
        <TextField
          value={draft.basisAmount ?? ''}
          onChange={(e) => onChange({ ...draft, basisAmount: e.target.value })}
          error={Boolean(errors.basis)}
          helperText={errors.basis}
          inputMode="decimal"
          sx={{ width: 88 }}
          slotProps={{
            input: {
              endAdornment: (
                <Typography variant="caption" color="text.secondary">
                  {displayUnit(derived.unit)}
                </Typography>
              ),
            },
          }}
        />
        <Link
          component="button"
          type="button"
          variant="caption"
          color="text.secondary"
          underline="hover"
          onClick={() =>
            onChange({
              ...draft,
              basisCustom: true,
              basisAmount: String(derived.amount),
              basisUnit: derived.unit,
            })
          }
        >
          use a different unit
        </Link>
      </Stack>
    );
  }

  return (
    <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
      <Typography variant="body2" color="text.secondary">
        Nutrition per {pack.servings ? 'serving' : 'item'}
      </Typography>
      <Link
        component="button"
        type="button"
        variant="caption"
        color="text.secondary"
        underline="hover"
        onClick={() =>
          onChange({
            ...draft,
            basisCustom: true,
            basisAmount: String(derived.amount),
            basisUnit: derived.unit,
          })
        }
      >
        use a different amount
      </Link>
    </Stack>
  );
}

export function NutritionFields({
  draft,
  errors,
  pack,
  onChange,
}: {
  draft: NutritionDraft;
  errors: Record<string, string>;
  pack: PackContext;
  onChange: (next: NutritionDraft) => void;
}) {
  const set = (key: string, value: string) => onChange({ ...draft, [key]: value });

  return (
    <Stack spacing={2}>
      <Typography variant="h3">Nutrition</Typography>

      <BasisRow draft={draft} errors={errors} pack={pack} onChange={onChange} />

      <Grid container spacing={2}>
        {ESSENTIAL.map((field) => (
          <Grid key={field.key as string} size={{ xs: 6, sm: 3 }}>
            <NumberField field={field} draft={draft} errors={errors} set={set} />
          </Grid>
        ))}
      </Grid>

      <Grid container spacing={2}>
        {DETAILED.map((field) => (
          <Grid key={field.key as string} size={{ xs: 6, sm: 4 }}>
            <NumberField field={field} draft={draft} errors={errors} set={set} />
          </Grid>
        ))}
      </Grid>
    </Stack>
  );
}
