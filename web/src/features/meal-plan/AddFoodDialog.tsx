import CheckIcon from '@mui/icons-material/CheckCircleOutlineOutlined';
import RadioButtonUncheckedIcon from '@mui/icons-material/RadioButtonUncheckedOutlined';
import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Divider from '@mui/material/Divider';
import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState, type FormEvent } from 'react';
import { ApiError, type Amount, type MealPlanEntry, type MealSlot, type Product } from '../../api/client';
import { useCreateConsumption, useCreateMealPlanEntry, useUpdateMealPlanEntry } from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';
import {
  AmountFields,
  amountDraftFrom,
  draftToAmount,
  validateAmountDraft,
  type AmountDraft,
} from './AmountFields';
import { combineDateTime, formatFullDate } from './date';
import { formatAmount } from './format';
import { ProductPicker } from './ProductPicker';
import { labelForSlot, SLOTS } from './slots';

type AddKind = 'planned' | 'eaten';

type EntryDraft = {
  kind: AddKind;
  product: Product | null;
  amount: AmountDraft;
  time: string;
  slot: MealSlot;
};

type AddedItem = {
  key: string;
  kind: AddKind;
  productId: string;
  productName: string;
  amount: Amount;
};

function emptyDraft(kind: AddKind, slot: MealSlot): EntryDraft {
  return {
    kind,
    product: null,
    amount: amountDraftFrom(null),
    time: '',
    slot,
  };
}

function validate(draft: EntryDraft): Record<string, string> {
  const errors = validateAmountDraft(draft.amount);
  if (!draft.product) errors.product_id = 'Pick a product';
  return errors;
}

function flattenComponentErrors(fields: Record<string, string>): Record<string, string> {
  return Object.fromEntries(
    Object.entries(fields).map(([field, message]) => [
      field.replace(/^components\.0\./, ''),
      message,
    ]),
  );
}

export function AddFoodDialog({
  open,
  onClose,
  memberId,
  date,
  slot,
  kind = 'eaten',
  entry = null,
}: {
  open: boolean;
  onClose: () => void;
  memberId: string;
  date: string;
  slot: MealSlot;
  kind?: AddKind;
  entry?: MealPlanEntry | null;
}) {
  const createConsumption = useCreateConsumption();
  const createPlan = useCreateMealPlanEntry();
  const updatePlan = useUpdateMealPlanEntry();
  const [draft, setDraft] = useState(() => emptyDraft(kind, slot));
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [failure, setFailure] = useState<string | null>(null);
  const [added, setAdded] = useState<AddedItem[]>([]);
  const busy = createConsumption.isPending || createPlan.isPending || updatePlan.isPending;
  const slotLocked = Boolean(entry) || added.length > 0;

  function resetAndClose() {
    setDraft(emptyDraft(kind, slot));
    setErrors({});
    setFailure(null);
    setAdded([]);
    onClose();
  }

  function handleClose() {
    if (!busy) resetAndClose();
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    const found = validate(draft);
    setErrors(found);
    if (Object.keys(found).length > 0) return;

    const amount = draftToAmount(draft.amount);
    if (!draft.product || !amount) return;

    try {
      if (draft.kind === 'eaten') {
        await createConsumption.mutateAsync({
          member_id: memberId,
          product_id: draft.product.id,
          slot: draft.slot,
          amount,
          consumed_on: date,
          consumed_at: draft.time ? combineDateTime(date, draft.time) : null,
        });
      }
      setAdded([
        ...added,
        {
          key: crypto.randomUUID(),
          kind: draft.kind,
          productId: draft.product.id,
          productName: draft.product.name,
          amount,
        },
      ]);
      setDraft(emptyDraft(draft.kind, draft.slot));
      setErrors({});
    } catch (caught) {
      if (caught instanceof ApiError) {
        const fields = flattenComponentErrors(caught.fieldErrors);
        if (Object.keys(fields).length > 0) setErrors(fields);
        else setFailure(caught.message);
      } else {
        setFailure(draft.kind === 'planned' ? 'Could not add to the plan.' : 'Could not log the food.');
      }
    }
  }

  async function finishPlan() {
    if (kind !== 'planned' || added.length === 0) {
      handleClose();
      return;
    }
    setFailure(null);
    try {
      const components = added.map((item) => ({ product_id: item.productId, amount: item.amount }));
      if (entry) {
        await updatePlan.mutateAsync({
          id: entry.id,
          revision: entry.revision,
          body: {
            components: [
              ...entry.components.map((component) => ({
                id: component.id,
                product_id: component.product_id,
                amount: component.amount,
              })),
              ...components,
            ],
          },
        });
      } else {
        await createPlan.mutateAsync({
          planned_on: date,
          slot: draft.slot,
          components,
        });
      }
      resetAndClose();
    } catch (caught) {
      setFailure(caught instanceof ApiError ? caught.message : 'Could not add to the plan.');
    }
  }

  return (
    <FormDialog open={open} onClose={handleClose} maxWidth="sm" fullWidth>
      <form onSubmit={onSubmit}>
        <DialogTitle sx={{ pb: 0.75 }}>{kind === 'planned' ? 'Add planned meal' : 'Add food'}</DialogTitle>
        <DialogContent>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2.5 }}>
            {slotLocked ? `${labelForSlot(draft.slot)} · ` : ''}{formatFullDate(date)}
          </Typography>
          <Stack spacing={3}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}

            {slotLocked ? null : (
              <TextField
                select
                label="Meal"
                value={draft.slot}
                onChange={(event) => setDraft({ ...draft, slot: event.target.value as MealSlot })}
              >
                {SLOTS.map((candidate) => (
                  <MenuItem key={candidate.value} value={candidate.value}>
                    {candidate.label}
                  </MenuItem>
                ))}
              </TextField>
            )}

            {added.length > 0 ? (
              <Stack spacing={0.5} divider={<Divider />}>
                {added.map((item) => (
                  <Stack
                    key={item.key}
                    direction="row"
                    spacing={1}
                    sx={{ alignItems: 'center', py: 0.75 }}
                  >
                    {item.kind === 'eaten' ? (
                      <CheckIcon fontSize="small" color="success" />
                    ) : (
                      <RadioButtonUncheckedIcon fontSize="small" color="disabled" />
                    )}
                    <Typography variant="body2" sx={{ flexGrow: 1 }}>
                      {item.productName}
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
                      {formatAmount(item.amount)}
                    </Typography>
                  </Stack>
                ))}
              </Stack>
            ) : null}

            <ProductPicker
              key={added.length}
              value={draft.product}
              onChange={(product) =>
                setDraft({ ...draft, product, amount: amountDraftFrom(product) })
              }
              error={Boolean(errors.product_id)}
              helperText={errors.product_id}
              autoFocus
            />

            {draft.kind === 'eaten' ? (
              <TextField
                type="time"
                label="Time eaten (optional)"
                value={draft.time}
                onChange={(event) => setDraft({ ...draft, time: event.target.value })}
                slotProps={{ inputLabel: { shrink: true } }}
              />
            ) : null}

            {draft.product ? (
              <AmountFields
                product={draft.product}
                draft={draft.amount}
                errors={errors}
                onChange={(amount) => setDraft({ ...draft, amount })}
              />
            ) : null}
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={kind === 'planned' && added.length > 0 ? finishPlan : handleClose} disabled={busy}>
            {kind === 'planned' && added.length > 0 ? 'Save meal' : added.length > 0 ? 'Done' : 'Cancel'}
          </Button>
          <Button type="submit" variant="contained" disabled={busy || !draft.product}>
            {busy ? 'Adding…' : draft.kind === 'planned' ? 'Add to meal' : 'Add'}
          </Button>
        </DialogActions>
      </form>
    </FormDialog>
  );
}
