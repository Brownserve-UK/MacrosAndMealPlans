import AddIcon from '@mui/icons-material/AddOutlined';
import AccessTimeIcon from '@mui/icons-material/AccessTimeOutlined';
import PeopleIcon from '@mui/icons-material/PeopleOutlineOutlined';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import Pagination from '@mui/material/Pagination';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import { useState, type ReactNode } from 'react';
import type { RecipeSummary } from '../../api/client';
import { useRecipes } from '../../api/queries';
import { PageHeader } from '../../components/PageHeader';
import { EmptyState, ErrorState, Loading } from '../../components/States';
import { useDebounced } from '../../hooks/useDebounced';
import { RecipeImage } from './RecipeImage';

const PER_PAGE = 24;

export function RecipesPage() {
  const [search, setSearch] = useState('');
  const [showArchived, setShowArchived] = useState(false);
  const [page, setPage] = useState(1);
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
        subtitle="Your recipes, ready when you are."
        actions={
          <Button component={Link} to="/recipes/new" variant="contained" startIcon={<AddIcon />}>
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

      <Chip
        label="Show archived"
        onClick={() => {
          setShowArchived((current) => !current);
          setPage(1);
        }}
        variant={showArchived ? 'filled' : 'outlined'}
        color={showArchived ? 'primary' : 'default'}
        sx={{ mb: 2.5 }}
      />

      {query.isLoading ? (
        <Loading label="Finding recipes" />
      ) : items.length === 0 ? (
        <EmptyState
          title={search ? 'Nothing matched' : 'No recipes yet'}
          description={search ? `Nothing matches "${search}".` : 'Add your first recipe.'}
          action={
            <Button component={Link} to="/recipes/new" variant="contained" startIcon={<AddIcon />}>
              New recipe
            </Button>
          }
        />
      ) : (
        <>
          <Box
            sx={{
              display: 'grid',
              gridTemplateColumns: { xs: '1fr', sm: 'repeat(2, 1fr)', lg: 'repeat(3, 1fr)' },
              gap: 2.5,
            }}
          >
            {items.map((recipe) => (
              <RecipeCard key={recipe.id} recipe={recipe} />
            ))}
          </Box>
          {pageCount > 1 ? (
            <Stack sx={{ alignItems: 'center', mt: 4 }}>
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
    </>
  );
}

function RecipeCard({ recipe }: { recipe: RecipeSummary }) {
  const chips = [
    ...recipe.meal_categories.map(labelMealCategory),
    ...recipe.country_categories.map(countryLabel),
    ...recipe.tags,
  ].slice(0, 3);
  const time = timeLabel(recipe);
  return (
    <Link to="/recipes/$id" params={{ id: recipe.id }} style={{ textDecoration: 'none' }}>
      <Card
        sx={{
          height: '100%',
          overflow: 'hidden',
          transition: 'transform 150ms ease, box-shadow 150ms ease',
          '&:hover': { transform: 'translateY(-2px)', boxShadow: 5 },
          '&:focus-within': { outline: '3px solid', outlineColor: 'primary.main', outlineOffset: 2 },
        }}
      >
        <Box sx={{ position: 'relative', aspectRatio: '16 / 10', bgcolor: 'primary.50' }}>
          {recipe.photo_version ? (
            <RecipeImage
              id={recipe.id}
              version={recipe.photo_version}
              size="card"
              alt=""
              sx={{ width: '100%', height: '100%', objectFit: 'cover' }}
            />
          ) : (
            <Box
              sx={{
                width: '100%',
                height: '100%',
                display: 'grid',
                placeItems: 'center',
                background: 'linear-gradient(135deg, #f4c987 0%, #d9794d 100%)',
              }}
            >
              <Typography variant="h2" sx={{ color: '#442414', opacity: 0.78 }}>
                {recipe.name.slice(0, 1).toUpperCase()}
              </Typography>
            </Box>
          )}
          {recipe.archived_at ? (
            <Chip label="Archived" size="small" sx={{ position: 'absolute', top: 12, right: 12 }} />
          ) : recipe.unresolved_count > 0 ? (
            <Chip
              label="Needs matching"
              color="warning"
              size="small"
              sx={{ position: 'absolute', top: 12, right: 12 }}
            />
          ) : null}
        </Box>
        <Stack spacing={1.5} sx={{ p: 2.5 }}>
          <div>
            <Typography variant="h3" color="text.primary">
              {recipe.name}
            </Typography>
            {recipe.description ? (
              <Typography
                variant="body2"
                color="text.secondary"
                sx={{ mt: 0.75, display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical', overflow: 'hidden' }}
              >
                {recipe.description}
              </Typography>
            ) : null}
          </div>
          <Stack direction="row" spacing={2} color="text.secondary">
            <Fact icon={<PeopleIcon fontSize="inherit" />} text={`Serves ${recipe.servings}`} />
            {time ? <Fact icon={<AccessTimeIcon fontSize="inherit" />} text={time} /> : null}
          </Stack>
          {chips.length > 0 ? (
            <Stack direction="row" spacing={0.75} useFlexGap sx={{ flexWrap: 'wrap' }}>
              {chips.map((chip) => (
                <Chip key={chip} label={chip} size="small" variant="outlined" />
              ))}
            </Stack>
          ) : null}
        </Stack>
      </Card>
    </Link>
  );
}

function Fact({ icon, text }: { icon: ReactNode; text: string }) {
  return (
    <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center', fontSize: 18 }}>
      {icon}
      <Typography variant="caption">{text}</Typography>
    </Stack>
  );
}

function timeLabel(recipe: RecipeSummary) {
  if (recipe.preparation_minutes && recipe.cooking_minutes) {
    return `${recipe.preparation_minutes + recipe.cooking_minutes} min total`;
  }
  if (recipe.preparation_minutes) return `${recipe.preparation_minutes} min prep`;
  if (recipe.cooking_minutes) return `${recipe.cooking_minutes} min cook`;
  return null;
}

export function labelMealCategory(value: string) {
  return value.slice(0, 1).toUpperCase() + value.slice(1);
}

export function countryLabel(code: string) {
  const flag = String.fromCodePoint(...[...code].map((character) => character.charCodeAt(0) + 127397));
  const name = new Intl.DisplayNames(['en'], { type: 'region' }).of(code) ?? code;
  return `${flag} ${name}`;
}
