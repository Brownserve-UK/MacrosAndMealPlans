import AddIcon from '@mui/icons-material/AddOutlined';
import DeleteIcon from '@mui/icons-material/DeleteOutlineOutlined';
import EditIcon from '@mui/icons-material/EditOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import IconButton from '@mui/material/IconButton';
import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState, type ReactNode } from 'react';
import { ApiError } from '../../api/client';
import type { Product, Purchase, ShoppingRequirement, Unit } from '../../api/client';
import { useProducts, useRecordPurchase, useUpdatePurchase } from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';
import { UnitSelect } from '../../components/UnitSelect';
import { formatDayLabel } from '../meal-plan/date';
import { labelForSlot } from '../meal-plan/slots';
import { formatQuantity } from '../stock/SpokenFor';
import { purchasesOf, requirementKey } from './requirementKey';
import { sectionLabel } from './sections';

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

function Heading({ children }: { children: ReactNode }) {
  return (
    <Typography variant="overline" color="text.secondary" sx={{ display: 'block', mt: 1 }}>
      {children}
    </Typography>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <Box>
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
        {label}
      </Typography>
      <Typography variant="body1">{value}</Typography>
    </Box>
  );
}

type Flag = { severity: 'warning' | 'info'; text: string };

function flags(requirement: ShoppingRequirement): Flag[] {
  const out: Flag[] = [];
  if (requirement.assignment.kind === 'needs_earlier_opportunity') {
    out.push({ severity: 'warning', text: 'Needed before your next shop' });
  }
  if (requirement.assignment.kind === 'unassigned') {
    out.push({ severity: 'info', text: 'No shop to put this on yet' });
  }
  if (requirement.certainty.kind === 'suggested') {
    out.push({
      severity: 'info',
      text:
        requirement.certainty.reason === 'unknown_availability'
          ? 'No stock recorded, so we cannot tell what you already have'
          : 'Only needed if an unconfirmed meal was eaten',
    });
  }
  if (requirement.gaps?.includes('incompatible_units')) {
    out.push({
      severity: 'warning',
      text: 'Some meals measure this differently, so the amount may be short',
    });
  }
  return out;
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
  const [editing, setEditing] = useState<Purchase | 'new' | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const update = useUpdatePurchase();

  const ingredientId =
    requirement.subject.kind === 'ingredient' ? requirement.subject.ingredient_id : undefined;
  const products = useProducts(
    ingredientId ? { mapped_ingredient_id: ingredientId, per_page: 200 } : { per_page: 1 },
  );
  const choices: Product[] = ingredientId ? (products.data?.items ?? []) : [];

  function nameFor(purchase: Purchase): string {
    if (!purchase.product_id) return 'Product not said yet';
    return (
      choices.find((product) => product.id === purchase.product_id)?.name ?? requirement.name
    );
  }

  async function remove(purchase: Purchase) {
    setFailure(null);
    try {
      await update.mutateAsync({
        id: purchase.id,
        revision: purchase.revision,
        cancelled: true,
      });
    } catch (caught) {
      setFailure(caught instanceof ApiError ? caught.message : 'Could not remove that.');
    }
  }

  const dates = [
    requirement.required_by ? { label: 'Needed by', value: requirement.required_by } : null,
    requirement.use_by_at_least
      ? { label: 'Must keep until', value: requirement.use_by_at_least }
      : null,
  ].filter((entry) => entry != null);

  if (editing) {
    return (
      <PurchaseForm
        open={open}
        requirement={requirement}
        purchase={editing === 'new' ? null : editing}
        choices={choices}
        opportunityDate={opportunityDate}
        onDone={() => setEditing(null)}
      />
    );
  }

  return (
    <FormDialog open={open} onClose={onClose} maxWidth="xs" fullWidth>
      <DialogTitle sx={{ pb: 0 }}>
        {requirement.name}
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
          {sectionLabel(requirement.section)}
        </Typography>
      </DialogTitle>

      <DialogContent>
        <Stack spacing={2} sx={{ pt: 1 }}>
          {failure ? <Alert severity="error">{failure}</Alert> : null}

          <Box>
            <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
              Still to buy
            </Typography>
            <Typography variant="h4" component="p">
              {requirement.quantity ? formatQuantity(requirement.quantity) : 'Nothing'}
            </Typography>
          </Box>

          {flags(requirement).map((flag) => (
            <Alert key={flag.text} severity={flag.severity}>
              {flag.text}
            </Alert>
          ))}

          {dates.length > 0 ? (
            <Box sx={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 2 }}>
              {dates.map((entry) => (
                <Fact key={entry.label} label={entry.label} value={formatDayLabel(entry.value)} />
              ))}
            </Box>
          ) : null}

          {buying || purchasesOf(requirement).length > 0 ? (
            <Box>
              <Heading>In the trolley</Heading>
              {purchasesOf(requirement).length === 0 ? (
                <Typography variant="body2" color="text.secondary">
                  Nothing yet.
                </Typography>
              ) : (
                <Stack divider={<Box sx={{ borderTop: 1, borderColor: 'divider' }} />}>
                  {purchasesOf(requirement).map((purchase) => (
                    <Stack
                      key={purchase.id}
                      direction="row"
                      spacing={1}
                      sx={{ alignItems: 'center', py: 0.75 }}
                    >
                      <Box sx={{ flex: 1, minWidth: 0 }}>
                        <Typography variant="body2">{nameFor(purchase)}</Typography>
                        <Typography variant="caption" color="text.secondary">
                          {purchase.quantity
                            ? formatQuantity(purchase.quantity)
                            : 'Amount not said yet'}
                        </Typography>
                      </Box>
                      <IconButton
                        size="small"
                        aria-label={`Change ${nameFor(purchase)}`}
                        onClick={() => setEditing(purchase)}
                      >
                        <EditIcon fontSize="small" />
                      </IconButton>
                      <IconButton
                        size="small"
                        aria-label={`Remove ${nameFor(purchase)}`}
                        disabled={update.isPending}
                        onClick={() => void remove(purchase)}
                      >
                        <DeleteIcon fontSize="small" />
                      </IconButton>
                    </Stack>
                  ))}
                </Stack>
              )}
              {buying ? (
                <Button size="small" startIcon={<AddIcon />} onClick={() => setEditing('new')}>
                  Add another
                </Button>
              ) : null}
            </Box>
          ) : null}

          {requirement.claims.length > 0 ? (
            <Box>
              <Heading>What needs it</Heading>
              <Box
                sx={{
                  display: 'grid',
                  gridTemplateColumns: 'auto 1fr auto',
                  columnGap: 1.5,
                  rowGap: 0.5,
                }}
              >
                {requirement.claims.map((claim, index) => (
                  <Box key={index} sx={{ display: 'contents' }}>
                    <Typography variant="body2" color="text.secondary">
                      {formatDayLabel(claim.planned_on)}
                    </Typography>
                    <Typography variant="body2" noWrap>
                      {claim.recipe_name ?? labelForSlot(claim.slot)}
                    </Typography>
                    <Typography variant="body2" color="text.secondary">
                      {formatQuantity(claim.quantity)}
                    </Typography>
                  </Box>
                ))}
              </Box>
            </Box>
          ) : null}
        </Stack>
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose}>Close</Button>
      </DialogActions>
    </FormDialog>
  );
}

function PurchaseForm({
  open,
  requirement,
  purchase,
  choices,
  opportunityDate,
  onDone,
}: {
  open: boolean;
  requirement: ShoppingRequirement;
  purchase: Purchase | null;
  choices: Product[];
  opportunityDate?: string | null;
  onDone: () => void;
}) {
  const record = useRecordPurchase();
  const update = useUpdatePurchase();

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

  const chosen = productId || (choices.length === 1 ? choices[0]!.id : '');
  const parsed = amount.trim() === '' ? null : Number(amount);
  const complete =
    chosen !== '' && parsed != null && !Number.isNaN(parsed) && parsed > 0 && unit !== '';
  const saving = record.isPending || update.isPending;

  async function onSubmit(event: React.FormEvent) {
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
      onDone();
    } catch (caught) {
      setFailure(caught instanceof ApiError ? caught.message : 'Could not save.');
    }
  }

  return (
    <FormDialog open={open} onClose={onDone} maxWidth="xs" fullWidth>
      <form onSubmit={onSubmit}>
        <DialogTitle>{purchase ? 'Change what you bought' : 'Add what you bought'}</DialogTitle>
        <DialogContent>
          <Stack spacing={2.5} sx={{ pt: 0.5 }}>
            {failure ? <Alert severity="error">{failure}</Alert> : null}

            {choices.length > 1 ? (
              <TextField
                select
                label="Product"
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
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={onDone}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={saving}>
            {saving ? 'Saving…' : 'Save'}
          </Button>
        </DialogActions>
      </form>
    </FormDialog>
  );
}
