import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import MenuItem from '@mui/material/MenuItem';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { ApiError } from '../../api/client';
import type { Purchase, Unit } from '../../api/client';
import { useIngredients, usePendingPutAway, useProducts, usePutAway } from '../../api/queries';
import { PageHeader } from '../../components/PageHeader';
import { UnitSelect } from '../../components/UnitSelect';
import { EmptyState, ErrorState, Loading } from '../../components/States';

export function PutAwayPage() {
  const waiting = usePendingPutAway();
  const ingredients = useIngredients({ per_page: 200 });
  const navigate = useNavigate();

  if (waiting.isLoading) return <Loading label="Fetching what you bought" />;
  if (waiting.isError) {
    return <ErrorState error={waiting.error} onRetry={() => waiting.refetch()} />;
  }

  const purchases = waiting.data ?? [];
  const named = new Map<string, string>();
  for (const ingredient of ingredients.data?.items ?? []) {
    named.set(ingredient.id, ingredient.name);
  }

  return (
    <>
      <PageHeader
        title="Put away"
        subtitle={
          purchases.length === 1
            ? '1 thing to sort out.'
            : `${purchases.length} things to sort out.`
        }
      />

      {purchases.length === 0 ? (
        <EmptyState title="Nothing waiting" description="Everything you bought is in stock." />
      ) : (
        <Stack spacing={2}>
          {purchases.map((purchase) => (
            <PutAwayRow key={purchase.id} purchase={purchase} named={named} />
          ))}
          <Stack direction="row" spacing={1}>
            <Button onClick={() => void navigate({ to: '/shopping' })}>Do this later</Button>
          </Stack>
        </Stack>
      )}
    </>
  );
}

function PutAwayRow({ purchase, named }: { purchase: Purchase; named: Map<string, string> }) {
  const update = usePutAway();
  const [search, setSearch] = useState('');
  const [productId, setProductId] = useState(purchase.product_id ?? '');
  const [amount, setAmount] = useState(purchase.quantity ? String(purchase.quantity.amount) : '');
  const [unit, setUnit] = useState<Unit | ''>(purchase.quantity?.unit ?? '');
  const [failure, setFailure] = useState<string | null>(null);
  const [done, setDone] = useState(false);

  const products = useProducts(
    search.trim() ? { q: search.trim(), per_page: 8 } : { per_page: 20 },
  );
  const choices = products.data?.items ?? [];
  const name =
    purchase.name ??
    choices.find((product) => product.id === purchase.product_id)?.name ??
    (purchase.ingredient_id ? named.get(purchase.ingredient_id) : undefined) ??
    'Something you bought';

  const parsed = amount.trim() === '' ? null : Number(amount);
  const complete = productId !== '' && parsed != null && !Number.isNaN(parsed) && parsed > 0 && unit !== '';

  async function onSave() {
    setFailure(null);
    try {
      await update.mutateAsync({
        id: purchase.id,
        revision: purchase.revision,
        product_id: productId,
        quantity: { amount: parsed as number, unit: unit as Unit },
      });
      setDone(true);
    } catch (caught) {
      setFailure(caught instanceof ApiError ? caught.message : 'Could not save that.');
    }
  }

  if (done) return null;

  return (
    <Paper variant="outlined" sx={{ p: 2.5 }}>
      <Stack spacing={2}>
        {failure ? <Alert severity="error">{failure}</Alert> : null}

        <Typography variant="subtitle1">{name}</Typography>

        <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1.5}>
          <TextField
            select
            label="Product"
            value={productId}
            onChange={(e) => setProductId(e.target.value)}
            sx={{ flexGrow: 1 }}
            slotProps={{
              select: {
                MenuProps: { autoFocus: false },
                renderValue: (value) =>
                  choices.find((p) => p.id === value)?.name ?? 'Which product?',
              },
            }}
          >
            <MenuItem disableRipple onKeyDown={(e) => e.stopPropagation()}>
              <TextField
                size="small"
                placeholder="Search products"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                onClick={(e) => e.stopPropagation()}
                fullWidth
              />
            </MenuItem>
            {choices.map((product) => (
              <MenuItem key={product.id} value={product.id}>
                {product.name}
              </MenuItem>
            ))}
          </TextField>

          <TextField
            label="Amount"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            inputMode="decimal"
            sx={{ width: { sm: 120 } }}
          />
          <UnitSelect label="Unit" value={unit} onChange={setUnit} sx={{ width: { sm: 140 } }} />
        </Stack>

        <Stack direction="row" spacing={1}>
          <Button
            variant="contained"
            disabled={!complete || update.isPending}
            onClick={() => void onSave()}
          >
            Put it away
          </Button>
        </Stack>
      </Stack>
    </Paper>
  );
}
