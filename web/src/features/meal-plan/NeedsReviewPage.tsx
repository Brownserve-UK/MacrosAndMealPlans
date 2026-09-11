import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Tab from '@mui/material/Tab';
import Tabs from '@mui/material/Tabs';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import { ApiError, type MealPlanEntry, type Product } from '../../api/client';
import { Link } from '@tanstack/react-router';
import {
  useMarkMealPlanEaten,
  useMarkMealPlanNotEaten,
  useNeedsReview,
  useSetPreparedMealMapping,
  useSetProductMapping,
  useShoppingList,
} from '../../api/queries';
import { cannotBuyInTime, unreachableMeals } from './unreachable';
import { useAuth } from '../../auth/AuthProvider';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { NewProductDialog } from '../products/NewProductDialog';
import { AteSomethingElseDialog } from './AteSomethingElseDialog';
import { parseIsoDate } from './date';
import { Fact, FactBar, MealCard } from './MealCard';
import { MealOutcomeDialog } from './MealOutcomeDialog';
import { ProductPicker } from './ProductPicker';
import { labelForSlot } from './slots';

function whenLabel(entry: MealPlanEntry) {
  const day = parseIsoDate(entry.planned_on).toLocaleDateString('en-GB', {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
  });
  return entry.planned_time ? `${day} · ${entry.planned_time}` : day;
}

function whenFor(iso: string) {
  return parseIsoDate(iso).toLocaleDateString('en-GB', {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
  });
}

function ReviewCard({
  entry,
  busy,
  onEaten,
  onNotEaten,
  onSomethingElse,
}: {
  entry: MealPlanEntry;
  busy: boolean;
  onEaten: () => void;
  onNotEaten: () => void;
  onSomethingElse: () => void;
}) {
  return (
    <MealCard
      header={
        <Stack direction="row" spacing={2} sx={{ justifyContent: 'space-between', alignItems: 'center' }}>
          <FactBar>
            <Fact label="When" value={whenLabel(entry)} />
            <Fact label="Meal" value={labelForSlot(entry.slot)} />
          </FactBar>
          <Chip size="small" color="warning" variant="outlined" label="Assumed" />
        </Stack>
      }
      foods={entry.components.map((component) => ({
        id: component.id,
        name: component.item_name,
        amount: component.amount,
      }))}
      actions={
        <>
          <Button size="small" variant="contained" disabled={busy} onClick={onEaten}>
            Ate it
          </Button>
          <Button size="small" color="warning" disabled={busy} onClick={onNotEaten}>
            Not eaten
          </Button>
          <Button size="small" disabled={busy} onClick={onSomethingElse}>
            Ate something else
          </Button>
        </>
      }
    />
  );
}

export function NeedsReviewPage() {
  const { principal } = useAuth();
  const memberId = principal?.member_id ?? '';
  const review = useNeedsReview();
  const shopping = useShoppingList(undefined);
  const markEaten = useMarkMealPlanEaten();
  const markNotEaten = useMarkMealPlanNotEaten();
  const setMapping = useSetProductMapping();
  const setPreparedMealMapping = useSetPreparedMealMapping();
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<'personal' | 'household' | 'shopping' | 'ingredients'>(
    'personal',
  );
  const [replacing, setReplacing] = useState<MealPlanEntry | null>(null);
  const [householdReview, setHouseholdReview] = useState<string | null>(null);
  const [creatingIngredient, setCreatingIngredient] = useState<{ id: string; kind: 'ingredient' | 'prepared_meal' } | null>(null);
  const [linkingIngredient, setLinkingIngredient] = useState<{ id: string; kind: 'ingredient' | 'prepared_meal' } | null>(null);
  const [mappingProduct, setMappingProduct] = useState<Product | null>(null);

  const busy = markEaten.isPending || markNotEaten.isPending;

  async function resolve(entry: MealPlanEntry, eaten: boolean) {
    try {
      if (eaten) {
        await markEaten.mutateAsync({
          id: entry.id,
          revision: entry.revision,
          body: {
            consumed_on: entry.planned_on,
            consumed_at: null,
            components: entry.components.map((component) => ({
              component_id: component.id,
              amount: component.amount,
            })),
          },
        });
      } else {
        await markNotEaten.mutateAsync({ id: entry.id, revision: entry.revision });
      }
      setError(null);
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not update this meal.');
    }
  }

  if (review.isError) return <ErrorState error={review.error} onRetry={() => review.refetch()} />;

  const personal = review.data?.personal_meals ?? [];
  const household = review.data?.household_meals ?? [];
  const ingredients = review.data?.food_mappings ?? [];
  const canReviewHousehold = principal?.permissions.includes('household:write') ?? false;
  const canMapIngredients = principal?.permissions.includes('catalogue:write') ?? false;
  const canShop = principal?.permissions.includes('shopping:read') ?? false;
  const unreachable = unreachableMeals(shopping.data);

  async function linkProduct(food: { id: string; kind: 'ingredient' | 'prepared_meal' }) {
    if (!mappingProduct) return;
    try {
      if (food.kind === 'ingredient') {
        await setMapping.mutateAsync({
          id: mappingProduct.id,
          revision: mappingProduct.revision,
          ingredientId: food.id,
        });
      } else {
        await setPreparedMealMapping.mutateAsync({
          id: mappingProduct.id,
          revision: mappingProduct.revision,
          preparedMealId: food.id,
        });
      }
      setLinkingIngredient(null);
      setMappingProduct(null);
      setError(null);
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not link this product.');
    }
  }

  return (
    <Box>
      <PageHeader title="Needs review" subtitle="Things that need your attention." />

      <Tabs
        value={tab}
        onChange={(_, value) => setTab(value)}
        variant="scrollable"
        scrollButtons="auto"
        sx={{ mb: 3 }}
      >
        <Tab value="personal" label={`My meals (${personal.length})`} />
        {canReviewHousehold ? (
          <Tab value="household" label={`Household meals (${household.length})`} />
        ) : null}
        {canShop ? <Tab value="shopping" label={`Shopping (${unreachable.length})`} /> : null}
        {canMapIngredients ? (
          <Tab value="ingredients" label={`Ingredient mappings (${ingredients.length})`} />
        ) : null}
      </Tabs>

      {error ? (
        <Alert severity="error" onClose={() => setError(null)} sx={{ mb: 2 }}>
          {error}
        </Alert>
      ) : null}

      {review.isLoading ? <Loading label="Loading meals" /> : null}

      {review.data && tab === 'personal' && personal.length === 0 ? (
        <Paper variant="outlined" sx={{ px: 3, py: 4 }}>
          <Typography color="text.secondary">No meals need review.</Typography>
        </Paper>
      ) : null}

      {tab === 'personal' && personal.length > 0 ? (
        <Box component="section" sx={{ mb: 4 }}>
          <Stack spacing={2}>
            {personal.map((entry) => (
              <ReviewCard
                key={entry.id}
                entry={entry}
                busy={busy}
                onEaten={() => void resolve(entry, true)}
                onNotEaten={() => void resolve(entry, false)}
                onSomethingElse={() => setReplacing(entry)}
              />
            ))}
          </Stack>
        </Box>
      ) : null}

      {tab === 'household' && household.length === 0 ? (
        <Paper variant="outlined" sx={{ px: 3, py: 4 }}>
          <Typography color="text.secondary">No household meals need review.</Typography>
        </Paper>
      ) : null}

      {tab === 'household' && household.length > 0 ? (
        <Box component="section">
          <Stack spacing={2}>
            {household.map((entry) => (
              <MealCard
                key={entry.id}
                header={
                  <Stack direction="row" spacing={2} sx={{ justifyContent: 'space-between', alignItems: 'center' }}>
                    <FactBar>
                      <Fact label="When" value={whenLabel(entry)} />
                      <Fact label="Meal" value={labelForSlot(entry.slot)} />
                    </FactBar>
                    <Chip
                      size="small"
                      color="warning"
                      variant="outlined"
                      label={entry.status === 'assumed' ? 'Assumed' : 'Partly recorded'}
                    />
                  </Stack>
                }
                foods={entry.components.map((component) => ({
                  id: component.id,
                  name: component.item_name,
                  amount: component.amount,
                }))}
                actions={
                  <Button size="small" variant="contained" onClick={() => setHouseholdReview(entry.id)}>
                    Record results
                  </Button>
                }
              />
            ))}
          </Stack>
        </Box>
      ) : null}

      {tab === 'shopping' && unreachable.length === 0 ? (
        <Paper variant="outlined" sx={{ px: 3, py: 4 }}>
          <Typography color="text.secondary">
            Every planned meal can be shopped for in time.
          </Typography>
        </Paper>
      ) : null}

      {tab === 'shopping' && unreachable.length > 0 ? (
        <Box component="section">
          <Stack spacing={2}>
            {unreachable.map((meal) => (
              <Paper key={meal.entryId} variant="outlined" sx={{ p: 2.5 }}>
                <Stack
                  direction={{ xs: 'column', sm: 'row' }}
                  spacing={2}
                  sx={{ alignItems: { sm: 'center' } }}
                >
                  <Box sx={{ flexGrow: 1, minWidth: 0 }}>
                    <Typography sx={{ fontWeight: 600 }}>
                      {meal.recipeName ?? labelForSlot(meal.slot)}
                    </Typography>
                    <Typography variant="body2" color="text.secondary" className="numeral">
                      {whenFor(meal.plannedOn)} · {labelForSlot(meal.slot)}
                    </Typography>
                    <Typography variant="body2" color="warning.main" sx={{ mt: 0.5 }}>
                      {cannotBuyInTime(meal.names)}
                    </Typography>
                  </Box>
                  <Box sx={{ flexShrink: 0 }}>
                    <Link to="/shopping" className="app-link">
                      <Button variant="outlined" component="span">
                        Add a shop
                      </Button>
                    </Link>
                  </Box>
                </Stack>
              </Paper>
            ))}
          </Stack>
        </Box>
      ) : null}

      {tab === 'ingredients' && ingredients.length === 0 ? (
        <Paper variant="outlined" sx={{ px: 3, py: 4 }}>
          <Typography color="text.secondary">No foods need a product mapping.</Typography>
        </Paper>
      ) : null}

      {tab === 'ingredients' && ingredients.length > 0 ? (
        <Stack spacing={1.5}>
          {ingredients.map((ingredient) => (
            <Paper key={ingredient.id} variant="outlined" sx={{ p: 2.5 }}>
              <Stack spacing={2}>
                <Stack
                  direction={{ xs: 'column', sm: 'row' }}
                  spacing={1}
                  sx={{ justifyContent: 'space-between', alignItems: { sm: 'center' } }}
                >
                  <Typography sx={{ fontWeight: 600 }}>{ingredient.name}</Typography>
                  <Stack direction="row" spacing={1}>
                    <Button size="small" onClick={() => setCreatingIngredient({ id: ingredient.id, kind: ingredient.kind })}>
                      Create product
                    </Button>
                    <Button
                      size="small"
                      variant="contained"
                      onClick={() => {
                        setLinkingIngredient({ id: ingredient.id, kind: ingredient.kind });
                        setMappingProduct(null);
                      }}
                    >
                      Link product
                    </Button>
                  </Stack>
                </Stack>
                {linkingIngredient?.id === ingredient.id ? (
                  <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1.5}>
                    <Box sx={{ flexGrow: 1 }}>
                      <ProductPicker
                        value={mappingProduct}
                        onChange={setMappingProduct}
                        unmappedOnly
                      />
                    </Box>
                    <Button
                      variant="contained"
                      disabled={!mappingProduct || setMapping.isPending || setPreparedMealMapping.isPending}
                      onClick={() => void linkProduct(linkingIngredient)}
                    >
                      Link
                    </Button>
                    <Button onClick={() => setLinkingIngredient(null)}>Cancel</Button>
                  </Stack>
                ) : null}
              </Stack>
            </Paper>
          ))}
        </Stack>
      ) : null}

      {replacing ? (
        <AteSomethingElseDialog
          open
          onClose={() => setReplacing(null)}
          entryId={replacing.id}
          revision={replacing.revision}
          memberId={memberId}
          consumedOn={replacing.planned_on}
        />
      ) : null}

      {householdReview ? (
        <HouseholdReview entryId={householdReview} onClose={() => setHouseholdReview(null)} />
      ) : null}

      {creatingIngredient ? (
        <NewProductDialog
          open
          onClose={() => setCreatingIngredient(null)}
          mappedIngredientId={creatingIngredient.kind === 'ingredient' ? creatingIngredient.id : undefined}
          mappedPreparedMealId={creatingIngredient.kind === 'prepared_meal' ? creatingIngredient.id : undefined}
        />
      ) : null}
    </Box>
  );
}

function HouseholdReview({ entryId, onClose }: { entryId: string; onClose: () => void }) {
  const review = useNeedsReview();
  const entry = review.data?.household_meals.find((candidate) => candidate.id === entryId);
  if (!entry) return null;
  return <MealOutcomeDialog meal={entryToPlannerMeal(entry)} onClose={onClose} />;
}

function entryToPlannerMeal(entry: MealPlanEntry) {
  return {
    id: entry.id,
    scope: entry.scope,
    member_id: entry.member_id ?? undefined,
    owner_name: undefined,
    planned_on: entry.planned_on,
    planned_time: entry.planned_time ?? undefined,
    slot: entry.slot,
    status: entry.status,
    foods: entry.components.map((component) => ({
      id: component.id,
      ...(component.item_kind === 'recipe'
        ? { item_kind: 'recipe' as const, recipe_id: component.recipe_id }
        : component.item_kind === 'dish'
          ? { item_kind: 'dish' as const, dish_recipe_id: component.dish_recipe_id }
          : component.item_kind === 'ingredient'
            ? { item_kind: 'ingredient' as const, ingredient_id: component.ingredient_id }
            : component.item_kind === 'prepared_meal'
              ? { item_kind: 'prepared_meal' as const, prepared_meal_id: component.prepared_meal_id }
              : { item_kind: 'product' as const, product_id: component.product_id }),
      item_name: component.item_name,
      amount: component.amount,
      shortage: component.preparation.shortage,
      needs_cooking: component.needs_cooking,
      cooked: component.cooked,
    })),
    people: entry.participants.map((person) => ({
      member_id: person.member_id,
      display_name: person.display_name,
      status: person.status,
      allocations: person.allocations,
      can_record: true,
    })),
    guest_groups: entry.guest_groups,
    opted_out: entry.opted_out ?? [],
    can_opt_out: false,
    can_join: false,
    capabilities: { can_edit: false, can_delete: false, can_record_guests: true },
    revision: entry.revision,
  };
}
