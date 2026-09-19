import Chip from '@mui/material/Chip';
import { alpha } from '@mui/material/styles';
import type { Tone } from './IconTile';

export type Kind = 'recipe' | 'food' | 'product' | 'saved_meal' | 'dish' | 'fridge' | 'freezer';

const KINDS: Record<Kind, { label: string; tone: Tone }> = {
  recipe: { label: 'Recipe', tone: 'primary' },
  food: { label: 'Food', tone: 'neutral' },
  product: { label: 'Product', tone: 'neutral' },
  saved_meal: { label: 'Saved meal', tone: 'secondary' },
  dish: { label: 'Cooked food', tone: 'secondary' },
  fridge: { label: 'Fridge', tone: 'neutral' },
  freezer: { label: 'Freezer', tone: 'neutral' },
};

export function KindChip({ kind }: { kind: Kind }) {
  const { label, tone } = KINDS[kind];
  return (
    <Chip
      size="small"
      label={label}
      sx={(theme) =>
        tone === 'neutral'
          ? {
              backgroundColor: 'transparent',
              border: '1px solid',
              borderColor: 'divider',
              color: 'text.secondary',
            }
          : {
              backgroundColor: theme.vars
                ? `rgba(${theme.vars.palette[tone].mainChannel} / 0.12)`
                : alpha(theme.palette[tone].main, 0.12),
              color: `${tone}.main`,
            }
      }
    />
  );
}
