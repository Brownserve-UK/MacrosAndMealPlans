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
import { ApiError } from '../../api/client';
import type { ShoppingRequirement, Unit } from '../../api/client';
import { useProducts, useRecordPurchase, useUpdatePurchase } from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';
import { UnitSelect } from '../../components/UnitSelect';
import { formatDayLabel } from '../meal-plan/date';
import { formatQuantity } from '../stock/SpokenFor';
import { requirementKey } from './requirementKey';

export function RequirementDialog({
  open,
  requirement,
  opportunityDate,
  buying,
  onClose,
}: {
  open: boolean;
  requirement: ShoppingRequirement | null;
  opportunityDate?: string | null;
  buying?: boolean;
  onClose: () => void;
}) {
  if (!requirement) return null;
  return (
    <Body
      key={requirementKey(requirement)}
      open={open}
      requirement={requirement}
      opportunityDate={opportunityDate}
      buying={Boolean(buying)}
      onClose={onClose}
    />
  );
}

function notes(requirement: ShoppingRequirement): string[] {
  const lines: string[] = [];
  if (requirement.assignment.kind === 'needs_earlier_opportunity') {
    lines.push('Needed before your next shop.');
  }
  if (requirement.assignment.kind === 'unassigned') {
    lines.push('No shop to put this on yet.');
  }
  if (requirement.certainty.kind === 'suggested') {
    lines.push(
      requirement.certainty.reason === 'unknown_availability'
        ? 'No stock recorded, so we cannot tell what you already have.'
        : 'Only needed if an unconfirmed meal was eaten.',
    );
  }
  if (requirement.gaps?.includes('incompatible_units')) {
    lines.push('Some meals measure this differently, so the amount may be short.');
  }
  return lines;
}

function Body({
  open,
  requirement,
  opportunityDate,
  buying,
  onClose,
}: {
  open: boolean;
  requirement: ShoppingRequirement;
  opportunityDate?: string | null;
  buying: boolean;
  onClose: () => void;
}) {
  const record = useRecordPurchase();
  const update = useUpdatePurchase();
  const purchase = requirement.purchase ?? null;

  const ingredientId =
    requirement.subject.kind === 'ingredient' ? requirement.subject.ingredient_id : undefined;
  const pinnedProductId =
    requirement.subject.kind === 'product' ? requirement.subject.product_id : undefined;

  const [productId, setProductId] = useState(purchase?.product_id ?? pinnedProductId ?? '');
  const [amount, setAmount] = useState(
    purchase?.quantity
      ? String(purchase.quantity.amount)
      : requirement.quantity
        ? String(requirement.quantity.amount)
        : '',
  );
  const [unit, setUnit] = useState<Unit | ''>(
    purchase?.quantity?.unit ?? requirement.quantity?.unit ?? '',
  );
  const [failure, setFailure] = useState<string | null>(null);

  const products = useProducts(
    ingredientId ? { mapped_ingredient_id: ingredientId, per_page: 200 } : { per_page: 1 },
  );
  const choices = pinnedProductId ? [] : (products.data?.items ?? []);
  const chosen = productId || (choices.length === 1 ? choices[0]!.id : '');

  const parsed = amount.trim() === '' ? null : Number(amount);
  const complete =
    chosen !== '' && parsed != null && !Number.isNaN(parsed) && parsed > 0 && unit !== '';
  const saving = record.isPending || update.isPending;

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    if (amount.trim() !== '' && (parsed == null || Number.isNaN(parsed) || parsed <= 0)) {
      setFailure('Enter an amount above zero, or leave it blank.');
      return;
    }
    const quantity = complete ? { amount: parsed, unit: unit as Unit } : undefined;
    try {
      if (purchase) {
        await update.mutateAsync({
          id: purchase.id,
          revision: purchase.revision,
          product_id: chosen || undefined,
          quantity,
        });
      } else {
        await record.mutateAsync({
          ingredient_id: ingredientId,
          product_id: chosen || undefined,
          quantity,
          opportunity_date: opportunityDate ?? undefined,
        });
      }
      onClose();
    } catch (caught) {
      setFailure(caught instanceof ApiError ? caught.message : 'Could not save.');
    }
  }

  return (
    <FormDialog open={open} onClose={onClose} maxWidth="xs" fullWidth>
      <form onSubmit={onSubmit}>
        <DialogTitle sx={{ pb: 0.5 }}>{requirement.name}</DialogTitle>
        <DialogContent>
          <Stack spacing={2} sx={{ pt: 0.5 }}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}

            {requirement.quantity ? (
              <Typography variant="h6">{formatQuantity(requirement.quantity)}</Typography>
            ) : null}

            {requirement.required_by ? (
              <Typography variant="body2">
                Needed by {formatDayLabel(requirement.required_by)}
              </Typography>
            ) : null}
            {requirement.use_by_at_least ? (
              <Typography variant="body2">
                Use by at least {formatDayLabel(requirement.use_by_at_least)}
              </Typography>
            ) : null}

            {notes(requirement).map((line) => (
              <Typography key={line} variant="body2" color="text.secondary">
                {line}
              </Typography>
            ))}

            {requirement.claims.length > 0 ? (
              <div>
                <Typography variant="overline" color="text.secondary">
                  What needs it
                </Typography>
                {requirement.claims.map((claim, index) => (
                  <Typography key={index} variant="body2" color="text.secondary">
                    {formatDayLabel(claim.planned_on)} · {claim.recipe_name ?? claim.slot} ·{' '}
                    {formatQuantity(claim.quantity)}
                  </Typography>
                ))}
              </div>
            ) : null}

            {buying ? (
              <>
                <Divider />
                {choices.length > 1 ? (
                  <TextField
                    select
                    label="Which one"
                    value={productId}
                    onChange={(e) => setProductId(e.target.value)}
                    fullWidth
                  >
                    <MenuItem value="">Not sure yet</MenuItem>
                    {choices.map((product) => (
                      <MenuItem key={product.id} value={product.id}>
                        {product.name}
                      </MenuItem>
                    ))}
                  </TextField>
                ) : null}

                <Stack direction="row" spacing={1.5}>
                  <TextField
                    label="Amount"
                    value={amount}
                    onChange={(e) => setAmount(e.target.value)}
                    inputMode="decimal"
                    fullWidth
                  />
                  <UnitSelect label="Unit" value={unit} onChange={setUnit} sx={{ minWidth: 120 }} />
                </Stack>
              </>
            ) : null}
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={onClose}>{buying ? 'Cancel' : 'Close'}</Button>
          {buying ? (
            <Button type="submit" variant="contained" disabled={saving}>
              {saving ? 'Saving…' : complete ? 'Add to stock' : 'Mark bought'}
            </Button>
          ) : null}
        </DialogActions>
      </form>
    </FormDialog>
  );
}
