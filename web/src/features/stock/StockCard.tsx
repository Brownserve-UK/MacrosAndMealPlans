import ChevronRightIcon from '@mui/icons-material/ChevronRight';
import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import type { Availability, StockItem } from '../../api/client';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { levelFor } from './stockLevel';

export type StockGroup = {
  id: string;
  name: string;
  items: StockItem[];
  availability: Availability | null;
};

function firstDate(items: StockItem[]): string | null {
  const dates = items
    .map((item) => item.usability_deadline?.date ?? item.source_date?.date ?? null)
    .filter((value): value is string => value !== null)
    .sort();
  return dates[0] ?? null;
}

export function groupSortDate(group: StockGroup): string | null {
  return firstDate(group.items);
}

export function locationSubtitle(items: StockItem[]): string {
  const locations = [...new Set(items.map((item) => item.storage_location))];
  const date = firstDate(items);
  const location = locations.join(', ');
  return date ? `${location} · nearest date ${new Date(`${date}T00:00:00`).toLocaleDateString('en-GB')}` : location;
}

export function StockRow({
  testId,
  name,
  subtitle,
  availability,
}: {
  testId: string;
  name: string;
  subtitle: string;
  availability: Availability | null;
}) {
  const level = levelFor(availability);

  return (
    <Box
      data-testid={testId}
      sx={{
        display: 'flex',
        alignItems: 'center',
        gap: 1.75,
        px: { xs: 2, sm: 2.5 },
        py: 1.5,
        transition: 'background-color 120ms ease',
        '&:hover': { backgroundColor: 'action.hover' },
        '&:hover .chevron': { opacity: 1 },
      }}
    >
      <InitialsAvatar name={name} size={40} />

      <Stack sx={{ minWidth: 0, flexGrow: 1 }} spacing={0.25}>
        <Typography variant="subtitle1" sx={{ fontWeight: 600 }} noWrap>
          {name}
        </Typography>
        <Typography variant="caption" color="text.secondary" noWrap>
          {subtitle}
        </Typography>
      </Stack>

      {level.figure ? (
        <Stack spacing={0.5} sx={{ width: { xs: 128, sm: 176 }, flexShrink: 0 }}>
          <Box
            aria-hidden
            sx={{
              height: 8,
              borderRadius: 999,
              overflow: 'hidden',
              backgroundColor: 'text.primary',
            }}
          >
            <Box
              sx={{
                width: level.solidRed ? '100%' : `${level.fillPct}%`,
                height: '100%',
                backgroundColor: level.colour,
              }}
            />
          </Box>
          <Typography
            variant="caption"
            className="numeral"
            sx={{ fontWeight: 600, color: level.colour, textAlign: 'right' }}
          >
            {level.figure.needed} / {level.figure.available}
          </Typography>
        </Stack>
      ) : (
        <Typography
          variant="body2"
          sx={{ color: level.colour, flexShrink: 0, textAlign: 'right' }}
        >
          {level.statusWord}
        </Typography>
      )}

      <ChevronRightIcon
        className="chevron"
        fontSize="small"
        sx={{
          color: 'text.disabled',
          opacity: 0,
          transition: 'opacity 120ms ease',
          flexShrink: 0,
        }}
      />
    </Box>
  );
}

export function StockCard({ group }: { group: StockGroup }) {
  if (group.items.length === 0) return null;

  return (
    <Link
      to="/stock/products/$productId"
      params={{ productId: group.id }}
      style={{ textDecoration: 'none', color: 'inherit' }}
    >
      <StockRow
        testId={`stock-card-${group.id}`}
        name={group.name}
        subtitle={locationSubtitle(group.items)}
        availability={group.availability}
      />
    </Link>
  );
}

export function IngredientCard({ group, productCount }: { group: StockGroup; productCount: number }) {
  if (group.items.length === 0) return null;

  return (
    <Link
      to="/stock/ingredients/$ingredientId"
      params={{ ingredientId: group.id }}
      style={{ textDecoration: 'none', color: 'inherit' }}
    >
      <StockRow
        testId={`stock-ingredient-${group.id}`}
        name={group.name}
        subtitle={productCount === 1 ? '1 product' : `${productCount} products`}
        availability={group.availability}
      />
    </Link>
  );
}
