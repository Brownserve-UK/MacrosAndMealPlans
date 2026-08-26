import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState, type FormEvent } from 'react';
import { ApiError, type MealItem, type MealSlot, type Product } from '../../api/client';
import {
  useDeleteConsumption,
  useDeleteMealPlanEntry,
  useMarkMealPlanNotEaten,
  useProduct,
  useReopenMealPlanEntry,
  useUpdateConsumption,
  useUpdateMealPlanEntry,
} from '../../api/queries';
import { ConflictDialog } from '../../components/ConflictDialog';
import { FormDialog } from '../../components/FormDialog';
import { Loading } from '../../components/States';
import {
  AmountFields,
  amountDraftFrom,
  draftToAmount,
  validateAmountDraft,
  type AmountDraft,
} from './AmountFields';
import { combineDateTime, nowTime } from './date';
import { ProductPicker } from './ProductPicker';
import { SLOTS } from './slots';

function amountToDraft(item: MealItem): AmountDraft {
  const amount = item.amount;
  return amount.kind === 'measure'
    ? { kind: 'measure', value: String(amount.value), unit: amount.unit }
    : { kind: amount.kind, value: String(amount.value), unit: 'g' };
}

export function EditFoodDialog({
  open,
  onClose,
  item,
  date,
  slot,
  memberId,
}: {
  open: boolean;
  onClose: () => void;
  item: MealItem;
  date: string;
  slot: MealSlot;
  memberId: string;
}) {
  const product = useProduct(item.product_id);
  const updateConsumption = useUpdateConsumption();
  const deleteConsumption = useDeleteConsumption();
  const updateEntry = useUpdateMealPlanEntry();
  const deleteEntry = useDeleteMealPlanEntry();
  const markNotEaten = useMarkMealPlanNotEaten();
  const reopenEntry = useReopenMealPlanEntry();

  const isLogged = item.kind === 'logged';
  const isPlanned = item.kind === 'planned';
  const linked = item.linked_record_id != null;

  const [draft, setDraft] = useState(() => ({
    product: null as Product | null,
    amount: amountToDraft(item),
    date,
    time: item.at ?? nowTime(),
    slot,
  }));
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [failure, setFailure] = useState<string | null>(null);
  const [conflict, setConflict] = useState<ApiError | null>(null);

  const busy =
    updateConsumption.isPending ||
    deleteConsumption.isPending ||
    updateEntry.isPending ||
    deleteEntry.isPending ||
    markNotEaten.isPending ||
    reopenEntry.isPending;

  function handleClose() {
    if (busy) return;
    setErrors({});
    setFailure(null);
    setConflict(null);
    onClose();
  }

  function report(caught: unknown, fallback: string, flattenComponents = false) {
    if (caught instanceof ApiError) {
      if (caught.isConflict) setConflict(caught);
      else {
        const raw = caught.fieldErrors;
        const fields = flattenComponents
          ? Object.fromEntries(
              Object.entries(raw).map(([field, message]) => [
                field.replace(/^components\.0\./, ''),
                message,
              ]),
            )
          : raw;
        if (Object.keys(fields).length > 0) setErrors(fields);
        else setFailure(caught.message);
      }
    } else setFailure(fallback);
  }

  async function saveLinked(event: FormEvent) {
    event.preventDefault();
    if (!item.linked_record_id) return;
    setFailure(null);
    const found = validateAmountDraft(draft.amount);
    setErrors(found);
    if (Object.keys(found).length > 0) return;
    const amount = draftToAmount(draft.amount);
    if (!amount) return;

    try {
      await updateConsumption.mutateAsync({
        id: item.linked_record_id,
        revision: item.revision,
        body: {
          amount,
          consumed_on: draft.date,
          consumed_at: combineDateTime(draft.date, draft.time || nowTime()),
          ...(isLogged ? { slot: draft.slot } : {}),
        },
      });
      handleClose();
    } catch (caught) {
      report(caught, 'Could not save.');
    }
  }

  async function savePlanned(event: FormEvent) {
    event.preventDefault();
    if (item.kind !== 'planned') return;
    setFailure(null);
    const chosenProduct = draft.product ?? product.data ?? null;
    const found = validateAmountDraft(draft.amount);
    if (!chosenProduct) found.product_id = 'Pick a product';
    setErrors(found);
    if (Object.keys(found).length > 0) return;
    const amount = draftToAmount(draft.amount);
    if (!amount || !chosenProduct) return;

    try {
      await updateEntry.mutateAsync({
        id: item.entry_id,
        revision: item.revision,
        body: {
          planned_on: draft.date,
          planned_time: draft.time || null,
          slot: draft.slot,
          components: [{ product_id: chosenProduct.id, amount }],
        },
      });
      handleClose();
    } catch (caught) {
      report(caught, 'Could not save.', true);
    }
  }

  async function onRemove() {
    try {
      if (item.kind === 'logged') {
        await deleteConsumption.mutateAsync({
          id: item.record_id,
          revision: item.revision,
          memberId,
        });
      } else {
        await deleteEntry.mutateAsync({ id: item.entry_id, revision: item.revision });
      }
      handleClose();
    } catch (caught) {
      report(caught, 'Could not remove.');
    }
  }

  async function onNotEaten() {
    if (item.kind !== 'planned') return;
    try {
      await markNotEaten.mutateAsync({ id: item.entry_id, revision: item.revision });
      handleClose();
    } catch (caught) {
      report(caught, 'Could not update.');
    }
  }

  async function onUndo() {
    if (item.kind !== 'planned') return;
    try {
      await reopenEntry.mutateAsync({ id: item.entry_id, revision: item.revision });
      handleClose();
    } catch (caught) {
      report(caught, 'Could not undo.');
    }
  }

  if (item.status === 'not_eaten' && item.kind === 'planned') {
    return (
      <FormDialog open={open} onClose={handleClose} maxWidth="sm" fullWidth>
        <DialogTitle>{item.product_name}</DialogTitle>
        <DialogContent>
          <Typography color="text.secondary">Marked not eaten.</Typography>
        </DialogContent>
        <DialogActions sx={{ justifyContent: 'space-between', px: 3, pb: 2 }}>
          <Button onClick={onUndo} disabled={busy}>
            {busy ? 'Reopening…' : 'Reopen to plan'}
          </Button>
          <Button onClick={handleClose}>Close</Button>
        </DialogActions>
      </FormDialog>
    );
  }

  const onSubmit = linked ? saveLinked : savePlanned;

  return (
    <FormDialog open={open} onClose={handleClose} maxWidth="sm" fullWidth>
      <form onSubmit={onSubmit}>
        <DialogTitle>{item.product_name}</DialogTitle>
        <DialogContent>
          <Stack spacing={3} sx={{ pt: 0.5 }}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}

            {isPlanned && !linked ? (
              product.isLoading ? (
                <Loading label="Loading product" />
              ) : (
                <ProductPicker
                  value={draft.product ?? product.data ?? null}
                  onChange={(next) =>
                    setDraft({ ...draft, product: next, amount: amountDraftFrom(next) })
                  }
                  error={Boolean(errors.product_id)}
                  helperText={errors.product_id}
                />
              )
            ) : null}

            {product.isLoading && !isPlanned ? (
              <Loading label="Loading product" />
            ) : (
              <AmountFields
                product={draft.product ?? product.data ?? null}
                draft={draft.amount}
                errors={errors}
                onChange={(amount) => setDraft({ ...draft, amount })}
              />
            )}

            <Stack direction="row" spacing={2}>
              <TextField
                type="date"
                label="Day"
                value={draft.date}
                onChange={(e) => e.target.value && setDraft({ ...draft, date: e.target.value })}
                slotProps={{ inputLabel: { shrink: true } }}
                fullWidth
              />
              <TextField
                type="time"
                label={linked ? 'Time eaten' : 'Time'}
                value={draft.time}
                onChange={(e) => setDraft({ ...draft, time: e.target.value })}
                slotProps={{ inputLabel: { shrink: true } }}
                fullWidth
              />
            </Stack>

            {isLogged || !linked ? (
              <TextField
                select
                label="Meal"
                value={draft.slot}
                onChange={(event) => setDraft({ ...draft, slot: event.target.value as MealSlot })}
                fullWidth
              >
                {SLOTS.map((candidate) => (
                  <MenuItem key={candidate.value} value={candidate.value}>
                    {candidate.label}
                  </MenuItem>
                ))}
              </TextField>
            ) : null}
          </Stack>
        </DialogContent>
        <DialogActions sx={{ justifyContent: 'space-between', px: 3, pb: 2 }}>
          <Stack direction="row" spacing={1}>
            {isPlanned && !linked ? (
              <>
                <Button color="warning" onClick={onNotEaten} disabled={busy}>
                  Not eaten
                </Button>
                <Button color="error" onClick={onRemove} disabled={busy}>
                  Remove
                </Button>
              </>
            ) : isPlanned && linked ? (
              <Button onClick={onUndo} disabled={busy}>
                {busy ? 'Undoing…' : 'Undo'}
              </Button>
            ) : (
              <Button color="error" onClick={onRemove} disabled={busy}>
                {busy ? 'Removing…' : 'Remove'}
              </Button>
            )}
          </Stack>
          <Stack direction="row" spacing={1}>
            <Button onClick={handleClose} disabled={busy}>
              Cancel
            </Button>
            <Button type="submit" variant="contained" disabled={busy}>
              {busy ? 'Saving…' : 'Save changes'}
            </Button>
          </Stack>
        </DialogActions>
      </form>

      <ConflictDialog
        error={conflict}
        onDismiss={() => setConflict(null)}
        onReload={() => {
          setConflict(null);
          handleClose();
        }}
      />
    </FormDialog>
  );
}
