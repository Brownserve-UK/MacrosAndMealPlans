import AddIcon from '@mui/icons-material/AddOutlined';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import MenuItem from '@mui/material/MenuItem';
import Pagination from '@mui/material/Pagination';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';
import type { components } from '../../api/schema';
import type { IngredientListParams } from '../../api/queries';
import { useIngredients } from '../../api/queries';
import { RecordListShell, RecordRow } from '../../components/RecordList';
import { PageHeader } from '../../components/PageHeader';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { useDebounced } from '../../hooks/useDebounced';
import { NewIngredientDialog } from './NewIngredientDialog';

type Item = components['schemas']['IngredientListItemDto'];

type Filter = 'all' | 'needs_products' | 'archived';

type SortKey = 'name' | 'created' | 'product_count';

type SortOption = {
  value: SortKey;
  label: string;
  sort_by: IngredientListParams['sort_by'];
  sort: IngredientListParams['sort'];
};

const SORTS = [
  { value: 'name', label: 'A-Z', sort_by: 'name', sort: 'asc' },
  { value: 'created', label: 'Date added', sort_by: 'created', sort: 'desc' },
  { value: 'product_count', label: 'Product count', sort_by: 'product_count', sort: 'desc' },
] as const satisfies readonly SortOption[];

const PER_PAGE = 25;

function describe(item: Item): string {
  const count = item.mapped_product_count;
  if (count === 0) return 'No products';
  return count === 1 ? '1 product' : `${count} products`;
}

export function IngredientsPage() {
  const [search, setSearch] = useState('');
  const [filter, setFilter] = useState<Filter>('all');
  const [sort, setSort] = useState<SortKey>('name');
  const [page, setPage] = useState(1);
  const [addOpen, setAddOpen] = useState(false);

  const debounced = useDebounced(search, 300);
  const chosen = SORTS.find((option) => option.value === sort) ?? SORTS[0];
  const query = useIngredients({
    q: debounced || undefined,
    needs_products: filter === 'needs_products' || undefined,
    include_archived: filter === 'archived' || undefined,
    sort_by: chosen.sort_by,
    sort: chosen.sort,
    page,
    per_page: PER_PAGE,
  });

  const items = query.data?.items ?? [];
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

      <Stack
        direction="row"
        spacing={1}
        sx={{ mb: 2.5, flexWrap: 'wrap', gap: 1, alignItems: 'center' }}
      >
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

        <TextField
          select
          size="small"
          label="Sort"
          value={sort}
          onChange={(event) => {
            setSort(event.target.value as SortKey);
            setPage(1);
          }}
          sx={{ ml: 'auto', minWidth: 168 }}
        >
          {SORTS.map((option) => (
            <MenuItem key={option.value} value={option.value}>
              {option.label}
            </MenuItem>
          ))}
        </TextField>
      </Stack>

      {query.isError ? (
        <ErrorState error={query.error} onRetry={() => query.refetch()} />
      ) : query.isLoading ? (
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
