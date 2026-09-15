import AddIcon from '@mui/icons-material/AddOutlined';
import CheckIcon from '@mui/icons-material/CheckOutlined';
import DeleteIcon from '@mui/icons-material/DeleteOutlineOutlined';
import RemoveIcon from '@mui/icons-material/RemoveOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import IconButton from '@mui/material/IconButton';
import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Tooltip from '@mui/material/Tooltip';
import Typography from '@mui/material/Typography';
import { useMemo, useState } from 'react';
import type { Amount, Ingredient, MealSlot, PlannerMeal, PreparedMeal, Product, RecipeSummary, Unit } from '../../api/client';
import { ApiError } from '../../api/client';
import {
  useCreateMealPlanEntry,
  useHouseholdSettings,
  useHouseholdSlotAttendance,
  useMembers,
  useRecipeNutrition,
  useUpdateMealPlanEntry,
} from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { FormDialog } from '../../components/FormDialog';
import { displayUnit } from '../../components/UnitSelect';
import { parseIsoDate } from './date';
import { FoodSearch, type Dish, type FoodChoice } from './FoodSearch';
import type { PlannableDish } from './UseItUp';

type EditorMode = 'member' | 'household';

type FoodDraft = {
  componentId: string;
  itemKind: 'product' | 'recipe' | 'dish' | 'ingredient' | 'prepared_meal';
  itemId: string;
  name: string;
  amount: Amount;
};

const UNITS: Unit[] = ['mg', 'g', 'kg', 'oz', 'lb', 'ml', 'l', 'tsp', 'tbsp', 'fl_oz', 'cup', 'item', 'piece', 'slice', 'clove', 'can', 'pack', 'bunch'];

function amountValue(amount: Amount) {
  return String(amount.value);
}

function withAmountValue(amount: Amount, raw: string): Amount {
  return { ...amount, value: Number(raw) || 0 };
}

function equalShare(amount: Amount, dinerCount: number): Amount {
  return { ...amount, value: amount.value / Math.max(dinerCount, 1) };
}

function itemIdOf(food: PlannerMeal['foods'][number]): string {
  switch (food.item_kind) {
    case 'product':
      return food.product_id;
    case 'dish':
      return food.dish_recipe_id;
    case 'recipe':
      return food.recipe_id;
    case 'ingredient':
      return food.ingredient_id;
    case 'prepared_meal':
      return food.prepared_meal_id;
    default:
      return food satisfies never;
  }
}

function initialFoods(meal: PlannerMeal | null): FoodDraft[] {
  return (meal?.foods ?? []).map((food) => ({
    componentId: food.id,
    itemKind: food.item_kind,
    itemId: itemIdOf(food),
    name: food.item_name,
    amount: food.amount,
  }));
}

function initialDiners(meal: PlannerMeal | null, household: boolean): number {
  if (!household) return 1;
  const guests = meal?.guest_groups.reduce((sum, group) => sum + group.count, 0) ?? 0;
  return (meal?.people.length ?? 0) + guests;
}

function initialMaking(meal: PlannerMeal | null, household: boolean): number | null {
  const recipe = (meal?.foods ?? []).find((food) => food.amount.kind === 'servings');
  if (!recipe) return null;
  const planned = Math.round(recipe.amount.value);
  return planned > initialDiners(meal, household) ? planned : null;
}

function RecipeKcal({ recipeId }: { recipeId: string }) {
  const nutrition = useRecipeNutrition(recipeId);
  const kcal = nutrition.data?.nutrition.energy_kcal;
  if (kcal == null) return null;
  return (
    <Typography variant="caption" color="text.secondary">
      {`${Math.round(kcal)} kcal a serving`}
    </Typography>
  );
}

export function MealEditorDialog({
  open,
  onClose,
  date,
  slot,
  meal,
  mode,
  startWith,
}: {
  open: boolean;
  onClose: () => void;
  date: string;
  slot: MealSlot;
  meal: PlannerMeal | null;
  mode: EditorMode;
  startWith?: PlannableDish | null;
}) {
  const { principal } = useAuth();
  const household = mode === 'household';
  const members = useMembers({ include_archived: false, per_page: 200 });
  const mealTimes = useHouseholdSettings();
  const create = useCreateMealPlanEntry();
  const update = useUpdateMealPlanEntry();
  const [plannedTimeOverride, setPlannedTimeOverride] = useState<string | null>(
    meal ? meal.planned_time ?? '' : null,
  );
  const mealNoun = slot === 'snacks' ? 'snack' : 'meal';
  const [selectedMembers, setSelectedMembers] = useState<string[]>(
    household
      ? meal?.people.map((person) => person.member_id) ?? []
      : principal?.member_id
        ? [principal.member_id]
        : [],
  );
  const [guestCount, setGuestCount] = useState(meal?.guest_groups.reduce((sum, group) => sum + group.count, 0) ?? 0);
  const [foods, setFoods] = useState<FoodDraft[]>(() => {
    const existing = initialFoods(meal);
    if (!startWith || existing.some((food) => food.itemId === startWith.recipeId)) {
      return existing;
    }
    return [...existing, {
      componentId: crypto.randomUUID(),
      itemKind: 'dish',
      itemId: startWith.recipeId,
      name: startWith.name,
      amount: { kind: 'servings', value: 1 },
    }];
  });
  const [making, setMaking] = useState<number | null>(() => initialMaking(meal, household));
  const [error, setError] = useState<string | null>(null);
  const busy = create.isPending || update.isPending;

  const attendance = useHouseholdSlotAttendance(
    household ? date : '',
    household ? slot : '',
    meal?.id,
  );
  const attendanceByMember = useMemo(() => {
    const map = new Map<string, { state: string; claimedTime: string | null }>();
    for (const row of attendance.data ?? []) {
      map.set(row.member_id, { state: row.attendance, claimedTime: row.claimed_time ?? null });
    }
    return map;
  }, [attendance.data]);

  const visibleMembers = useMemo(() => members.data?.items ?? [], [members.data]);
  const diners = household ? selectedMembers.length + guestCount : 1;
  const forecast = Math.max(making ?? diners, diners);
  const hasRecipe = foods.some((food) => food.itemKind === 'recipe');
  const plannedTime = plannedTimeOverride
    ?? (slot === 'snacks' ? '' : mealTimes.data?.[slot] ?? '');
  const slotLabel = slot === 'snacks' ? 'snack' : slot;
  const dateLabel = parseIsoDate(date).toLocaleDateString('en-GB', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  });

  function servingsFor(food: FoodDraft): Amount {
    return food.itemKind === 'recipe' ? { kind: 'servings', value: forecast } : food.amount;
  }

  function dishIds(): string[] {
    return foods.filter((food) => food.itemKind === 'dish').map((food) => food.itemId);
  }

  function memberBlockedReason(memberId: string): string | null {
    if (meal?.people.some((person) => person.member_id === memberId)) return null;
    const row = attendanceByMember.get(memberId);
    if (!row) return null;
    if (row.state === 'self_catering') return 'Has their own plan';
    if (row.state === 'opted_out') return 'Opted out';
    if (row.state === 'participating') {
      return row.claimedTime ? `Already eating at ${row.claimedTime}` : 'Already in another meal';
    }
    return null;
  }

  function addProduct(next: Product) {
    if (foods.some((food) => food.itemKind === 'product' && food.itemId === next.id)) return;
    setFoods((current) => [...current, {
      componentId: crypto.randomUUID(),
      itemKind: 'product',
      itemId: next.id,
      name: next.name,
      amount: {
        kind: 'measure',
        unit: next.nutrition.basis?.unit ?? next.package_quantity?.unit ?? 'g',
        value: next.nutrition.basis?.amount ?? 100,
      },
    }]);
  }

  function addRecipe(next: RecipeSummary) {
    if (foods.some((food) => food.itemKind === 'recipe' && food.itemId === next.id)) return;
    setFoods((current) => [...current, {
      componentId: crypto.randomUUID(),
      itemKind: 'recipe',
      itemId: next.id,
      name: next.name,
      amount: { kind: 'servings', value: Math.max(forecast, 1) },
    }]);
  }

  function addDish(next: Dish) {
    if (foods.some((food) => food.itemKind === 'dish' && food.itemId === next.recipeId)) return;
    setFoods((current) => [...current, {
      componentId: crypto.randomUUID(),
      itemKind: 'dish',
      itemId: next.recipeId,
      name: next.name,
      amount: { kind: 'servings', value: Math.min(Math.max(forecast, 1), next.servings) },
    }]);
  }

  function addIngredient(next: Ingredient) {
    if (foods.some((food) => food.itemKind === 'ingredient' && food.itemId === next.id)) return;
    setFoods((current) => [...current, {
      componentId: crypto.randomUUID(),
      itemKind: 'ingredient',
      itemId: next.id,
      name: next.name,
      amount: { kind: 'measure', unit: next.default_unit, value: 100 },
    }]);
  }

  function addPreparedMeal(next: PreparedMeal) {
    if (foods.some((food) => food.itemKind === 'prepared_meal' && food.itemId === next.id)) return;
    setFoods((current) => [...current, {
      componentId: crypto.randomUUID(),
      itemKind: 'prepared_meal',
      itemId: next.id,
      name: next.name,
      amount: { kind: 'measure', unit: next.default_unit, value: 100 },
    }]);
  }

  function addSavedMeal(template: FoodChoice & { kind: 'saved_meal' }) {
    setFoods((current) => {
      const additions: FoodDraft[] = [];
      for (const component of template.savedMeal.components) {
        let itemKind: FoodDraft['itemKind'];
        let itemId: string;
        switch (component.item_kind) {
          case 'product':
            itemKind = 'product';
            itemId = component.product_id;
            break;
          case 'recipe':
            itemKind = 'recipe';
            itemId = component.recipe_id;
            break;
          case 'ingredient':
            itemKind = 'ingredient';
            itemId = component.ingredient_id;
            break;
          case 'prepared_meal':
            itemKind = 'prepared_meal';
            itemId = component.prepared_meal_id;
            break;
          default:
            continue;
        }
        const already =
          current.some((food) => food.itemKind === itemKind && food.itemId === itemId) ||
          additions.some((food) => food.itemKind === itemKind && food.itemId === itemId);
        if (already) continue;
        additions.push({
          componentId: crypto.randomUUID(),
          itemKind,
          itemId,
          name: component.item_name,
          amount: component.amount,
        });
      }
      return [...current, ...additions];
    });
  }

  function addFood(choice: FoodChoice) {
    if (choice.kind === 'product') addProduct(choice.product);
    else if (choice.kind === 'dish') addDish(choice.dish);
    else if (choice.kind === 'recipe') addRecipe(choice.recipe);
    else if (choice.kind === 'ingredient') addIngredient(choice.ingredient);
    else if (choice.kind === 'prepared_meal') addPreparedMeal(choice.preparedMeal);
    else addSavedMeal(choice);
  }

  function setFoodAmount(componentId: string, amount: Amount) {
    setFoods((current) => current.map((food) => food.componentId === componentId ? { ...food, amount } : food));
  }

  function toggleMember(memberId: string) {
    setSelectedMembers((current) => current.includes(memberId)
      ? current.filter((id) => id !== memberId)
      : [...current, memberId]);
  }

  async function save() {
    if (foods.length === 0) {
      setError('Add at least one food.');
      return;
    }
    if (foods.some((food) =>
      food.itemKind !== 'recipe' && food.itemKind !== 'dish'
      && (!Number.isFinite(food.amount.value) || food.amount.value <= 0))) {
      setError('Every food needs an amount greater than zero.');
      return;
    }
    if (household && diners === 0) {
      setError(`Choose who is eating this ${mealNoun}.`);
      return;
    }
    setError(null);

    const components = foods.map((food) => ({
      id: food.componentId,
      ...(food.itemKind === 'product'
        ? { item_kind: 'product' as const, product_id: food.itemId }
        : food.itemKind === 'dish'
          ? { item_kind: 'dish' as const, dish_recipe_id: food.itemId }
          : food.itemKind === 'ingredient'
            ? { item_kind: 'ingredient' as const, ingredient_id: food.itemId }
            : food.itemKind === 'prepared_meal'
              ? { item_kind: 'prepared_meal' as const, prepared_meal_id: food.itemId }
              : { item_kind: 'recipe' as const, recipe_id: food.itemId }),
      amount: servingsFor(food),
    }));
    const participants = household
      ? selectedMembers.map((memberId) => ({ member_id: memberId, allocations: [] }))
      : undefined;
    const guestAllocations = household && guestCount > 0
      ? foods.map((food) => ({
          component_id: food.componentId,
          amount: food.itemKind === 'recipe'
            ? { kind: 'servings' as const, value: 1 }
            : equalShare(food.amount, diners),
        }))
      : [];

    try {
      if (meal) {
        await update.mutateAsync({
          id: meal.id,
          revision: meal.revision,
          body: {
            planned_on: date,
            slot,
            planned_time: plannedTime || null,
            components,
            ...(household ? { participants, guest_count: guestCount, guest_allocations: guestAllocations } : {}),
          },
        });
      } else {
        await create.mutateAsync({
          planned_on: date,
          slot,
          planned_time: plannedTime || null,
          household,
          components,
          participants,
          guest_count: household ? guestCount : 0,
          guest_allocations: guestAllocations,
        });
      }
      onClose();
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not save this meal.');
    }
  }

  const spare = forecast - diners;

  return (
    <FormDialog open={open} onClose={busy ? undefined : onClose} fullWidth maxWidth="sm">
      <DialogTitle sx={{ pb: 1 }}>
        <Typography component="span" variant="h2">{meal ? `Edit ${slotLabel}` : `Plan ${slotLabel}`}</Typography>
        <Typography component="span" variant="body2" color="text.secondary" sx={{ display: 'block', mt: 0.5 }}>{dateLabel}</Typography>
      </DialogTitle>
      <DialogContent dividers>
        <Stack spacing={2.5}>
          {error ? <Alert severity="error">{error}</Alert> : null}

          <TextField
            label={slot === 'snacks' ? 'Time (optional)' : 'Time'}
            type="time"
            value={plannedTime}
            onChange={(event) => setPlannedTimeOverride(event.target.value)}
            slotProps={{ inputLabel: { shrink: true } }}
            sx={{ width: { xs: '100%', sm: 180 } }}
          />

          <Box>
            <Typography variant="h3" sx={{ mb: 1 }}>Food</Typography>
            <Stack spacing={1.5}>
              <FoodSearch
                onPick={addFood}
                excludeProductIds={foods.filter((food) => food.itemKind === 'product').map((food) => food.itemId)}
                excludeRecipeIds={foods.filter((food) => food.itemKind === 'recipe').map((food) => food.itemId)}
                excludeDishIds={dishIds()}
                excludeIngredientIds={foods.filter((food) => food.itemKind === 'ingredient').map((food) => food.itemId)}
                excludePreparedMealIds={foods.filter((food) => food.itemKind === 'prepared_meal').map((food) => food.itemId)}
              />
              {foods.map((food) => (
                <Box key={food.componentId} sx={{ border: '1px solid', borderColor: 'divider', borderRadius: 2, p: 1.5 }}>
                  <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
                    <Box sx={{ flex: 1, minWidth: 0 }}>
                      <Typography>{food.name}</Typography>
                      {food.itemKind === 'recipe' ? <RecipeKcal recipeId={food.itemId} /> : null}
                    </Box>
                    {food.itemKind === 'product' || food.itemKind === 'ingredient' || food.itemKind === 'prepared_meal' ? (
                      <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                        <TextField
                          label="Amount"
                          type="number"
                          value={amountValue(food.amount)}
                          onChange={(event) => setFoodAmount(food.componentId, withAmountValue(food.amount, event.target.value))}
                          slotProps={{ htmlInput: { min: 0, step: 'any' } }}
                          sx={{ width: 110 }}
                        />
                        {food.amount.kind === 'measure' ? (
                          <TextField
                            select
                            label="Unit"
                            value={food.amount.unit}
                            onChange={(event) => setFoodAmount(food.componentId, { kind: 'measure', value: food.amount.value, unit: event.target.value as Unit })}
                            sx={{ width: 100 }}
                          >
                            {UNITS.map((unit) => <MenuItem key={unit} value={unit}>{displayUnit(unit)}</MenuItem>)}
                          </TextField>
                        ) : null}
                      </Stack>
                    ) : food.itemKind === 'dish' ? (
                      <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center' }}>
                        <IconButton
                          size="small"
                          aria-label={`Less ${food.name}`}
                          disabled={food.amount.value <= 1}
                          onClick={() => setFoodAmount(food.componentId, { kind: 'servings', value: food.amount.value - 1 })}
                        >
                          <RemoveIcon fontSize="small" />
                        </IconButton>
                        <Typography className="numeral" sx={{ minWidth: 46, textAlign: 'center' }}>
                          {food.amount.value === 1 ? '1 serving' : `${food.amount.value} servings`}
                        </Typography>
                        <IconButton
                          size="small"
                          aria-label={`More ${food.name}`}
                          onClick={() => setFoodAmount(food.componentId, { kind: 'servings', value: food.amount.value + 1 })}
                        >
                          <AddIcon fontSize="small" />
                        </IconButton>
                      </Stack>
                    ) : null}
                    <IconButton aria-label={`Remove ${food.name}`} onClick={() => setFoods((current) => current.filter((candidate) => candidate.componentId !== food.componentId))}>
                      <DeleteIcon />
                    </IconButton>
                  </Stack>
                </Box>
              ))}
            </Stack>
          </Box>

          {household ? (
            <Box>
              <Stack direction="row" spacing={1} sx={{ alignItems: 'baseline', mb: 1 }}>
                <Typography variant="h3">Eating</Typography>
                {attendance.isFetching ? <Typography variant="caption" color="text.secondary">Checking who's free…</Typography> : null}
              </Stack>
              <Stack direction="row" spacing={1} sx={{ flexWrap: 'wrap', gap: 1, alignItems: 'center' }}>
                {visibleMembers.map((member) => {
                  const blocked = memberBlockedReason(member.id);
                  const picked = selectedMembers.includes(member.id);
                  const chip = (
                    <Chip
                      label={member.display_name}
                      icon={picked ? <CheckIcon /> : undefined}
                      aria-disabled={blocked ? true : undefined}
                      aria-pressed={blocked ? undefined : picked}
                      onClick={blocked ? undefined : () => toggleMember(member.id)}
                      disabled={attendance.isFetching}
                      variant="outlined"
                      sx={{
                        borderRadius: 999,
                        height: 36,
                        px: 0.5,
                        ...(picked
                          ? { bgcolor: 'action.selected', borderColor: 'primary.main', color: 'primary.main' }
                          : {}),
                        ...(blocked
                          ? { borderStyle: 'dashed', color: 'text.disabled', cursor: 'default' }
                          : {}),
                      }}
                    />
                  );
                  return blocked
                    ? <Tooltip key={member.id} title={blocked}><span>{chip}</span></Tooltip>
                    : <Box key={member.id} component="span">{chip}</Box>;
                })}
                <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center', bgcolor: 'action.hover', borderRadius: 2, pl: 1.5, height: 36 }}>
                  <Typography variant="body2" color="text.secondary">Guests</Typography>
                  <IconButton size="small" aria-label="Remove guest" disabled={guestCount === 0} onClick={() => setGuestCount(Math.max(0, guestCount - 1))}><RemoveIcon fontSize="small" /></IconButton>
                  <Typography className="numeral" sx={{ minWidth: 16, textAlign: 'center' }}>{guestCount}</Typography>
                  <IconButton size="small" aria-label="Add guest" onClick={() => setGuestCount(guestCount + 1)}><AddIcon fontSize="small" /></IconButton>
                </Stack>
              </Stack>
            </Box>
          ) : null}

          {hasRecipe ? (
            <Stack
              direction="row"
              spacing={2}
              sx={{ alignItems: 'center', bgcolor: 'action.hover', borderRadius: 2.5, px: 1.75, py: 1.5 }}
            >
              <Box sx={{ flex: 1, minWidth: 0 }}>
                <Typography
                  variant="subtitle2"
                  className="numeral"
                  color={diners > 0 ? 'text.primary' : 'text.disabled'}
                >
                  {diners === 0
                    ? 'Nobody yet'
                    : forecast === 1 ? 'About 1 serving' : `About ${forecast} servings`}
                </Typography>
                <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
                  {diners === 0
                    ? `Pick who this ${mealNoun} is for.`
                    : spare > 0
                      ? `${spare} spare for the freezer`
                      : 'For the shopping list'}
                </Typography>
              </Box>
              {diners > 0 && making === null ? (
                <Button size="small" onClick={() => setMaking(diners + 2)} sx={{ flexShrink: 0 }}>
                  Making more?
                </Button>
              ) : null}
              {making !== null ? (
                <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center', flexShrink: 0 }}>
                  <IconButton size="small" aria-label="Make less" disabled={forecast <= diners} onClick={() => setMaking(Math.max(diners, forecast - 1))}><RemoveIcon fontSize="small" /></IconButton>
                  <Typography className="numeral" sx={{ minWidth: 20, textAlign: 'center', fontWeight: 600 }}>{forecast}</Typography>
                  <IconButton size="small" aria-label="Make more" onClick={() => setMaking(forecast + 1)}><AddIcon fontSize="small" /></IconButton>
                </Stack>
              ) : null}
            </Stack>
          ) : null}
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={busy}>Cancel</Button>
        <Button
          variant="contained"
          onClick={() => void save()}
          disabled={busy || foods.length === 0 || (household && diners === 0)}
        >
          {meal ? 'Save changes' : `Plan ${mealNoun}`}
        </Button>
      </DialogActions>
    </FormDialog>
  );
}
