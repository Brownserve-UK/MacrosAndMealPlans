import Grid from '@mui/material/Grid';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import { useEffect, useMemo } from 'react';
import type { Amount, Product, Unit, UnitInfo } from '../../api/client';
import { useUnits } from '../../api/queries';
import { displayUnit } from '../../components/UnitSelect';

export type AmountKind = Amount['kind'];

export type AmountDraft = {
  kind: AmountKind;
  value: string;
  unit: Unit;
};

export type AmountProductInfo = Pick<Product, 'package_quantity' | 'servings_per_pack'> & {
  nutrition?: Product['nutrition'];
};

export type AmountOption = {
  key: string;
  label: string;
  kind: AmountKind;
  unit?: Unit;
};

const DIMENSION_ORDER: Record<string, number> = { mass: 0, volume: 1, count: 2 };

function referenceUnit(product: AmountProductInfo | null): Unit | undefined {
  return product?.nutrition?.basis?.unit ?? product?.package_quantity?.unit;
}

function formatNumber(value: number): string {
  return value.toLocaleString('en-GB', { maximumFractionDigits: 2 });
}

export function amountOptionsFor(
  product: AmountProductInfo | null,
  units: UnitInfo[],
): AmountOption[] {
  const reference = referenceUnit(product);
  const referenceInfo = units.find((u) => u.code === reference);

  let measures: UnitInfo[];
  if (!referenceInfo) {
    measures = [...units].sort(
      (a, b) => (DIMENSION_ORDER[a.dimension] ?? 99) - (DIMENSION_ORDER[b.dimension] ?? 99),
    );
  } else if (referenceInfo.convertible) {
    measures = units.filter((u) => u.dimension === referenceInfo.dimension && u.convertible);
  } else {
    measures = [referenceInfo];
  }

  const relative: AmountOption[] = [];
  if (product?.package_quantity && product.servings_per_pack) {
    relative.push({ key: 'servings', label: 'serving', kind: 'servings' });
  }
  if (product?.package_quantity) {
    relative.push({ key: 'packs', label: 'pack', kind: 'packs' });
  }

  const claimed = new Set(relative.map((option) => option.label));
  const measured: AmountOption[] = measures
    .filter((u) => !claimed.has(displayUnit(u.code)))
    .map((u) => ({
      key: `measure:${u.code}`,
      label: displayUnit(u.code),
      kind: 'measure',
      unit: u.code as Unit,
    }));

  return [...measured, ...relative];
}

export function optionKeyFor(draft: AmountDraft): string {
  return draft.kind === 'measure' ? `measure:${draft.unit}` : draft.kind;
}

export function applyOptionKey(draft: AmountDraft, key: string): AmountDraft {
  if (key === 'servings' || key === 'packs') return { ...draft, kind: key };
  return { ...draft, kind: 'measure', unit: key.slice('measure:'.length) as Unit };
}

export function amountDraftFrom(product: AmountProductInfo | null): AmountDraft {
  return { kind: 'measure', value: '', unit: referenceUnit(product) ?? 'g' };
}

export function draftToAmount(draft: AmountDraft): Amount | null {
  const raw = draft.value.trim();
  if (!raw) return null;
  const value = Number(raw);
  if (Number.isNaN(value)) return null;
  if (draft.kind === 'measure') return { kind: 'measure', value, unit: draft.unit };
  return { kind: draft.kind, value };
}

export function validateAmountDraft(draft: AmountDraft): Record<string, string> {
  const errors: Record<string, string> = {};
  const raw = draft.value.trim();
  if (!raw) {
    errors.amount = 'Required';
  } else {
    const value = Number(raw);
    if (Number.isNaN(value)) errors.amount = 'Must be a number';
    else if (value <= 0) errors.amount = 'Must be more than zero';
  }
  return errors;
}

export function unitHint(product: AmountProductInfo | null, draft: AmountDraft): string | null {
  const pack = product?.package_quantity;
  if (!pack) return null;
  const unit = displayUnit(pack.unit);

  if (draft.kind === 'packs') return `1 pack is ${formatNumber(pack.amount)} ${unit}`;
  if (draft.kind === 'servings' && product?.servings_per_pack) {
    const perServing = pack.amount / product.servings_per_pack;
    return `1 serving is ${formatNumber(perServing)} ${unit}`;
  }
  return null;
}

export function AmountFields({
  product,
  draft,
  errors,
  onChange,
}: {
  product: AmountProductInfo | null;
  draft: AmountDraft;
  errors: Record<string, string>;
  onChange: (next: AmountDraft) => void;
}) {
  const units = useUnits();
  const available = units.data;
  const options = useMemo(() => amountOptionsFor(product, available ?? []), [product, available]);
  const hint = unitHint(product, draft);
  const disabled = product === null;

  const selectedKey = optionKeyFor(draft);
  const offered = options.some((option) => option.key === selectedKey);

  useEffect(() => {
    const fallback = options[0];
    if (!offered && fallback) onChange(applyOptionKey(draft, fallback.key));
  }, [offered, options, draft, onChange]);

  return (
    <Grid container spacing={2}>
      <Grid size={6}>
        <TextField
          label="Amount"
          value={draft.value}
          onChange={(e) => onChange({ ...draft, value: e.target.value })}
          error={Boolean(errors.amount)}
          helperText={errors.amount}
          inputMode="decimal"
          disabled={disabled}
          fullWidth
        />
      </Grid>
      <Grid size={6}>
        <TextField
          select
          label="Unit"
          value={offered ? selectedKey : ''}
          onChange={(e) => onChange(applyOptionKey(draft, e.target.value))}
          helperText={hint ?? undefined}
          disabled={disabled}
          fullWidth
        >
          {options.map((option) => (
            <MenuItem key={option.key} value={option.key}>
              {option.label}
            </MenuItem>
          ))}
        </TextField>
      </Grid>
    </Grid>
  );
}
