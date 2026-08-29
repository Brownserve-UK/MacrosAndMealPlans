import AccessTimeIcon from '@mui/icons-material/AccessTimeOutlined';
import EditIcon from '@mui/icons-material/EditOutlined';
import InfoOutlinedIcon from '@mui/icons-material/InfoOutlined';
import PeopleIcon from '@mui/icons-material/PeopleOutlineOutlined';
import RestaurantIcon from '@mui/icons-material/RestaurantOutlined';
import SoupKitchenIcon from '@mui/icons-material/SoupKitchenOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Divider from '@mui/material/Divider';
import Grid from '@mui/material/Grid';
import IconButton from '@mui/material/IconButton';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Tooltip from '@mui/material/Tooltip';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import { useState, type ReactNode } from 'react';
import { ApiError, type Recipe, type RecipeNutrition } from '../../api/client';
import { useRecipe, useRecipeNutrition, useSetRecipeArchived } from '../../api/queries';
import { BackLabel } from '../../components/BackLink';
import { ConflictDialog } from '../../components/ConflictDialog';
import { PageHeader } from '../../components/PageHeader';
import { RecordMenu } from '../../components/RecordMenu';
import { ErrorState, Loading } from '../../components/States';
import { formatAmount } from '../meal-plan/format';
import { countryLabel, labelMealCategory } from './RecipesPage';
import { RecipeImage } from './RecipeImage';
import { ResolveComponentDialog } from './ResolveComponentDialog';

export function RecipePage({ id }: { id: string }) {
  const query = useRecipe(id);
  const archive = useSetRecipeArchived();
  const [conflict, setConflict] = useState<ApiError | null>(null);
  const [resolving, setResolving] = useState<{ componentId: string; text: string } | null>(null);

  if (query.isLoading) return <Loading label="Loading recipe" />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;
  const recipe = query.data;
  if (!recipe) return null;

  const onToggleArchive = async () => {
    try {
      await archive.mutateAsync({
        id: recipe.id,
        revision: recipe.revision,
        archived: !recipe.archived_at,
      });
    } catch (caught) {
      if (caught instanceof ApiError && caught.isConflict) setConflict(caught);
    }
  };

  const categoryChips = [
    ...recipe.meal_categories.map((category) => ({ key: `meal-${category}`, label: labelMealCategory(category) })),
    ...recipe.country_categories.map((country) => ({ key: `country-${country}`, label: countryLabel(country) })),
  ];
  const hasTime = recipe.preparation_minutes != null || recipe.cooking_minutes != null;
  const hasUnmatched = recipe.components.some(
    (component) => component.requirement.kind === 'unresolved',
  );

  return (
    <>
      <PageHeader
        back={
          <Link to="/recipes" className="app-link">
            <BackLabel>Recipes</BackLabel>
          </Link>
        }
        title={recipe.name}
        subtitle={recipe.description ?? undefined}
        actions={
          <Stack direction="row" spacing={1}>
            <Link to="/recipes/$id/edit" params={{ id: recipe.id }}>
              <Button variant="contained" startIcon={<EditIcon />}>Edit</Button>
            </Link>
            <RecordMenu
              archived={Boolean(recipe.archived_at)}
              onToggleArchive={onToggleArchive}
              updatedAt={recipe.updated_at}
            />
          </Stack>
        }
      />

      {recipe.archived_at ? <Alert severity="info" sx={{ mb: 3 }}>Archived.</Alert> : null}
      {hasUnmatched ? (
        <Alert severity="warning" sx={{ mb: 3 }}>
          Some ingredients aren't matched yet.
        </Alert>
      ) : null}

      <Stack spacing={3.5}>
        <Stack spacing={1.5}>
          <Stack direction="row" spacing={{ xs: 1.5, sm: 2.5 }} useFlexGap sx={{ flexWrap: 'wrap', alignItems: 'center' }}>
            <ServingFact servings={recipe.servings} />
            {hasTime ? <Divider orientation="vertical" flexItem sx={{ display: { xs: 'none', sm: 'block' } }} /> : null}
            <TimeFacts recipe={recipe} />
          </Stack>
          {categoryChips.length > 0 || recipe.tags.length > 0 ? (
            <Stack direction="row" spacing={0.75} useFlexGap sx={{ flexWrap: 'wrap' }}>
              {categoryChips.map((chip) => <Chip key={chip.key} label={chip.label} variant="outlined" />)}
              {recipe.tags.map((tag, index) => (
                <Chip key={`tag-${tag}`} label={tag} sx={index === 0 && categoryChips.length > 0 ? { ml: 1 } : undefined} />
              ))}
            </Stack>
          ) : null}
        </Stack>

        {recipe.photo_version ? (
          <Box sx={{ borderRadius: 3, overflow: 'hidden', aspectRatio: { xs: '4 / 3', md: '16 / 7' } }}>
            <RecipeImage
              id={recipe.id}
              version={recipe.photo_version}
              size="hero"
              alt={recipe.name}
              sx={{ width: '100%', height: '100%', objectFit: 'cover' }}
            />
          </Box>
        ) : null}

        <Grid container spacing={3}>
          <Grid size={{ xs: 12, md: 4 }}>
            <Stack spacing={3}>
              <Paper sx={{ p: { xs: 2.5, md: 3 } }}>
                <Typography variant="h2" sx={{ mb: 2 }}>Ingredients</Typography>
                <Stack spacing={1.5} divider={<Divider flexItem />}>
                  {recipe.components.map((component) => (
                    <Stack key={component.id} spacing={0.75}>
                      <Stack direction="row" sx={{ justifyContent: 'space-between', gap: 2 }}>
                        <Typography>{component.name}</Typography>
                        <Typography color="text.secondary" sx={{ whiteSpace: 'nowrap' }}>
                          {formatAmount(component.amount)}
                        </Typography>
                      </Stack>
                      {component.requirement.kind === 'unresolved' ? (
                        <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                          <Chip size="small" color="warning" variant="outlined" label="Not matched" />
                          <Button
                            size="small"
                            onClick={() =>
                              setResolving({
                                componentId: component.id,
                                text: component.name,
                              })
                            }
                          >
                            Match
                          </Button>
                        </Stack>
                      ) : null}
                    </Stack>
                  ))}
                </Stack>
              </Paper>
              <NutritionPanel recipeId={recipe.id} />
            </Stack>
          </Grid>
          <Grid size={{ xs: 12, md: 8 }}>
            <Paper sx={{ p: { xs: 2.5, md: 4 }, minHeight: '100%' }}>
              <Typography variant="h2" sx={{ mb: 3 }}>Instructions</Typography>
              {recipe.instructions.length === 0 ? (
                <Typography color="text.secondary">No instructions yet.</Typography>
              ) : (
                <Stack spacing={3}>
                  {recipe.instructions.map((instruction, index) => (
                    <Stack key={instruction.id} direction="row" spacing={2.5}>
                      <Box
                        sx={{
                          width: 34,
                          height: 34,
                          borderRadius: '50%',
                          bgcolor: 'primary.main',
                          color: 'primary.contrastText',
                          display: 'grid',
                          placeItems: 'center',
                          flexShrink: 0,
                          fontWeight: 700,
                        }}
                      >
                        {index + 1}
                      </Box>
                      <Typography sx={{ pt: 0.5, whiteSpace: 'pre-wrap' }}>{instruction.text}</Typography>
                    </Stack>
                  ))}
                </Stack>
              )}
            </Paper>
          </Grid>
        </Grid>

        {recipe.notes ? (
          <Paper sx={{ p: 3 }}>
            <Typography variant="h2" sx={{ mb: 1.5 }}>Notes</Typography>
            <Typography sx={{ whiteSpace: 'pre-wrap' }}>{recipe.notes}</Typography>
          </Paper>
        ) : null}

      </Stack>

      <ConflictDialog
        error={conflict}
        onDismiss={() => setConflict(null)}
        onReload={() => {
          setConflict(null);
          void query.refetch();
        }}
      />
      {resolving ? (
        <ResolveComponentDialog
          recipeId={recipe.id}
          revision={recipe.revision}
          componentId={resolving.componentId}
          text={resolving.text}
          onClose={() => setResolving(null)}
          onConflict={setConflict}
        />
      ) : null}
    </>
  );
}

function ServingFact({ servings }: { servings: number }) {
  return (
    <Stack direction="row" spacing={0.75} sx={{ alignItems: 'center' }}>
      <Box sx={{ color: 'warning.main', display: 'grid', placeItems: 'center' }}>
        <PeopleIcon />
      </Box>
      <Stack spacing={0}>
        <Typography variant="caption" sx={{ color: 'text.secondary', fontWeight: 650, lineHeight: 1.2 }}>
          Serves
        </Typography>
        <Typography variant="body2" className="numeral" sx={{ color: 'text.primary', lineHeight: 1.35 }}>
          {servings}
        </Typography>
      </Stack>
    </Stack>
  );
}

function TimeFacts({ recipe }: { recipe: Recipe }) {
  const preparation = recipe.preparation_minutes;
  const cooking = recipe.cooking_minutes;
  if (preparation == null && cooking == null) return null;

  return (
    <Stack direction="row" spacing={{ xs: 1.25, sm: 2 }} useFlexGap sx={{ alignItems: 'stretch', flexWrap: 'wrap' }}>
      {preparation != null && cooking != null ? (
        <TimeFact
          icon={<AccessTimeIcon />}
          label="Total Time"
          minutes={preparation + cooking}
        />
      ) : null}
      {preparation != null && cooking != null ? <Divider orientation="vertical" flexItem sx={{ display: { xs: 'none', sm: 'block' } }} /> : null}
      {preparation != null ? <TimeFact icon={<RestaurantIcon />} label="Prep Time" minutes={preparation} /> : null}
      {preparation != null && cooking != null ? <Divider orientation="vertical" flexItem sx={{ display: { xs: 'none', sm: 'block' } }} /> : null}
      {cooking != null ? <TimeFact icon={<SoupKitchenIcon />} label="Cook Time" minutes={cooking} /> : null}
    </Stack>
  );
}

function TimeFact({ icon, label, minutes }: { icon: ReactNode; label: string; minutes: number }) {
  return (
    <Stack direction="row" spacing={0.75} sx={{ alignItems: 'center' }}>
      <Box sx={{ color: 'warning.main', display: 'grid', placeItems: 'center' }}>{icon}</Box>
      <Stack spacing={0}>
        <Typography variant="caption" sx={{ color: 'text.secondary', fontWeight: 650, lineHeight: 1.2 }}>
          {label}
        </Typography>
        <Typography variant="body2" className="numeral" sx={{ color: 'text.primary', lineHeight: 1.35 }}>
          {minutes} min
        </Typography>
      </Stack>
    </Stack>
  );
}

function NutritionPanel({ recipeId }: { recipeId: string }) {
  const query = useRecipeNutrition(recipeId);
  const quality = query.data?.quality;
  const estimated = quality === 'estimated' || quality === 'partial';
  return (
    <Paper sx={{ p: 3 }}>
      <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center' }}>
        <Typography variant="h2">Nutrition</Typography>
        {quality != null && quality !== 'known' ? <QualityHint quality={quality} /> : null}
      </Stack>
      <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
        {estimated ? 'Per serving (estimated)' : 'Per serving'}
      </Typography>
      {query.data ? <NutritionRows data={query.data} /> : <Loading label="Working it out" />}
    </Paper>
  );
}

const QUALITY_HINT: Record<Exclude<RecipeNutrition['quality'], 'known'>, string> = {
  estimated:
    'Estimated from typical products for each ingredient. Actual values depend on the products you use.',
  partial:
    'Estimated from typical products for each ingredient. Actual values depend on the products you use. Some ingredients have no nutrition data yet.',
  unknown: 'No nutrition data for these ingredients yet.',
};

function QualityHint({ quality }: { quality: Exclude<RecipeNutrition['quality'], 'known'> }) {
  return (
    <Tooltip title={QUALITY_HINT[quality]}>
      <IconButton size="small" aria-label="About this nutrition" sx={{ color: 'text.secondary' }}>
        <InfoOutlinedIcon fontSize="small" />
      </IconButton>
    </Tooltip>
  );
}

const ROWS: { key: 'energy_kcal' | 'protein_g' | 'carbohydrate_g' | 'fat_g'; label: string; unit: string }[] = [
  { key: 'energy_kcal', label: 'Energy', unit: 'kcal' },
  { key: 'protein_g', label: 'Protein', unit: 'g' },
  { key: 'carbohydrate_g', label: 'Carbohydrate', unit: 'g' },
  { key: 'fat_g', label: 'Fat', unit: 'g' },
];

function NutritionRows({ data }: { data: RecipeNutrition }) {
  return (
    <Stack spacing={1.25}>
      {ROWS.map((row) => {
        const value = data.nutrition[row.key];
        return (
          <Stack key={row.key} direction="row" sx={{ justifyContent: 'space-between' }}>
            <Typography variant="body2" color="text.secondary">{row.label}</Typography>
            <Typography
              variant="body2"
              className="numeral"
              sx={value == null ? { color: 'warning.main' } : undefined}
            >
              {value == null ? 'Unknown' : `${Math.round(value * 10) / 10} ${row.unit}`}
            </Typography>
          </Stack>
        );
      })}
    </Stack>
  );
}
