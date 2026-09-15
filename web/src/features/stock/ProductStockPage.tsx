import AddIcon from '@mui/icons-material/AddOutlined';
import ChevronRightIcon from '@mui/icons-material/ChevronRight';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';
import type { DemandClaim, Quantity, StockItem, StockLevel } from '../../api/client';
import { useProduct, useProducts, useStock, useStockAvailability } from '../../api/queries';
import { BackLabel } from '../../components/BackLink';
import { PageHeader } from '../../components/PageHeader';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { displayUnit } from '../../components/UnitSelect';
import { NewStockDialog } from './NewStockDialog';
import { SpokenFor } from './SpokenFor';
import { levelFor } from './stockLevel';

function amountLabel(level: StockLevel): string {
  if (level.mode === 'not_tracked') return 'Amount not tracked';
  const amount = `${level.quantity.amount} ${displayUnit(level.quantity.unit)}`;
  return level.mode === 'estimated' ? `About ${amount}` : amount;
}

function dateLabel(item: StockItem): string | null {
  if (item.usability_deadline) {
    return `Usable until ${new Date(`${item.usability_deadline.date}T00:00:00`).toLocaleDateString('en-GB')}`;
  }
  if (item.source_date) {
    const label = item.source_date.kind === 'use_by' ? 'Use by' : 'Best before';
    return `${label} ${new Date(`${item.source_date.date}T00:00:00`).toLocaleDateString('en-GB')}`;
  }
  return null;
}

function sumIn(claims: DemandClaim[], unit: string): number | null {
  let total = 0;
  for (const claim of claims) {
    if (claim.quantity.unit !== unit) return null;
    total += claim.quantity.amount;
  }
  return total;
}

export function ProductStockPage({ productId }: { productId: string }) {
  const [addOpen, setAddOpen] = useState(false);
  const product = useProduct(productId);
  const stock = useStock({ product_id: productId, per_page: 200 });
  const availability = useStockAvailability(productId);
  const ingredientId = product.data?.mapped_ingredient_id ?? undefined;
  const pool = useProducts({ mapped_ingredient_id: ingredientId, per_page: 50 });

  if (product.isLoading || stock.isLoading || availability.isLoading) {
    return <Loading label="Loading product stock" />;
  }
  if (product.isError || !product.data) {
    return <ErrorState error={product.error} onRetry={() => product.refetch()} />;
  }
  if (stock.isError) return <ErrorState error={stock.error} onRetry={() => stock.refetch()} />;
  if (availability.isError) {
    return <ErrorState error={availability.error} onRetry={() => availability.refetch()} />;
  }

  const items = stock.data?.items ?? [];
  const row = availability.data?.products.find((candidate) => candidate.product_id === productId);
  const level = levelFor(row?.availability ?? null);

  const claims = availability.data?.claims ?? [];
  const pinned = claims.filter(
    (claim) => claim.subject.kind === 'product' && claim.subject.product_id === productId,
  );
  const shared = claims.filter((claim) => claim.subject.kind === 'ingredient');
  const ingredientRow = availability.data?.ingredients.find(
    (candidate) => candidate.ingredient_id === ingredientId,
  );
  const alternatives = (pool.data?.items ?? [])
    .filter((candidate) => candidate.id !== productId)
    .map((candidate) => candidate.name);

  let expectedFromHere: Quantity | null = null;
  if (row?.availability.state === 'quantified') {
    const unit = row.availability.planned_demand.unit;
    const pinnedTotal = sumIn(pinned, unit);
    if (pinnedTotal !== null) {
      const share = row.availability.planned_demand.amount - pinnedTotal;
      if (share > 0) expectedFromHere = { amount: share, unit };
    }
  }

  return (
    <>
      <Link to="/stock" className="app-link">
        <BackLabel>Stock</BackLabel>
      </Link>
      <PageHeader
        title={product.data.name}
        subtitle="Stock on hand for this product."
        actions={
          <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
            Add stock
          </Button>
        }
      />

      <Paper variant="outlined" sx={{ p: 3, mb: 3 }}>
        <Stack direction="row" sx={{ alignItems: 'baseline', justifyContent: 'space-between' }}>
          <Typography variant="overline" color="text.secondary">
            Availability
          </Typography>
          <Typography className="numeral" sx={{ fontWeight: 600, color: level.colour }}>
            {level.figure ? `${level.figure.needed} / ${level.figure.available}` : level.statusWord}
          </Typography>
        </Stack>
        {level.figure ? (
          <Box
            aria-hidden
            sx={{ mt: 1.25, height: 8, borderRadius: 999, overflow: 'hidden', backgroundColor: 'text.primary' }}
          >
            <Box
              sx={{
                width: level.solidRed ? '100%' : `${level.fillPct}%`,
                height: '100%',
                backgroundColor: level.colour,
              }}
            />
          </Box>
        ) : null}
        {level.detailLine ? (
          <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 1 }}>
            {level.detailLine}
          </Typography>
        ) : null}
      </Paper>

      <SpokenFor
        pinned={pinned}
        shared={shared}
        ingredientName={ingredientRow?.name ?? null}
        expectedFromHere={expectedFromHere}
        alternatives={alternatives}
      />

      <Typography variant="h2" sx={{ mb: 2 }}>
        On hand
      </Typography>
      {items.length === 0 ? (
        <EmptyState title="No stock on hand" description="Add stock for this product." />
      ) : (
        <Stack spacing={1.5}>
          {items.map((item) => (
            <Paper key={item.id} variant="outlined">
              <Link
                to="/stock/$id"
                params={{ id: item.id }}
                style={{ color: 'inherit', textDecoration: 'none' }}
              >
                <Stack direction="row" spacing={2} sx={{ p: 2.5, alignItems: 'center' }}>
                  <Stack spacing={0.25} sx={{ minWidth: 0, flexGrow: 1 }}>
                    <Typography sx={{ fontWeight: 600 }}>{amountLabel(item.level)}</Typography>
                    <Typography variant="body2" color="text.secondary">
                      {item.storage_location}
                      {dateLabel(item) ? ` · ${dateLabel(item)}` : ''}
                    </Typography>
                    {item.note ? <Typography variant="body2">{item.note}</Typography> : null}
                  </Stack>
                  <ChevronRightIcon color="disabled" />
                </Stack>
              </Link>
            </Paper>
          ))}
        </Stack>
      )}

      <NewStockDialog
        open={addOpen}
        onClose={() => setAddOpen(false)}
        product={product.data}
      />
    </>
  );
}
