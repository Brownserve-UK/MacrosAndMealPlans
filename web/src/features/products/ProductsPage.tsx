import AddIcon from '@mui/icons-material/AddOutlined';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Pagination from '@mui/material/Pagination';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';
import type { Product } from '../../api/client';
import { useProducts } from '../../api/queries';
import { RecordListShell, RecordRow } from '../../components/RecordList';
import { PageHeader } from '../../components/PageHeader';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { displayUnit } from '../../components/UnitSelect';
import { useDebounced } from '../../hooks/useDebounced';
import { NewProductDialog } from './NewProductDialog';

type Filter = 'all' | 'unmapped' | 'archived';

const PER_PAGE = 25;

function describe(product: Product): string {
  const parts = [product.brand, product.retailer].filter(Boolean);
  if (product.package_quantity) {
    parts.push(`${product.package_quantity.amount} ${displayUnit(product.package_quantity.unit)}`);
  }
  if (product.servings_per_pack) {
    parts.push(`${product.servings_per_pack} servings`);
  }
  return parts.length > 0 ? parts.join(' · ') : '—';
}

export function ProductsPage() {
  const [search, setSearch] = useState('');
  const [filter, setFilter] = useState<Filter>('all');
  const [page, setPage] = useState(1);
  const [addOpen, setAddOpen] = useState(false);

  const debounced = useDebounced(search, 300);
  const query = useProducts({
    q: debounced || undefined,
    unmapped: filter === 'unmapped' || undefined,
    include_archived: filter === 'archived' || undefined,
    page,
    per_page: PER_PAGE,
  });

  const items = query.data?.items ?? [];
  const total = query.data?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / PER_PAGE));

  return (
    <>
      <PageHeader
        title="Products"
        subtitle="The specific items you buy."
        actions={
          <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
            Add product
          </Button>
        }
        search={{
          value: search,
          onChange: (next) => {
            setSearch(next);
            setPage(1);
          },
          placeholder: 'Search products',
        }}
      />

      <Stack direction="row" spacing={1} sx={{ mb: 2.5, flexWrap: 'wrap', gap: 1 }}>
        {(
          [
            ['all', 'All'],
            ['unmapped', 'No ingredient'],
            ['archived', 'Archived'],
          ] as const
        ).map(([value, label]) => (
          <Chip
            key={value}
            label={label}
            onClick={() => {
              setFilter(value);
              setPage(1);
            }}
            variant={filter === value ? 'filled' : 'outlined'}
            color={filter === value ? 'primary' : 'default'}
          />
        ))}
      </Stack>

      {query.isError ? (
        <ErrorState error={query.error} onRetry={() => query.refetch()} />
      ) : query.isLoading ? (
        <Loading label="Finding products" />
      ) : items.length === 0 ? (
        <EmptyState
          title={search ? 'Nothing matched' : 'No products yet'}
          description={
            search
              ? `Nothing matches "${search}".`
              : 'Products carry the nutrition. Add the things you buy.'
          }
          action={
            <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
              Add product
            </Button>
          }
        />
      ) : (
        <>
          <RecordListShell>
            {items.map((product) => (
              <Link key={product.id} to="/products/$id" params={{ id: product.id }}>
                <RecordRow
                  name={product.name}
                  detail={describe(product)}
                  muted={Boolean(product.archived_at)}
                  trailing={
                    product.nutrition?.energy_kcal != null ? (
                      <Typography
                        className="numeral"
                        variant="body2"
                        color="text.secondary"
                        sx={{ flexShrink: 0 }}
                      >
                        {product.nutrition.energy_kcal} kcal
                      </Typography>
                    ) : (
                      <Chip size="small" variant="outlined" label="No nutrition" />
                    )
                  }
                />
              </Link>
            ))}
          </RecordListShell>

          {pageCount > 1 ? (
            <Stack sx={{ alignItems: 'center', mt: 3 }}>
              <Pagination
                count={pageCount}
                page={page}
                onChange={(_, next) => {
                  setPage(next);
                  window.scrollTo({ top: 0 });
                }}
                shape="rounded"
                color="primary"
              />
            </Stack>
          ) : null}
        </>
      )}

      <NewProductDialog open={addOpen} onClose={() => setAddOpen(false)} />
    </>
  );
}
