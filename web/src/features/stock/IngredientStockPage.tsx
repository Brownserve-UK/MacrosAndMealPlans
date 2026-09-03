import Box from '@mui/material/Box';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import type { StockItem } from '../../api/client';
import {
  useIngredient,
  useIngredientProducts,
  useStock,
  useStockAvailability,
} from '../../api/queries';
import { BackLabel } from '../../components/BackLink';
import { PageHeader } from '../../components/PageHeader';
import { RecordListShell } from '../../components/RecordList';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { locationSubtitle, StockRow } from './StockCard';
import { levelFor } from './stockLevel';

export function IngredientStockPage({ ingredientId }: { ingredientId: string }) {
  const ingredient = useIngredient(ingredientId);
  const products = useIngredientProducts(ingredientId);
  const stock = useStock({ per_page: 200 });
  const availability = useStockAvailability();

  if (ingredient.isLoading || products.isLoading || stock.isLoading || availability.isLoading) {
    return <Loading label="Loading ingredient stock" />;
  }
  if (ingredient.isError || !ingredient.data) {
    return <ErrorState error={ingredient.error} onRetry={() => ingredient.refetch()} />;
  }
  if (stock.isError) return <ErrorState error={stock.error} onRetry={() => stock.refetch()} />;
  if (availability.isError) {
    return <ErrorState error={availability.error} onRetry={() => availability.refetch()} />;
  }

  const pool = products.data?.items ?? [];
  const itemsByProduct = new Map<string, StockItem[]>();
  for (const item of stock.data?.items ?? []) {
    const held = itemsByProduct.get(item.product_id) ?? [];
    held.push(item);
    itemsByProduct.set(item.product_id, held);
  }

  const row = availability.data?.ingredients.find(
    (candidate) => candidate.ingredient_id === ingredientId,
  );
  const level = levelFor(row?.availability ?? null);
  const held = pool.filter((product) => (itemsByProduct.get(product.id) ?? []).length > 0);

  const availabilityByProduct = new Map(
    (availability.data?.products ?? []).map((candidate) => [candidate.product_id, candidate]),
  );

  return (
    <>
      <PageHeader
        back={
          <Link to="/stock" className="app-link">
            <BackLabel>Stock</BackLabel>
          </Link>
        }
        title={ingredient.data.name}
        subtitle="Stock on hand for this ingredient."
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

      <Typography variant="h2" sx={{ mb: 2 }}>
        Products
      </Typography>
      {held.length === 0 ? (
        <EmptyState title="No stock on hand" description="Add stock for one of these products." />
      ) : (
        <RecordListShell>
          {held.map((product) => {
            const items = itemsByProduct.get(product.id) ?? [];
            return (
              <Link
                key={product.id}
                to="/stock/products/$productId"
                params={{ productId: product.id }}
                style={{ textDecoration: 'none', color: 'inherit' }}
              >
                <StockRow
                  testId={`stock-card-${product.id}`}
                  name={product.name}
                  subtitle={locationSubtitle(items)}
                  availability={availabilityByProduct.get(product.id)?.availability ?? null}
                />
              </Link>
            );
          })}
        </RecordListShell>
      )}
    </>
  );
}
