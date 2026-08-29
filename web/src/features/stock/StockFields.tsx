import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import type { Product, StorageLocation, TrackingMode, Unit } from '../../api/client';
import type { components } from '../../api/schema';
import { UnitSelect } from '../../components/UnitSelect';
import { ProductPicker } from '../meal-plan/ProductPicker';

export type StockDraft = {
  product: Product | null;
  trackingMode: TrackingMode;
  unit: Unit | '';
  quantity: string;
  low: string;
  high: string;
  storageLocation: StorageLocation;
  note: string;
};

export function emptyStockDraft(): StockDraft {
  return {
    product: null,
    trackingMode: 'exact',
    unit: 'g',
    quantity: '',
    low: '',
    high: '',
    storageLocation: 'chilled',
    note: '',
  };
}

const STORAGE_LOCATIONS: { value: StorageLocation; label: string }[] = [
  { value: 'ambient', label: 'Ambient' },
  { value: 'chilled', label: 'Chilled' },
  { value: 'frozen', label: 'Frozen' },
];

const TRACKING_MODES: { value: TrackingMode; label: string; hint: string }[] = [
  { value: 'exact', label: 'Exact', hint: 'You track the remaining amount.' },
  { value: 'estimated', label: 'Estimated', hint: 'You only know it roughly, as a range.' },
  { value: 'not_tracked', label: 'Not tracked', hint: 'A staple you always keep in; assumed available.' },
];

export function validateStockDraft(draft: StockDraft): Record<string, string> {
  const errors: Record<string, string> = {};
  if (!draft.product) errors.product = 'Pick a product';
  if (draft.trackingMode === 'exact') {
    if (!draft.quantity.trim() || Number(draft.quantity) < 0) errors.quantity = 'Enter an amount';
    if (!draft.unit) errors.unit = 'Pick a unit';
  }
  if (draft.trackingMode === 'estimated') {
    if (!draft.low.trim() || Number(draft.low) < 0) errors.low = 'Enter a low bound';
    if (!draft.high.trim() || Number(draft.high) < 0) errors.high = 'Enter a high bound';
    if (draft.low.trim() && draft.high.trim() && Number(draft.low) > Number(draft.high)) {
      errors.high = 'The high bound cannot be below the low bound';
    }
    if (!draft.unit) errors.unit = 'Pick a unit';
  }
  return errors;
}

export function draftToLevel(draft: StockDraft): components['schemas']['StockLevelDto'] {
  if (draft.trackingMode === 'not_tracked') return { mode: 'not_tracked' };
  if (draft.trackingMode === 'estimated') {
    return {
      mode: 'estimated',
      low: Number(draft.low),
      high: Number(draft.high),
      unit: draft.unit as Unit,
    };
  }
  return {
    mode: 'exact',
    quantity: { amount: Number(draft.quantity), unit: draft.unit as Unit },
  };
}

export function StockFields({
  draft,
  onChange,
  errors,
  lockProduct,
}: {
  draft: StockDraft;
  onChange: (next: StockDraft) => void;
  errors: Record<string, string>;
  lockProduct?: boolean;
}) {
  const set = <K extends keyof StockDraft>(key: K, value: StockDraft[K]) =>
    onChange({ ...draft, [key]: value });

  return (
    <Stack spacing={2.5}>
      {!lockProduct && (
        <ProductPicker
          value={draft.product}
          onChange={(next) => set('product', next)}
          error={Boolean(errors.product)}
          helperText={errors.product}
        />
      )}

      <TextField
        select
        label="Tracking"
        value={draft.trackingMode}
        onChange={(event) => set('trackingMode', event.target.value as TrackingMode)}
        helperText={TRACKING_MODES.find((m) => m.value === draft.trackingMode)?.hint}
      >
        {TRACKING_MODES.map((mode) => (
          <MenuItem key={mode.value} value={mode.value}>
            {mode.label}
          </MenuItem>
        ))}
      </TextField>

      {draft.trackingMode === 'exact' && (
        <Stack direction="row" spacing={2}>
          <TextField
            label="Amount"
            type="number"
            value={draft.quantity}
            onChange={(event) => set('quantity', event.target.value)}
            error={Boolean(errors.quantity)}
            helperText={errors.quantity}
            sx={{ flexGrow: 1 }}
          />
          <UnitSelect
            label="Unit"
            value={draft.unit}
            onChange={(next) => set('unit', next)}
            error={Boolean(errors.unit)}
            helperText={errors.unit}
            sx={{ width: 200 }}
          />
        </Stack>
      )}

      {draft.trackingMode === 'estimated' && (
        <Stack direction="row" spacing={2}>
          <TextField
            label="At least"
            type="number"
            value={draft.low}
            onChange={(event) => set('low', event.target.value)}
            error={Boolean(errors.low)}
            helperText={errors.low}
          />
          <TextField
            label="At most"
            type="number"
            value={draft.high}
            onChange={(event) => set('high', event.target.value)}
            error={Boolean(errors.high)}
            helperText={errors.high}
          />
          <UnitSelect
            label="Unit"
            value={draft.unit}
            onChange={(next) => set('unit', next)}
            error={Boolean(errors.unit)}
            helperText={errors.unit}
            sx={{ width: 180 }}
          />
        </Stack>
      )}

      <TextField
        select
        label="Storage"
        value={draft.storageLocation}
        onChange={(event) => set('storageLocation', event.target.value as StorageLocation)}
      >
        {STORAGE_LOCATIONS.map((location) => (
          <MenuItem key={location.value} value={location.value}>
            {location.label}
          </MenuItem>
        ))}
      </TextField>

      <TextField
        label="Note"
        value={draft.note}
        onChange={(event) => set('note', event.target.value)}
        placeholder="Where it is, what it's for"
      />
    </Stack>
  );
}
