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
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Typography from '@mui/material/Typography';
import { useState, type FormEvent } from 'react';
import {
  ApiError,
  type Amount,
  type MealPlanEntry,
  type MealSlot,
  type Product,
  type RecipeSummary,
} from '../../api/client';
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
import { RecipePicker } from './RecipePicker';
import { labelForSlot, SLOTS } from './slots';

type AddKind = 'planned' | 'eaten';
type Source = 'product' | 'recipe';

type ItemRef = { product_id: string } | { recipe_id: string };

type EntryDraft = {
  kind: AddKind;
  source: Source;
  product: Product | null;
  recipe: RecipeSummary | null;
  amount: AmountDraft;
  servings: string;
  time: string;
  slot: MealSlot;
};

type AddedItem = {
  key: string;
  kind: AddKind;
  item: ItemRef;
  name: string;
  amount: Amount;
};

function emptyDraft(kind: AddKind, slot: MealSlot): EntryDraft {
  return {
    kind,
    source: 'product',
    product: null,
    recipe: null,
    amount: amountDraftFrom(null),
    servings: '1',
    time: '',
    slot,
  };
}

function validate(draft: EntryDraft): Record<string, string> {
  if (draft.source === 'recipe') {
    const errors: Record<string, string> = {};
    if (!draft.recipe) errors.item = 'Pick a recipe';
    const servings = Number(draft.servings);
    if (!draft.servings.trim() || Number.isNaN(servings) || servings <= 0) {
      errors.amount = 'Servings must be more than zero';
    }
    return errors;
  }
  const errors = validateAmountDraft(draft.amount);
  if (!draft.product) errors.item = 'Pick a product';
  return errors;
}

function draftItem(draft: EntryDraft): ItemRef | null {
  if (draft.source === 'recipe') {
    return draft.recipe ? { recipe_id: draft.recipe.id } : null;
  }
  return draft.product ? { product_id: draft.product.id } : null;
}

function draftAmount(draft: EntryDraft): Amount | null {
  if (draft.source === 'recipe') {
    const value = Number(draft.servings);
    return Number.isNaN(value) || value <= 0 ? null : { kind: 'servings', value };
  }
  return draftToAmount(draft.amount);
}

function componentItem(component: MealPlanEntry['components'][number]): ItemRef {
  return component.item_kind === 'recipe'
    ? { recipe_id: component.recipe_id }
    : { product_id: component.product_id };
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
  const ready = draft.source === 'recipe' ? Boolean(draft.recipe) : Boolean(draft.product);

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

  function setSource(source: Source) {
    setDraft({ ...emptyDraft(draft.kind, draft.slot), source, time: draft.time });
    setErrors({});
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    const found = validate(draft);
    setErrors(found);
    if (Object.keys(found).length > 0) return;

    const item = draftItem(draft);
    const amount = draftAmount(draft);
    if (!item || !amount) return;
    const name = draft.source === 'recipe' ? draft.recipe!.name : draft.product!.name;

    try {
      if (draft.kind === 'eaten') {
        await createConsumption.mutateAsync({
          member_id: memberId,
          ...item,
          slot: draft.slot,
          amount,
          consumed_on: date,
          consumed_at: draft.time ? combineDateTime(date, draft.time) : null,
        });
      }
      setAdded([...added, { key: crypto.randomUUID(), kind: draft.kind, item, name, amount }]);
      setDraft({ ...emptyDraft(draft.kind, draft.slot), source: draft.source });
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
      const components = added.map((entryItem) => ({ ...entryItem.item, amount: entryItem.amount }));
      if (entry) {
        await updatePlan.mutateAsync({
          id: entry.id,
          revision: entry.revision,
          body: {
            components: [
              ...entry.components.map((component) => ({
                id: component.id,
                ...componentItem(component),
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
                {added.map((entryItem) => (
                  <Stack
                    key={entryItem.key}
                    direction="row"
                    spacing={1}
                    sx={{ alignItems: 'center', py: 0.75 }}
                  >
                    {entryItem.kind === 'eaten' ? (
                      <CheckIcon fontSize="small" color="success" />
                    ) : (
                      <RadioButtonUncheckedIcon fontSize="small" color="disabled" />
                    )}
                    <Typography variant="body2" sx={{ flexGrow: 1 }}>
                      {entryItem.name}
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
                      {formatAmount(entryItem.amount)}
                    </Typography>
                  </Stack>
                ))}
              </Stack>
            ) : null}

            <ToggleButtonGroup
              exclusive
              size="small"
              value={draft.source}
              onChange={(_, next) => next && setSource(next)}
              aria-label="What to add"
            >
              <ToggleButton value="product">Product</ToggleButton>
              <ToggleButton value="recipe">Recipe</ToggleButton>
            </ToggleButtonGroup>

            {draft.source === 'recipe' ? (
              <RecipePicker
                key={`recipe-${added.length}`}
                value={draft.recipe}
                onChange={(recipe) => setDraft({ ...draft, recipe })}
                error={Boolean(errors.item)}
                helperText={errors.item}
                autoFocus
              />
            ) : (
              <ProductPicker
                key={`product-${added.length}`}
                value={draft.product}
                onChange={(product) =>
                  setDraft({ ...draft, product, amount: amountDraftFrom(product) })
                }
                error={Boolean(errors.item)}
                helperText={errors.item}
                autoFocus
              />
            )}

            {draft.kind === 'eaten' ? (
              <TextField
                type="time"
                label="Time eaten (optional)"
                value={draft.time}
                onChange={(event) => setDraft({ ...draft, time: event.target.value })}
                slotProps={{ inputLabel: { shrink: true } }}
              />
            ) : null}

            {draft.source === 'recipe' ? (
              <TextField
                type="number"
                label="Servings"
                value={draft.servings}
                onChange={(event) => setDraft({ ...draft, servings: event.target.value })}
                error={Boolean(errors.amount)}
                helperText={errors.amount}
                slotProps={{ htmlInput: { min: 0, step: 0.5 } }}
              />
            ) : draft.product ? (
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
          <Button type="submit" variant="contained" disabled={busy || !ready}>
            {busy ? 'Adding…' : draft.kind === 'planned' ? 'Add to meal' : 'Add'}
          </Button>
        </DialogActions>
      </form>
    </FormDialog>
  );
}
