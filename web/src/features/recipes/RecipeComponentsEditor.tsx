import DeleteIcon from '@mui/icons-material/DeleteOutlineOutlined';
import AddIcon from '@mui/icons-material/AddOutlined';
import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import IconButton from '@mui/material/IconButton';
import Stack from '@mui/material/Stack';
import type { Amount, Product, RecipeComponent } from '../../api/client';
import { useProduct } from '../../api/queries';
import { Loading } from '../../components/States';
import {
  AmountFields,
  amountDraftFrom,
  draftToAmount,
  validateAmountDraft,
  type AmountDraft,
} from '../meal-plan/AmountFields';
import { ProductPicker } from '../meal-plan/ProductPicker';

export type ComponentLine = {
  key: string;
  id?: string;
  productId: string | null;
  product: Product | null;
  amount: AmountDraft;
};

type ComponentRequest = {
  id?: string;
  product_id: string;
  amount: Amount;
};

export function newLine(): ComponentLine {
  return {
    key: crypto.randomUUID(),
    productId: null,
    product: null,
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
    productId: component.product_id,
    product: null,
    amount: amountToDraft(component.amount),
  }));
}

export function linesAreValid(lines: ComponentLine[]): boolean {
  if (lines.length === 0) return false;
  return lines.every(
    (line) =>
      line.productId != null && Object.keys(validateAmountDraft(line.amount)).length === 0,
  );
}

export function linesToComponents(lines: ComponentLine[]): ComponentRequest[] | null {
  const out: ComponentRequest[] = [];
  for (const line of lines) {
    const amount = draftToAmount(line.amount);
    if (!line.productId || !amount) return null;
    out.push({ ...(line.id ? { id: line.id } : {}), product_id: line.productId, amount });
  }
  return out;
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
  const loaded = useProduct(line.productId ?? '', { enabled: Boolean(line.productId) && !line.product });
  const product = line.product ?? loaded.data ?? null;

  return (
    <Stack direction="row" spacing={1.5} sx={{ alignItems: 'flex-start' }}>
      <Stack spacing={2} sx={{ flexGrow: 1 }}>
        <ProductPickerLoader
          value={product}
          loading={loaded.isLoading}
          excludeIds={excludeIds}
          onChange={(next) =>
            onChange({
              ...line,
              product: next,
              productId: next?.id ?? null,
              amount: amountDraftFrom(next),
            })
          }
        />
        {product ? (
          <AmountFields
            product={product}
            draft={line.amount}
            errors={{}}
            onChange={(amount) => onChange({ ...line, amount })}
          />
        ) : null}
      </Stack>
      <IconButton aria-label="Remove product" onClick={onRemove} sx={{ mt: 1 }}>
        <DeleteIcon fontSize="small" />
      </IconButton>
    </Stack>
  );
}

function ProductPickerLoader({
  value,
  loading,
  excludeIds,
  onChange,
}: {
  value: Product | null;
  loading: boolean;
  excludeIds: string[];
  onChange: (next: Product | null) => void;
}) {
  if (loading && !value) return <Loading label="Loading product" />;
  return <ProductPicker value={value} onChange={onChange} excludeIds={excludeIds} />;
}

export function RecipeComponentsEditor({
  lines,
  onChange,
}: {
  lines: ComponentLine[];
  onChange: (next: ComponentLine[]) => void;
}) {
  const chosenIds = lines.map((line) => line.productId).filter((id): id is string => Boolean(id));

  return (
    <Stack spacing={2.5} divider={<Divider />}>
      {lines.map((line) => (
        <ComponentLineEditor
          key={line.key}
          line={line}
          excludeIds={chosenIds.filter((id) => id !== line.productId)}
          onChange={(next) => onChange(lines.map((l) => (l.key === line.key ? next : l)))}
          onRemove={() => onChange(lines.filter((l) => l.key !== line.key))}
        />
      ))}
      <Button
        startIcon={<AddIcon />}
        onClick={() => onChange([...lines, newLine()])}
        sx={{ alignSelf: 'flex-start' }}
      >
        Add product
      </Button>
    </Stack>
  );
}
