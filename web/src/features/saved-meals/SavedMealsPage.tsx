import AddIcon from '@mui/icons-material/AddOutlined';
import Button from '@mui/material/Button';
import Pagination from '@mui/material/Pagination';
import Stack from '@mui/material/Stack';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';
import { useMealTemplates } from '../../api/queries';
import { PageHeader } from '../../components/PageHeader';
import { RecordListShell, RecordRow } from '../../components/RecordList';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { useDebounced } from '../../hooks/useDebounced';
import { SavedMealEditorDialog } from './SavedMealEditorDialog';

const PER_PAGE = 25;

function foodCount(count: number): string {
  return count === 1 ? '1 food' : `${count} foods`;
}

export function SavedMealsPage() {
  const [search, setSearch] = useState('');
  const [page, setPage] = useState(1);
  const [addOpen, setAddOpen] = useState(false);
  const debounced = useDebounced(search, 300);

  const query = useMealTemplates({ q: debounced || undefined, page, per_page: PER_PAGE });
  const items = query.data?.items ?? [];
  const total = query.data?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / PER_PAGE));

  return (
    <>
      <PageHeader
        title="Saved meals"
        subtitle="Meals you've built once and can plan again without redoing the work."
        actions={
          <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
            New saved meal
          </Button>
        }
        search={{
          value: search,
          onChange: (next) => {
            setSearch(next);
            setPage(1);
          },
          placeholder: 'Search saved meals',
        }}
      />

      {query.isError ? (
        <ErrorState error={query.error} onRetry={() => query.refetch()} />
      ) : query.isLoading ? (
        <Loading label="Finding saved meals" />
      ) : items.length === 0 ? (
        search ? (
          <EmptyState title="Nothing matched" description={`Nothing matches "${search}".`} />
        ) : (
          <EmptyState
            title="Nothing saved yet"
            description="Save a meal you've planned and it'll appear here, ready to plan again."
            action={
              <Button variant="contained" startIcon={<AddIcon />} onClick={() => setAddOpen(true)}>
                New saved meal
              </Button>
            }
          />
        )
      ) : (
        <>
          <RecordListShell>
            {items.map((template) => (
              <Link key={template.id} to="/saved-meals/$id" params={{ id: template.id }}>
                <RecordRow name={template.name} detail={foodCount(template.components.length)} />
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

      <SavedMealEditorDialog open={addOpen} onClose={() => setAddOpen(false)} template={null} />
    </>
  );
}
