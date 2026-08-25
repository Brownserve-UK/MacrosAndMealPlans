import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState, type FormEvent } from 'react';
import { ApiError } from '../../api/client';
import type { Product } from '../../api/client';
import { useCreateConsumption } from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';
import {
  AmountFields,
  amountDraftFrom,
  draftToAmount,
  validateAmountDraft,
  type AmountDraft,
} from './AmountFields';
import { ProductPicker } from './ProductPicker';
import { nowTime, combineDateTime, formatFullDate } from './date';

type EntryDraft = {
  product: Product | null;
  amount: AmountDraft;
  time: string;
};

function emptyDraft(): EntryDraft {
  return { product: null, amount: amountDraftFrom(null), time: nowTime() };
}

function validate(draft: EntryDraft): Record<string, string> {
  const errors = validateAmountDraft(draft.amount);
  if (!draft.product) errors.product_id = 'Pick a product';
  return errors;
}

export function LogFoodDialog({
  open,
  onClose,
  memberId,
  date,
}: {
  open: boolean;
  onClose: () => void;
  memberId: string;
  date: string;
}) {
  const create = useCreateConsumption();
  const [draft, setDraft] = useState<EntryDraft>(emptyDraft());
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

    const amount = draftToAmount(draft.amount);
    if (!draft.product || !amount) return;

    try {
      await create.mutateAsync({
        member_id: memberId,
        product_id: draft.product.id,
        amount,
        consumed_on: date,
        consumed_at: combineDateTime(date, draft.time),
      });
      handleClose();
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
        <DialogTitle sx={{ pb: 0.75 }}>Log food</DialogTitle>
        <DialogContent>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2.5 }}>
            {formatFullDate(date)}
          </Typography>
          <Stack spacing={3}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}

            <ProductPicker
              value={draft.product}
              onChange={(product) =>
                setDraft({ ...draft, product, amount: amountDraftFrom(product) })
              }
              error={Boolean(errors.product_id)}
              helperText={errors.product_id}
              autoFocus
            />

            {draft.product ? (
              <>
                <AmountFields
                  product={draft.product}
                  draft={draft.amount}
                  errors={errors}
                  onChange={(amount) => setDraft({ ...draft, amount })}
                />

                <TextField
                  type="time"
                  label="Time eaten"
                  value={draft.time}
                  onChange={(e) => e.target.value && setDraft({ ...draft, time: e.target.value })}
                />
              </>
            ) : null}
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={handleClose}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={create.isPending || !draft.product}>
            {create.isPending ? 'Saving…' : 'Log food'}
          </Button>
        </DialogActions>
      </form>
    </FormDialog>
  );
}
