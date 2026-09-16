import AddIcon from '@mui/icons-material/AddOutlined';
import DeleteIcon from '@mui/icons-material/DeleteOutlineOutlined';
import RemoveIcon from '@mui/icons-material/RemoveOutlined';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import type { Dispatch, SetStateAction } from 'react';
import type { Amount, Ingredient, PlannerMeal, PreparedMeal, Product, RecipeSummary, Unit } from '../../api/client';
import { useRecipeNutrition } from '../../api/queries';
import { displayUnit } from '../../components/UnitSelect';
import { FoodSearch, type Dish, type FoodChoice } from './FoodSearch';
import type { PlannableDish } from './UseItUp';

export type FoodDraft = {
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

export function initialFoods(meal: PlannerMeal | null, startWith?: PlannableDish | null): FoodDraft[] {
  const existing = (meal?.foods ?? []).map((food) => ({
    componentId: food.id,
    itemKind: food.item_kind,
    itemId: itemIdOf(food),
    name: food.item_name,
    amount: food.amount,
  }));
  if (!startWith || existing.some((food) => food.itemId === startWith.recipeId)) return existing;
  return [...existing, {
    componentId: crypto.randomUUID(),
    itemKind: 'dish',
    itemId: startWith.recipeId,
    name: startWith.name,
    amount: { kind: 'servings', value: 1 },
  }];
}

export function initialMaking(meal: PlannerMeal | null, dinerCount: number): number | null {
  const recipe = (meal?.foods ?? []).find((food) => food.amount.kind === 'servings');
  if (!recipe) return null;
  const planned = Math.round(recipe.amount.value);
  return planned > dinerCount ? planned : null;
}

export function forecastServings(making: number | null, dinerCount: number) {
  return Math.max(making ?? dinerCount, dinerCount);
}

export function amountForMeal(food: FoodDraft, forecast: number): Amount {
  return food.itemKind === 'recipe' ? { kind: 'servings', value: forecast } : food.amount;
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

export function MealFoodFields({
  foods,
  setFoods,
  dinerCount,
  making,
  setMaking,
  mealNoun,
}: {
  foods: FoodDraft[];
  setFoods: Dispatch<SetStateAction<FoodDraft[]>>;
  dinerCount: number;
  making: number | null;
  setMaking: Dispatch<SetStateAction<number | null>>;
  mealNoun: string;
}) {
  const forecast = forecastServings(making, dinerCount);
  const hasRecipe = foods.some((food) => food.itemKind === 'recipe');
  const spare = forecast - dinerCount;

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
          current.some((food) => food.itemKind === itemKind && food.itemId === itemId)
          || additions.some((food) => food.itemKind === itemKind && food.itemId === itemId);
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

  return (
    <Stack spacing={2.5}>
      <Stack spacing={1.5}>
        <FoodSearch
          onPick={addFood}
          excludeProductIds={foods.filter((food) => food.itemKind === 'product').map((food) => food.itemId)}
          excludeRecipeIds={foods.filter((food) => food.itemKind === 'recipe').map((food) => food.itemId)}
          excludeDishIds={foods.filter((food) => food.itemKind === 'dish').map((food) => food.itemId)}
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
              ) : (
                <Typography className="numeral" variant="body2">
                  {forecast === 1 ? '1 serving' : `${forecast} servings`}
                </Typography>
              )}
              <IconButton aria-label={`Remove ${food.name}`} onClick={() => setFoods((current) => current.filter((candidate) => candidate.componentId !== food.componentId))}>
                <DeleteIcon />
              </IconButton>
            </Stack>
          </Box>
        ))}
      </Stack>

      {hasRecipe ? (
        <Stack
          direction="row"
          spacing={2}
          sx={{ alignItems: 'center', bgcolor: 'action.hover', borderRadius: 2.5, px: 1.75, py: 1.5 }}
        >
          <Box sx={{ flex: 1, minWidth: 0 }}>
            <Typography variant="subtitle2" className="numeral" color={dinerCount > 0 ? 'text.primary' : 'text.disabled'}>
              {dinerCount === 0
                ? 'Nobody yet'
                : forecast === 1 ? 'About 1 serving' : `About ${forecast} servings`}
            </Typography>
            <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
              {dinerCount === 0
                ? `Pick who this ${mealNoun} is for.`
                : spare > 0
                  ? `${spare} spare for the freezer`
                  : 'For the shopping list'}
            </Typography>
          </Box>
          {making === null ? (
            <Button size="small" onClick={() => setMaking(dinerCount + 2)} sx={{ flexShrink: 0 }} disabled={dinerCount === 0}>
              Making more?
            </Button>
          ) : (
            <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center', flexShrink: 0 }}>
              <IconButton size="small" aria-label="Make less" disabled={forecast <= dinerCount} onClick={() => setMaking(Math.max(dinerCount, forecast - 1))}>
                <RemoveIcon fontSize="small" />
              </IconButton>
              <Typography className="numeral" sx={{ minWidth: 20, textAlign: 'center', fontWeight: 600 }}>{forecast}</Typography>
              <IconButton size="small" aria-label="Make more" onClick={() => setMaking(forecast + 1)}>
                <AddIcon fontSize="small" />
              </IconButton>
            </Stack>
          )}
        </Stack>
      ) : null}
    </Stack>
  );
}
