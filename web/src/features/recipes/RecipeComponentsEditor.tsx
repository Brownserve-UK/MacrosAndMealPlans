import DeleteIcon from '@mui/icons-material/DeleteOutlineOutlined';
import AddIcon from '@mui/icons-material/AddOutlined';
import Autocomplete, { createFilterOptions } from '@mui/material/Autocomplete';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Divider from '@mui/material/Divider';
import FormControlLabel from '@mui/material/FormControlLabel';
import IconButton from '@mui/material/IconButton';
import Stack from '@mui/material/Stack';
import Switch from '@mui/material/Switch';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import type { Amount, Ingredient, Product, RecipeComponent } from '../../api/client';
import {
  useCreateIngredient,
  useIngredient,
  useIngredientProducts,
  useIngredients,
  useProduct,
} from '../../api/queries';
import { Loading } from '../../components/States';
import { useDebounced } from '../../hooks/useDebounced';
import {
  AmountFields,
  amountDraftFrom,
  draftToAmount,
  validateAmountDraft,
  type AmountDraft,
} from '../meal-plan/AmountFields';

type Requirement =
  | {
      kind: 'ingredient';
      ingredientId: string | null;
      ingredient: Ingredient | null;
      pinned: boolean;
      productId: string | null;
      product: Product | null;
    }
  | { kind: 'unresolved'; text: string };

export type ComponentLine = {
  key: string;
  id?: string;
  requirement: Requirement;
  amount: AmountDraft;
};

type RequirementRequest =
  | { kind: 'ingredient'; ingredient_id: string }
  | { kind: 'product'; product_id: string }
  | { kind: 'unresolved'; text: string };

type ComponentRequest = {
  id?: string;
  requirement: RequirementRequest;
  amount: Amount;
};

export function newLine(): ComponentLine {
  return {
    key: crypto.randomUUID(),
    requirement: {
      kind: 'ingredient',
      ingredientId: null,
      ingredient: null,
      pinned: false,
      productId: null,
      product: null,
    },
    amount: amountDraftFrom(null),
  };
}

function amountToDraft(amount: Amount): AmountDraft {
  return amount.kind === 'measure'
    ? { kind: 'measure', value: String(amount.value), unit: amount.unit }
    : { kind: amount.kind, value: String(amount.value), unit: 'g' };
}

export function linesFromComponents(components: RecipeComponent[]): ComponentLine[] {
  return components.map((component) => ({
    key: component.id,
    id: component.id,
    requirement: requirementFrom(component),
    amount: amountToDraft(component.amount),
  }));
}

function requirementFrom(component: RecipeComponent): Requirement {
  const requirement = component.requirement;
  if (requirement.kind === 'unresolved') {
    return { kind: 'unresolved', text: requirement.text };
  }
  if (requirement.kind === 'product') {
    return {
      kind: 'ingredient',
      ingredientId: null,
      ingredient: null,
      pinned: true,
      productId: requirement.product_id,
      product: null,
    };
  }
  return {
    kind: 'ingredient',
    ingredientId: requirement.ingredient_id,
    ingredient: null,
    pinned: false,
    productId: null,
    product: null,
  };
}

function lineIsValid(line: ComponentLine): boolean {
  if (Object.keys(validateAmountDraft(line.amount)).length > 0) return false;
  if (line.requirement.kind === 'unresolved') return line.requirement.text.trim().length > 0;
  if (line.requirement.pinned) return line.requirement.productId != null;
  return line.requirement.ingredientId != null;
}

export function linesAreValid(lines: ComponentLine[]): boolean {
  if (lines.length === 0) return false;
  return lines.every(lineIsValid);
}

export function linesToComponents(lines: ComponentLine[]): ComponentRequest[] | null {
  const out: ComponentRequest[] = [];
  for (const line of lines) {
    const amount = draftToAmount(line.amount);
    if (!amount) return null;
    let requirement: RequirementRequest;
    if (line.requirement.kind === 'unresolved') {
      const text = line.requirement.text.trim();
      if (!text) return null;
      requirement = { kind: 'unresolved', text };
    } else if (line.requirement.pinned) {
      if (!line.requirement.productId) return null;
      requirement = { kind: 'product', product_id: line.requirement.productId };
    } else {
      if (!line.requirement.ingredientId) return null;
      requirement = { kind: 'ingredient', ingredient_id: line.requirement.ingredientId };
    }
    out.push({ ...(line.id ? { id: line.id } : {}), requirement, amount });
  }
  return out;
}

function chosenIngredientIds(lines: ComponentLine[]): string[] {
  return lines
    .map((line) => (line.requirement.kind === 'ingredient' ? line.requirement.ingredientId : null))
    .filter((id): id is string => Boolean(id));
}

function ComponentLineEditor({
  line,
  onChange,
  onRemove,
  excludeIds,
}: {
  line: ComponentLine;
  onChange: (next: ComponentLine) => void;
  onRemove: () => void;
  excludeIds: string[];
}) {
  if (line.requirement.kind === 'unresolved') {
    return (
      <Stack direction="row" spacing={1.5} sx={{ alignItems: 'flex-start' }}>
        <Stack spacing={1.5} sx={{ flexGrow: 1 }}>
          <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
            <Typography>{line.requirement.text}</Typography>
            <Chip size="small" color="warning" variant="outlined" label="Not matched" />
          </Stack>
          <Typography variant="caption" color="text.secondary">
            Match this from the recipe page.
          </Typography>
          <AmountFields
            product={null}
            draft={line.amount}
            errors={{}}
            onChange={(amount) => onChange({ ...line, amount })}
            allowWithoutProduct
          />
        </Stack>
        <IconButton aria-label="Remove line" onClick={onRemove} sx={{ mt: 1 }}>
          <DeleteIcon fontSize="small" />
        </IconButton>
      </Stack>
    );
  }

  const requirement = line.requirement;

  return (
    <Stack direction="row" spacing={1.5} sx={{ alignItems: 'flex-start' }}>
      <Stack spacing={2} sx={{ flexGrow: 1 }}>
        <IngredientField
          ingredientId={requirement.ingredientId}
          ingredient={requirement.ingredient}
          excludeIds={excludeIds}
          onPick={(next) =>
            onChange({
              ...line,
              requirement: {
                ...requirement,
                ingredientId: next?.id ?? null,
                ingredient: next,
                // A different ingredient invalidates any pinned product.
                productId: next?.id === requirement.ingredientId ? requirement.productId : null,
                product: next?.id === requirement.ingredientId ? requirement.product : null,
              },
              amount:
                next && !requirement.pinned
                  ? { kind: 'measure', value: line.amount.value, unit: next.default_unit }
                  : line.amount,
            })
          }
        />

        {requirement.ingredientId ? (
          <FormControlLabel
            control={
              <Switch
                size="small"
                checked={requirement.pinned}
                onChange={(event) =>
                  onChange({
                    ...line,
                    requirement: {
                      ...requirement,
                      pinned: event.target.checked,
                      productId: event.target.checked ? requirement.productId : null,
                      product: event.target.checked ? requirement.product : null,
                    },
                  })
                }
              />
            }
            label={<Typography variant="body2">Use a specific product</Typography>}
          />
        ) : null}

        {requirement.pinned && requirement.ingredientId ? (
          <PinnedProductField
            ingredientId={requirement.ingredientId}
            productId={requirement.productId}
            product={requirement.product}
            onPick={(next) =>
              onChange({
                ...line,
                requirement: { ...requirement, productId: next?.id ?? null, product: next },
                amount: next ? amountDraftFrom(next) : line.amount,
              })
            }
          />
        ) : null}

        <AmountFields
          product={requirement.pinned ? requirement.product : null}
          draft={line.amount}
          errors={{}}
          onChange={(amount) => onChange({ ...line, amount })}
          allowWithoutProduct
        />
      </Stack>
      <IconButton aria-label="Remove line" onClick={onRemove} sx={{ mt: 1 }}>
        <DeleteIcon fontSize="small" />
      </IconButton>
    </Stack>
  );
}

type IngredientOption = Ingredient | { create: true; inputValue: string; name: string };

const filterIngredients = createFilterOptions<IngredientOption>();

function IngredientField({
  ingredientId,
  ingredient,
  excludeIds,
  onPick,
}: {
  ingredientId: string | null;
  ingredient: Ingredient | null;
  excludeIds: string[];
  onPick: (next: Ingredient | null) => void;
}) {
  const [input, setInput] = useState('');
  const debounced = useDebounced(input, 300);
  const list = useIngredients({ q: debounced || undefined, per_page: 20 });
  const createIngredient = useCreateIngredient();
  const loaded = useIngredient(ingredientId ?? '', {
    enabled: Boolean(ingredientId) && !ingredient,
  });
  const current = ingredient ?? loaded.data ?? null;

  if (loaded.isLoading && !current) return <Loading label="Loading ingredient" />;

  const excluded = new Set(excludeIds);
  const options: IngredientOption[] = (list.data?.items ?? []).filter(
    (item) => item.id === current?.id || !excluded.has(item.id),
  );

  async function choose(option: IngredientOption | null) {
    if (option && 'create' in option) {
      const created = await createIngredient.mutateAsync({
        name: option.inputValue.trim(),
        default_unit: 'g',
      });
      onPick(created);
      return;
    }
    onPick(option);
  }

  return (
    <Autocomplete<IngredientOption>
      value={current}
      onChange={(_, next) => void choose(next)}
      inputValue={input}
      onInputChange={(_, next) => setInput(next)}
      options={options}
      filterOptions={(opts, params) => {
        const filtered = filterIngredients(opts, params);
        const typed = params.inputValue.trim();
        const exists = opts.some(
          (opt) => 'name' in opt && opt.name.toLowerCase() === typed.toLowerCase(),
        );
        if (typed && !exists) {
          filtered.push({ create: true, inputValue: typed, name: `Add "${typed}"` });
        }
        return filtered;
      }}
      getOptionLabel={(option) =>
        'create' in option ? option.inputValue : option.name
      }
      renderOption={(props, option) => (
        <li {...props} key={'create' in option ? '__create' : option.id}>
          {option.name}
        </li>
      )}
      isOptionEqualToValue={(a, b) =>
        'create' in a || 'create' in b ? false : a.id === b.id
      }
      loading={list.isLoading || createIngredient.isPending}
      renderInput={(params) => (
        <TextField
          {...params}
          label="Ingredient"
          placeholder="Search"
          helperText="The generic food this recipe needs."
        />
      )}
    />
  );
}

function PinnedProductField({
  ingredientId,
  productId,
  product,
  onPick,
}: {
  ingredientId: string;
  productId: string | null;
  product: Product | null;
  onPick: (next: Product | null) => void;
}) {
  const listed = useIngredientProducts(ingredientId);
  const loaded = useProduct(productId ?? '', {
    enabled: Boolean(productId) && !product,
  });
  const current = product ?? loaded.data ?? null;
  const options = listed.data?.items ?? [];

  if (listed.isLoading) return <Loading label="Loading products" />;

  if (options.length === 0) {
    return (
      <Typography variant="body2" color="text.secondary">
        No products are mapped to this ingredient yet.
      </Typography>
    );
  }

  return (
    <Autocomplete<Product>
      value={current}
      onChange={(_, next) => onPick(next)}
      options={options}
      getOptionLabel={(option) => option.name}
      isOptionEqualToValue={(a, b) => a.id === b.id}
      renderInput={(params) => (
        <TextField
          {...params}
          label="Product"
          placeholder="Choose"
          helperText="Only products that fulfil this ingredient."
        />
      )}
    />
  );
}

export function RecipeComponentsEditor({
  lines,
  onChange,
}: {
  lines: ComponentLine[];
  onChange: (next: ComponentLine[]) => void;
}) {
  const chosen = chosenIngredientIds(lines);

  return (
    <Stack spacing={2.5} divider={<Divider />}>
      {lines.map((line) => {
        const own =
          line.requirement.kind === 'ingredient' ? line.requirement.ingredientId : null;
        return (
          <ComponentLineEditor
            key={line.key}
            line={line}
            excludeIds={chosen.filter((id) => id !== own)}
            onChange={(next) => onChange(lines.map((l) => (l.key === line.key ? next : l)))}
            onRemove={() => onChange(lines.filter((l) => l.key !== line.key))}
          />
        );
      })}
      <Button
        startIcon={<AddIcon />}
        onClick={() => onChange([...lines, newLine()])}
        sx={{ alignSelf: 'flex-start' }}
      >
        Add ingredient
      </Button>
    </Stack>
  );
}
