import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import { useState, type FormEvent } from 'react';
import { ApiError } from '../../api/client';
import type { DiaryEntry } from '../../api/client';
import { useDeleteConsumption, useProduct, useProductNutrition, useUpdateConsumption } from '../../api/queries';
import { ConflictDialog } from '../../components/ConflictDialog';
import { FormDialog } from '../../components/FormDialog';
import { Loading } from '../../components/States';
import { useDebounced } from '../../hooks/useDebounced';
import {
  AmountFields,
  draftToAmount,
  validateAmountDraft,
  type AmountDraft,
} from './AmountFields';
import { extractTime, combineDateTime } from './date';
import { NutritionPreview } from './NutritionPreview';

type EditDraft = {
  amount: AmountDraft;
  date: string;
  time: string;
};

function draftFrom(record: DiaryEntry): EditDraft {
  return {
    amount:
      record.amount.kind === 'measure'
        ? { kind: 'measure', value: String(record.amount.value), unit: record.amount.unit }
        : { kind: record.amount.kind, value: String(record.amount.value), unit: 'g' },
    date: record.consumed_on,
    time: extractTime(record.consumed_at),
  };
}

export function EditEntryDialog({
  open,
  onClose,
  record,
}: {
  open: boolean;
  onClose: () => void;
  record: DiaryEntry;
}) {
  const product = useProduct(record.product_id);
  const update = useUpdateConsumption();
  const del = useDeleteConsumption();
  const [draft, setDraft] = useState<EditDraft>(draftFrom(record));
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [failure, setFailure] = useState<string | null>(null);
  const [conflict, setConflict] = useState<ApiError | null>(null);

  const debouncedValue = useDebounced(draft.amount.value, 300);
  const previewAmount = draftToAmount({ ...draft.amount, value: debouncedValue });
  const preview = useProductNutrition(record.product_id, previewAmount);

  function handleClose() {
    setErrors({});
    setFailure(null);
    setConflict(null);
    onClose();
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    const found = validateAmountDraft(draft.amount);
    setErrors(found);
    if (Object.keys(found).length > 0) return;

    const amount = draftToAmount(draft.amount);
    if (!amount) return;

    try {
      await update.mutateAsync({
        id: record.id,
        revision: record.revision,
        body: {
          amount,
          consumed_on: draft.date,
          consumed_at: combineDateTime(draft.date, draft.time),
        },
      });
      handleClose();
    } catch (caught) {
      if (caught instanceof ApiError) {
        if (caught.isConflict) setConflict(caught);
        else {
          const fields = caught.fieldErrors;
          if (Object.keys(fields).length > 0) setErrors(fields);
          else setFailure(caught.message);
        }
      } else setFailure('Could not save.');
    }
  }

  async function onDelete() {
    try {
      await del.mutateAsync({ id: record.id, revision: record.revision, memberId: record.member_id });
      handleClose();
    } catch (caught) {
      if (caught instanceof ApiError && caught.isConflict) setConflict(caught);
      else setFailure('Could not delete.');
    }
  }

  return (
    <FormDialog open={open} onClose={handleClose} maxWidth="sm" fullWidth>
      <form onSubmit={onSubmit}>
        <DialogTitle>{record.product_name}</DialogTitle>
        <DialogContent>
          <Stack spacing={3} sx={{ pt: 0.5 }}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}

            {product.isLoading ? (
              <Loading label="Loading product" />
            ) : (
              <>
                <AmountFields
                  product={product.data ?? null}
                  draft={draft.amount}
                  errors={errors}
                  onChange={(amount) => setDraft({ ...draft, amount })}
                />

                <Stack direction="row" spacing={2}>
                  <TextField
                    type="date"
                    label="Day"
                    value={draft.date}
                    onChange={(e) => e.target.value && setDraft({ ...draft, date: e.target.value })}
                    fullWidth
                  />
                  <TextField
                    type="time"
                    label="Time eaten"
                    value={draft.time}
                    onChange={(e) => e.target.value && setDraft({ ...draft, time: e.target.value })}
                    fullWidth
                  />
                </Stack>

                {preview.data ? <NutritionPreview nutrition={preview.data.nutrition} /> : null}
              </>
            )}
          </Stack>
        </DialogContent>
        <DialogActions sx={{ justifyContent: 'space-between', px: 3, pb: 2 }}>
          <Button color="error" onClick={onDelete} disabled={del.isPending}>
            {del.isPending ? 'Removing…' : 'Remove'}
          </Button>
          <Stack direction="row" spacing={1}>
            <Button onClick={handleClose}>Cancel</Button>
            <Button type="submit" variant="contained" disabled={update.isPending}>
              {update.isPending ? 'Saving…' : 'Save changes'}
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
