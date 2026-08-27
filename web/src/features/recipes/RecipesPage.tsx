import AddIcon from '@mui/icons-material/AddOutlined';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Pagination from '@mui/material/Pagination';
import Stack from '@mui/material/Stack';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';
import { useRecipes } from '../../api/queries';
import { PageHeader } from '../../components/PageHeader';
import { RecordListShell, RecordRow } from '../../components/RecordList';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { useDebounced } from '../../hooks/useDebounced';
import { NewRecipeDialog } from './NewRecipeDialog';

const PER_PAGE = 25;

function describe(servings: number, componentCount: number): string {
  const serves = `Serves ${servings}`;
  const products = componentCount === 1 ? '1 product' : `${componentCount} products`;
  return `${serves} · ${products}`;
}

export function RecipesPage() {
  const [search, setSearch] = useState('');
  const [showArchived, setShowArchived] = useState(false);
  const [page, setPage] = useState(1);
  const [addOpen, setAddOpen] = useState(false);

  const debounced = useDebounced(search, 300);
  const query = useRecipes({
    q: debounced || undefined,
    include_archived: showArchived || undefined,
    page,
    per_page: PER_PAGE,
  });

  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;

  const items = query.data?.items ?? [];
  const total = query.data?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / PER_PAGE));

  return (
    <>
      <PageHeader
        title="Recipes"
        subtitle="Reusable meals built from products, with their nutrition worked out for you."
        actions={
          <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
            New recipe
          </Button>
        }
        search={{
          value: search,
          onChange: (next) => {
            setSearch(next);
            setPage(1);
          },
          placeholder: 'Search recipes',
        }}
      />

      <Stack direction="row" spacing={1} sx={{ mb: 2.5 }}>
        <Chip
          label="Show archived"
          onClick={() => {
            setShowArchived((prev) => !prev);
            setPage(1);
          }}
          variant={showArchived ? 'filled' : 'outlined'}
          color={showArchived ? 'primary' : 'default'}
        />
      </Stack>

      {query.isLoading ? (
        <Loading label="Finding recipes" />
      ) : items.length === 0 ? (
        <EmptyState
          title={search ? 'Nothing matched' : 'No recipes yet'}
          description={
            search
              ? `Nothing matches "${search}".`
              : 'Create a recipe from the products you cook with, like a chicken curry or a smoothie.'
          }
          action={
            <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
              New recipe
            </Button>
          }
        />
      ) : (
        <>
          <RecordListShell>
            {items.map((recipe) => (
              <Link key={recipe.id} to="/recipes/$id" params={{ id: recipe.id }}>
                <RecordRow
                  name={recipe.name}
                  detail={describe(recipe.servings, recipe.components.length)}
                  muted={Boolean(recipe.archived_at)}
                  trailing={
                    recipe.archived_at ? (
                      <Chip size="small" variant="outlined" label="Archived" />
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

      <NewRecipeDialog open={addOpen} onClose={() => setAddOpen(false)} />
    </>
  );
}
