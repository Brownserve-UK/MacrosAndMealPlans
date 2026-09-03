import DeleteIcon from '@mui/icons-material/DeleteOutlineOutlined';
import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import IconButton from '@mui/material/IconButton';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useState, type FormEvent } from 'react';
import { ApiError, type Product, type RecipeSummary } from '../../api/client';
import type { components } from '../../api/schema';
import { useReviewMealOutcomes } from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';
import {
  AmountFields,
  amountDraftFrom,
  draftToAmount,
  validateAmountDraft,
  type AmountDraft,
} from './AmountFields';
import { FoodSearch, type FoodChoice } from './FoodSearch';
import { formatAmount } from './format';

type Picked =
  | { key: string; kind: 'product'; product: Product; amount: AmountDraft }
  | { key: string; kind: 'recipe'; recipe: RecipeSummary; servings: string };

function nameOf(picked: Picked) {
  return picked.kind === 'product' ? picked.product.name : picked.recipe.name;
}

export function AteSomethingElseDialog({
  open,
  onClose,
  entryId,
  revision,
  memberId,
  consumedOn,
}: {
  open: boolean;
  onClose: () => void;
  entryId: string;
  revision: number;
  memberId: string;
  consumedOn: string;
}) {
  const review = useReviewMealOutcomes();
  const [picked, setPicked] = useState<Picked[]>([]);
  const [failure, setFailure] = useState<string | null>(null);

  function add(choice: FoodChoice) {
    const key = crypto.randomUUID();
    setPicked((current) => [
      ...current,
      choice.kind === 'product'
        ? { key, kind: 'product', product: choice.product, amount: amountDraftFrom(choice.product) }
        : { key, kind: 'recipe', recipe: choice.recipe, servings: '1' },
    ]);
  }

  function remove(key: string) {
    setPicked((current) => current.filter((entry) => entry.key !== key));
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (picked.length === 0) {
      setFailure('Add what you ate instead.');
      return;
    }

    const replacements: components['schemas']['ReplacementItemRequest'][] = [];
    for (const entry of picked) {
      if (entry.kind === 'product') {
        const amount = draftToAmount(entry.amount);
        if (Object.keys(validateAmountDraft(entry.amount)).length > 0 || !amount) {
          setFailure(`Check the amount for ${entry.product.name}.`);
          return;
        }
        replacements.push({ item_kind: 'product', product_id: entry.product.id, amount });
      } else {
        const servings = Number(entry.servings);
        if (!entry.servings.trim() || Number.isNaN(servings) || servings <= 0) {
          setFailure(`Check the servings for ${entry.recipe.name}.`);
          return;
        }
        replacements.push({
          item_kind: 'recipe',
          recipe_id: entry.recipe.id,
          amount: { kind: 'servings', value: servings },
        });
      }
    }

    try {
      await review.mutateAsync({
        id: entryId,
        revision,
        body: {
          consumed_on: consumedOn,
          consumed_at: null,
          members: [{ member_id: memberId, result: 'changed', components: [], replacements }],
          guests: [],
        },
      });
      onClose();
    } catch (caught) {
      setFailure(caught instanceof ApiError ? caught.message : 'Could not record this.');
    }
  }

  return (
    <FormDialog open={open} onClose={review.isPending ? () => {} : onClose} maxWidth="sm" fullWidth>
      <form onSubmit={submit}>
        <DialogTitle>Ate something else</DialogTitle>
        <DialogContent>
          <Stack spacing={2.5} sx={{ pt: 0.5 }}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}
            <Typography color="text.secondary">
              The planned food will be marked not eaten.
            </Typography>
            <FoodSearch onPick={add} autoFocus />
            {picked.map((entry) => (
              <Paper key={entry.key} variant="outlined" sx={{ px: 2, py: 1.5 }}>
                <Stack direction="row" spacing={2} sx={{ alignItems: 'center', justifyContent: 'space-between' }}>
                  <Typography sx={{ minWidth: 0 }}>{nameOf(entry)}</Typography>
                  <IconButton aria-label={`Remove ${nameOf(entry)}`} onClick={() => remove(entry.key)} size="small">
                    <DeleteIcon fontSize="small" />
                  </IconButton>
                </Stack>
                {entry.kind === 'product' ? (
                  <AmountFields
                    draft={entry.amount}
                    errors={{}}
                    onChange={(amount) =>
                      setPicked((current) =>
                        current.map((candidate) =>
                          candidate.key === entry.key && candidate.kind === 'product'
                            ? { ...candidate, amount }
                            : candidate,
                        ),
                      )
                    }
                    product={entry.product}
                  />
                ) : (
                  <Typography variant="caption" color="text.secondary">
                    {formatAmount({ kind: 'servings', value: Number(entry.servings) || 1 })}
                  </Typography>
                )}
              </Paper>
            ))}
          </Stack>
        </DialogContent>
        <DialogActions sx={{ px: 3, pb: 2 }}>
          <Button onClick={onClose} disabled={review.isPending}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={review.isPending}>
            {review.isPending ? 'Saving…' : 'Save'}
          </Button>
        </DialogActions>
      </form>
    </FormDialog>
  );
}
