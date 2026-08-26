import CheckIcon from '@mui/icons-material/CheckCircleOutlineOutlined';
import RadioButtonUncheckedIcon from '@mui/icons-material/RadioButtonUncheckedOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Divider from '@mui/material/Divider';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Typography from '@mui/material/Typography';
import { useState, type FormEvent } from 'react';
import { ApiError, type Amount, type MealSlot, type Product } from '../../api/client';
import { useCreateConsumption, useCreateMealPlanEntry } from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';
import {
  AmountFields,
  amountDraftFrom,
  draftToAmount,
  validateAmountDraft,
  type AmountDraft,
} from './AmountFields';
import { combineDateTime, formatFullDate, nowTime, todayIso } from './date';
import { formatAmount } from './format';
import { ProductPicker } from './ProductPicker';
import { labelForSlot } from './slots';

type AddKind = 'planned' | 'eaten';

type EntryDraft = {
  kind: AddKind;
  product: Product | null;
  amount: AmountDraft;
  time: string;
  showTime: boolean;
};

type AddedItem = {
  key: string;
  kind: AddKind;
  productName: string;
  amount: Amount;
};

function allowedKinds(date: string): AddKind[] {
  const today = todayIso();
  if (date > today) return ['planned'];
  if (date < today) return ['eaten'];
  return ['eaten', 'planned'];
}

function emptyDraft(kind: AddKind): EntryDraft {
  return {
    kind,
    product: null,
    amount: amountDraftFrom(null),
    time: kind === 'eaten' ? nowTime() : '',
    showTime: false,
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
}: {
  open: boolean;
  onClose: () => void;
  memberId: string;
  date: string;
  slot: MealSlot;
}) {
  const createConsumption = useCreateConsumption();
  const createPlan = useCreateMealPlanEntry();
  const kinds = allowedKinds(date);
  const [draft, setDraft] = useState(() => emptyDraft(kinds[0] ?? 'eaten'));
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [failure, setFailure] = useState<string | null>(null);
  const [added, setAdded] = useState<AddedItem[]>([]);
  const busy = createConsumption.isPending || createPlan.isPending;

  function resetAndClose() {
    setDraft(emptyDraft(kinds[0] ?? 'eaten'));
    setErrors({});
    setFailure(null);
    setAdded([]);
    onClose();
  }

  function handleClose() {
    if (!busy) resetAndClose();
  }

  function setKind(kind: AddKind) {
    setDraft({
      ...draft,
      kind,
      time: kind === 'eaten' && !draft.time ? nowTime() : draft.time,
    });
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
      if (draft.kind === 'planned') {
        await createPlan.mutateAsync({
          planned_on: date,
          planned_time: draft.time || null,
          slot,
          components: [{ product_id: draft.product.id, amount }],
        });
      } else {
        await createConsumption.mutateAsync({
          member_id: memberId,
          product_id: draft.product.id,
          slot,
          amount,
          consumed_on: date,
          consumed_at: combineDateTime(date, draft.time || nowTime()),
        });
      }
      setAdded([
        ...added,
        { key: crypto.randomUUID(), kind: draft.kind, productName: draft.product.name, amount },
      ]);
      setDraft(emptyDraft(draft.kind));
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

  return (
    <FormDialog open={open} onClose={handleClose} maxWidth="sm" fullWidth>
      <form onSubmit={onSubmit}>
        <DialogTitle sx={{ pb: 0.75 }}>Add food</DialogTitle>
        <DialogContent>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2.5 }}>
            {labelForSlot(slot)} · {formatFullDate(date)}
          </Typography>
          <Stack spacing={3}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}

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

            {draft.product ? (
              <>
                <AmountFields
                  product={draft.product}
                  draft={draft.amount}
                  errors={errors}
                  onChange={(amount) => setDraft({ ...draft, amount })}
                />

                {kinds.length > 1 ? (
                  <ToggleButtonGroup
                    value={draft.kind}
                    exclusive
                    fullWidth
                    size="small"
                    aria-label="Add food as"
                    onChange={(_event, value: AddKind | null) => value && setKind(value)}
                  >
                    <ToggleButton value="planned">Planned</ToggleButton>
                    <ToggleButton value="eaten">Eaten</ToggleButton>
                  </ToggleButtonGroup>
                ) : null}

                {draft.showTime ? (
                  <TextField
                    type="time"
                    label={draft.kind === 'eaten' ? 'Time eaten' : 'Time'}
                    value={draft.time}
                    onChange={(event) => setDraft({ ...draft, time: event.target.value })}
                    slotProps={{ inputLabel: { shrink: true } }}
                  />
                ) : (
                  <Box>
                    <Button
                      size="small"
                      onClick={() => setDraft({ ...draft, showTime: true })}
                      sx={{ px: 0 }}
                    >
                      Adjust time
                    </Button>
                  </Box>
                )}
              </>
            ) : null}
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={handleClose} disabled={busy}>
            {added.length > 0 ? 'Done' : 'Cancel'}
          </Button>
          <Button type="submit" variant="contained" disabled={busy || !draft.product}>
            {busy ? 'Adding…' : draft.kind === 'planned' ? 'Add to plan' : 'Add'}
          </Button>
        </DialogActions>
      </form>
    </FormDialog>
  );
}
