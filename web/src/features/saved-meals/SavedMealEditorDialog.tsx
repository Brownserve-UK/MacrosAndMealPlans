import DeleteIcon from '@mui/icons-material/DeleteOutlineOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import IconButton from '@mui/material/IconButton';
import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import type { Amount, Ingredient, MealTemplate, PreparedMeal, Product, RecipeSummary, Unit } from '../../api/client';
import { ApiError } from '../../api/client';
import { useCreateMealTemplate, useUpdateMealTemplate } from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';
import { displayUnit } from '../../components/UnitSelect';
import { FoodSearch, type FoodChoice } from '../meal-plan/FoodSearch';

type ComponentDraft = {
  key: string;
  itemKind: 'product' | 'recipe' | 'ingredient' | 'prepared_meal';
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

function initialComponents(template: MealTemplate | null): ComponentDraft[] {
  return (template?.components ?? []).flatMap((component): ComponentDraft[] => {
    switch (component.item_kind) {
      case 'product':
        return [{ key: crypto.randomUUID(), itemKind: 'product', itemId: component.product_id, name: component.item_name, amount: component.amount }];
      case 'recipe':
        return [{ key: crypto.randomUUID(), itemKind: 'recipe', itemId: component.recipe_id, name: component.item_name, amount: component.amount }];
      case 'ingredient':
        return [{ key: crypto.randomUUID(), itemKind: 'ingredient', itemId: component.ingredient_id, name: component.item_name, amount: component.amount }];
      case 'prepared_meal':
        return [{ key: crypto.randomUUID(), itemKind: 'prepared_meal', itemId: component.prepared_meal_id, name: component.item_name, amount: component.amount }];
      default:
        return [];
    }
  });
}

function suggestedName(components: ComponentDraft[]): string {
  return components.map((component) => component.name).join(', ');
}

export function SavedMealEditorDialog({
  open,
  onClose,
  template,
}: {
  open: boolean;
  onClose: () => void;
  template: MealTemplate | null;
}) {
  const create = useCreateMealTemplate();
  const update = useUpdateMealTemplate();
  const [components, setComponents] = useState<ComponentDraft[]>(() => initialComponents(template));
  const [name, setName] = useState(template?.name ?? '');
  const [nameTouched, setNameTouched] = useState(Boolean(template));
  const [error, setError] = useState<string | null>(null);
  const busy = create.isPending || update.isPending;

  const displayName = nameTouched ? name : suggestedName(components) || name;

  function addProduct(next: Product) {
    if (components.some((component) => component.itemKind === 'product' && component.itemId === next.id)) return;
    setComponents((current) => [...current, {
      key: crypto.randomUUID(),
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
    if (components.some((component) => component.itemKind === 'recipe' && component.itemId === next.id)) return;
    setComponents((current) => [...current, {
      key: crypto.randomUUID(),
      itemKind: 'recipe',
      itemId: next.id,
      name: next.name,
      amount: { kind: 'servings', value: 1 },
    }]);
  }

  function addIngredient(next: Ingredient) {
    if (components.some((component) => component.itemKind === 'ingredient' && component.itemId === next.id)) return;
    setComponents((current) => [...current, {
      key: crypto.randomUUID(),
      itemKind: 'ingredient',
      itemId: next.id,
      name: next.name,
      amount: { kind: 'measure', unit: next.default_unit, value: 100 },
    }]);
  }

  function addPreparedMeal(next: PreparedMeal) {
    if (components.some((component) => component.itemKind === 'prepared_meal' && component.itemId === next.id)) return;
    setComponents((current) => [...current, {
      key: crypto.randomUUID(),
      itemKind: 'prepared_meal',
      itemId: next.id,
      name: next.name,
      amount: { kind: 'measure', unit: next.default_unit, value: 100 },
    }]);
  }

  function onPick(choice: FoodChoice) {
    if (choice.kind === 'product') addProduct(choice.product);
    else if (choice.kind === 'recipe') addRecipe(choice.recipe);
    else if (choice.kind === 'ingredient') addIngredient(choice.ingredient);
    else if (choice.kind === 'prepared_meal') addPreparedMeal(choice.preparedMeal);
  }

  function setAmount(key: string, amount: Amount) {
    setComponents((current) => current.map((component) => component.key === key ? { ...component, amount } : component));
  }

  function removeComponent(key: string) {
    setComponents((current) => current.filter((component) => component.key !== key));
  }

  function handleClose() {
    if (busy) return;
    setError(null);
    onClose();
  }

  async function save() {
    if (components.length === 0) {
      setError('Add at least one food.');
      return;
    }
    if (!displayName.trim()) {
      setError('Give this saved meal a name.');
      return;
    }
    if (components.some((component) => !Number.isFinite(component.amount.value) || component.amount.value <= 0)) {
      setError('Every food needs an amount greater than zero.');
      return;
    }
    setError(null);

    const body = {
      name: displayName.trim(),
      components: components.map((component) => ({
        ...(component.itemKind === 'product'
          ? { item_kind: 'product' as const, product_id: component.itemId }
          : component.itemKind === 'recipe'
            ? { item_kind: 'recipe' as const, recipe_id: component.itemId }
            : component.itemKind === 'ingredient'
              ? { item_kind: 'ingredient' as const, ingredient_id: component.itemId }
              : { item_kind: 'prepared_meal' as const, prepared_meal_id: component.itemId }),
        amount: component.amount,
      })),
    };

    try {
      if (template) {
        await update.mutateAsync({ id: template.id, revision: template.revision, body });
      } else {
        await create.mutateAsync(body);
      }
      onClose();
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not save this saved meal.');
    }
  }

  return (
    <FormDialog open={open} onClose={handleClose} fullWidth maxWidth="sm">
      <DialogTitle>{template ? 'Edit saved meal' : 'Save for reuse'}</DialogTitle>
      <DialogContent dividers>
        <Stack spacing={2.5}>
          {error ? <Alert severity="error">{error}</Alert> : null}

          <TextField
            label="Name"
            value={displayName}
            onChange={(event) => {
              setNameTouched(true);
              setName(event.target.value);
            }}
            fullWidth
          />

          <Box>
            <Typography variant="h3" sx={{ mb: 1 }}>Foods</Typography>
            <Stack spacing={1.5}>
              <FoodSearch
                onPick={onPick}
                excludeProductIds={components.filter((component) => component.itemKind === 'product').map((component) => component.itemId)}
                excludeRecipeIds={components.filter((component) => component.itemKind === 'recipe').map((component) => component.itemId)}
                excludeIngredientIds={components.filter((component) => component.itemKind === 'ingredient').map((component) => component.itemId)}
                excludePreparedMealIds={components.filter((component) => component.itemKind === 'prepared_meal').map((component) => component.itemId)}
                hideSavedMeals
                hideDishes
              />
              {components.map((component) => (
                <Box key={component.key} sx={{ border: '1px solid', borderColor: 'divider', borderRadius: 2, p: 1.5 }}>
                  <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
                    <Box sx={{ flex: 1, minWidth: 0 }}>
                      <Typography>{component.name}</Typography>
                    </Box>
                    <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                      <TextField
                        label={component.itemKind === 'recipe' ? 'Servings' : 'Amount'}
                        type="number"
                        value={amountValue(component.amount)}
                        onChange={(event) => setAmount(component.key, withAmountValue(component.amount, event.target.value))}
                        slotProps={{ htmlInput: { min: 0, step: 'any' } }}
                        sx={{ width: 110 }}
                      />
                      {component.amount.kind === 'measure' ? (
                        <TextField
                          select
                          label="Unit"
                          value={component.amount.unit}
                          onChange={(event) => setAmount(component.key, { kind: 'measure', value: component.amount.value, unit: event.target.value as Unit })}
                          sx={{ width: 100 }}
                        >
                          {UNITS.map((unit) => <MenuItem key={unit} value={unit}>{displayUnit(unit)}</MenuItem>)}
                        </TextField>
                      ) : null}
                    </Stack>
                    <IconButton aria-label={`Remove ${component.name}`} onClick={() => removeComponent(component.key)}>
                      <DeleteIcon />
                    </IconButton>
                  </Stack>
                </Box>
              ))}
            </Stack>
          </Box>
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={handleClose} disabled={busy}>Cancel</Button>
        <Button variant="contained" onClick={() => void save()} disabled={busy || components.length === 0}>
          {busy ? 'Saving…' : template ? 'Save changes' : 'Save'}
        </Button>
      </DialogActions>
    </FormDialog>
  );
}
