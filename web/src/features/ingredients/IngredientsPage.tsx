import AddIcon from '@mui/icons-material/AddOutlined';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Pagination from '@mui/material/Pagination';
import Stack from '@mui/material/Stack';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';
import type { components } from '../../api/schema';
import { useIngredients } from '../../api/queries';
import { RecordListShell, RecordRow } from '../../components/RecordList';
import { PageHeader } from '../../components/PageHeader';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { useDebounced } from '../../hooks/useDebounced';
import { NewIngredientDialog } from './NewIngredientDialog';

type Item = components['schemas']['IngredientListItemDto'];

type Filter = 'all' | 'needs_products' | 'archived';

const PER_PAGE = 25;

function describe(item: Item): string {
  const count = item.mapped_product_count;
  if (count === 0) return 'No products';
  return count === 1 ? '1 product' : `${count} products`;
}

export function IngredientsPage() {
  const [search, setSearch] = useState('');
  const [filter, setFilter] = useState<Filter>('all');
  const [page, setPage] = useState(1);
  const [addOpen, setAddOpen] = useState(false);

  const debounced = useDebounced(search, 300);
  const query = useIngredients({
    q: debounced || undefined,
    include_archived: filter === 'archived' || undefined,
    page,
    per_page: PER_PAGE,
  });

  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;

  const all = query.data?.items ?? [];
  const items = filter === 'needs_products' ? all.filter((i) => i.mapped_product_count === 0) : all;
  const total = query.data?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / PER_PAGE));

  return (
    <>
      <PageHeader
        title="Ingredients"
        subtitle="Generic foods that recipes ask for."
        actions={
          <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
            Add ingredient
          </Button>
        }
        search={{
          value: search,
          onChange: (next) => {
            setSearch(next);
            setPage(1);
          },
          placeholder: 'Search ingredients',
        }}
      />

      <Stack direction="row" spacing={1} sx={{ mb: 2.5, flexWrap: 'wrap', gap: 1 }}>
        {(
          [
            ['all', 'All'],
            ['needs_products', 'No products'],
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

      {query.isLoading ? (
        <Loading label="Finding ingredients" />
      ) : items.length === 0 ? (
        <EmptyState
          title={search ? 'Nothing matched' : 'No ingredients yet'}
          description={
            search
              ? `Nothing matches "${search}".`
              : 'Add the generic foods your recipes use, like whole milk or basmati rice.'
          }
          action={
            <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
              Add ingredient
            </Button>
          }
        />
      ) : (
        <>
          <RecordListShell>
            {items.map((item) => (
              <Link key={item.id} to="/ingredients/$id" params={{ id: item.id }}>
                <RecordRow
                  name={item.name}
                  detail={describe(item)}
                  muted={Boolean(item.archived_at)}
                  trailing={
                    item.mapped_product_count === 0 ? (
                      <Chip size="small" variant="outlined" label="No products" />
                    ) : null
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

      <NewIngredientDialog open={addOpen} onClose={() => setAddOpen(false)} />
    </>
  );
}
