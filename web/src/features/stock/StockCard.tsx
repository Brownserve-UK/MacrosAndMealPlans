import ChevronRightIcon from '@mui/icons-material/ChevronRight';
import { alpha } from '@mui/material/styles';
import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import { Fragment, type ReactNode } from 'react';
import type { Availability, StockItem } from '../../api/client';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { levelFor, type StockFigure } from './stockLevel';

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

export const LOCATION_ORDER: StockItem['storage_location'][] = ['frozen', 'chilled', 'ambient'];

export const LOCATION_LABEL: Record<StockItem['storage_location'], string> = {
  frozen: 'Freezer',
  chilled: 'Fridge',
  ambient: 'Cupboard',
};

export function primaryLocation(
  locations: StockItem['storage_location'][],
): StockItem['storage_location'] {
  const counts = new Map<StockItem['storage_location'], number>();
  for (const location of locations) counts.set(location, (counts.get(location) ?? 0) + 1);
  let best = LOCATION_ORDER[0] as StockItem['storage_location'];
  let bestCount = -1;
  for (const location of LOCATION_ORDER) {
    const count = counts.get(location) ?? 0;
    if (count > bestCount) {
      bestCount = count;
      best = location;
    }
  }
  return best;
}

export function withLocationGroups<T>(
  items: T[],
  locationOf: (item: T) => StockItem['storage_location'],
): { item: T; heading: { label: string; count: number } | null }[] {
  const counts = new Map<StockItem['storage_location'], number>();
  for (const item of items) {
    const location = locationOf(item);
    counts.set(location, (counts.get(location) ?? 0) + 1);
  }
  let previous: StockItem['storage_location'] | null = null;
  return items.map((item) => {
    const location = locationOf(item);
    const heading =
      location === previous ? null : { label: LOCATION_LABEL[location], count: counts.get(location) ?? 0 };
    previous = location;
    return { item, heading };
  });
}

export function StorageGroupHeading({ label, count }: { label: string; count: number }) {
  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'baseline',
        gap: 1.25,
        px: { xs: 2, sm: 2.5 },
        py: 1.25,
        fontSize: '0.7rem',
        fontWeight: 600,
        letterSpacing: '0.09em',
        textTransform: 'uppercase',
        color: 'text.disabled',
        backgroundColor: 'background.default',
        borderBottom: '1px solid',
        borderColor: 'divider',
      }}
    >
      {label}
      <Box component="span" sx={{ letterSpacing: 0, textTransform: 'none', fontWeight: 500 }}>
        {count}
      </Box>
    </Box>
  );
}

export function Gauge({ figure }: { figure: StockFigure }) {
  if (figure.short) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.6 }}>
        <Box
          aria-hidden
          sx={{
            flex: '1 1 auto',
            height: 12,
            borderRadius: 999,
            backgroundColor: (theme) => alpha(theme.palette.warning.main, 0.12),
          }}
        />
        <Box
          aria-hidden
          sx={{
            flex: 'none',
            width: 38,
            height: 12,
            borderRadius: 999,
            backgroundImage: (theme) =>
              `repeating-linear-gradient(115deg, ${theme.palette.warning.main} 0 4px, ${alpha(theme.palette.warning.main, 0.25)} 4px 8px)`,
          }}
        />
      </Box>
    );
  }
  return (
    <Box aria-hidden sx={{ height: 12, borderRadius: 999, overflow: 'hidden', backgroundColor: 'divider' }}>
      <Box sx={{ width: `${figure.fillPct}%`, height: '100%', backgroundColor: 'success.main' }} />
    </Box>
  );
}

export function StockRow({
  testId,
  name,
  subtitle,
  availability,
  figure,
}: {
  testId: string;
  name: string;
  subtitle: ReactNode;
  availability: Availability | null;
  figure?: string;
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

      {figure ? (
        <Typography
          variant="body2"
          className="numeral"
          sx={{ fontWeight: 600, flexShrink: 0, textAlign: 'right' }}
        >
          {figure}
        </Typography>
      ) : level.figure ? (
        <Stack spacing={0.75} sx={{ width: { xs: 'auto', sm: 176 }, flexShrink: 0 }}>
          <Box sx={{ display: { xs: 'none', sm: 'block' } }}>
            <Gauge figure={level.figure} />
          </Box>
          <Stack
            direction={{ xs: 'column', sm: 'row' }}
            className="numeral"
            sx={{
              justifyContent: 'space-between',
              alignItems: { xs: 'flex-end', sm: 'baseline' },
              gap: { xs: 0, sm: 1 },
            }}
          >
            <Typography variant="body2" sx={{ fontWeight: 600 }} noWrap>
              {level.figure.onHand}
            </Typography>
            <Typography
              variant="caption"
              sx={{
                color: level.figure.short ? 'warning.main' : 'text.secondary',
                fontWeight: level.figure.short ? 600 : 400,
                whiteSpace: 'nowrap',
              }}
            >
              {level.figure.free}
            </Typography>
          </Stack>
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

export type CookedFoodPlace = {
  location: StockItem['storage_location'];
  servings: number;
  useBy: string | null;
};

export type CookedFoodRow = {
  key: string;
  recipeId: string;
  name: string;
  places: CookedFoodPlace[];
  servings: number;
  useBy: string | null;
};

const PLACE_LABEL: Record<StockItem['storage_location'], string> = {
  ambient: 'Cupboard',
  chilled: 'Fridge',
  frozen: 'Freezer',
};

export const PLACE_ORDER: StockItem['storage_location'][] = ['chilled', 'frozen', 'ambient'];

function servingsFigure(servings: number): number {
  return Number.isInteger(servings) ? servings : Number(servings.toFixed(1));
}

function shortDate(date: string): string {
  return new Date(`${date}T00:00:00`).toLocaleDateString('en-GB', {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
  });
}

function cookedSubtitle(row: CookedFoodRow): ReactNode {
  return row.places.map((place, index) => (
    <Fragment key={place.location}>
      {index > 0 ? (
        <Box component="span" sx={{ mx: 1, color: 'text.disabled' }}>
          |
        </Box>
      ) : null}
      <Box component="span" sx={{ fontWeight: 600 }}>
        {PLACE_LABEL[place.location]}
      </Box>
      {`: ${servingsFigure(place.servings)}`}
      {place.useBy ? ` · ${shortDate(place.useBy)}` : ''}
    </Fragment>
  ));
}

export function PreparedPortionCard({ row }: { row: CookedFoodRow }) {
  const servings = servingsFigure(row.servings);

  return (
    <Link
      to="/stock/dishes/$recipeId"
      params={{ recipeId: row.recipeId }}
      style={{ textDecoration: 'none', color: 'inherit' }}
    >
      <StockRow
        testId={`stock-portion-${row.key}`}
        name={row.name}
        subtitle={cookedSubtitle(row)}
        availability={null}
        figure={servings === 1 ? '1 serving' : `${servings} servings`}
      />
    </Link>
  );
}

export function IngredientCard({
  group,
  productCount,
  kind = 'ingredient',
}: {
  group: StockGroup;
  productCount: number;
  kind?: 'ingredient' | 'prepared_meal';
}) {
  if (group.items.length === 0) return null;

  const row = (
    <StockRow
      testId={`stock-ingredient-${group.id}`}
      name={group.name}
      subtitle={productCount === 1 ? '1 product' : `${productCount} products`}
      availability={group.availability}
    />
  );

  return kind === 'ingredient' ? (
    <Link
      to="/stock/ingredients/$ingredientId"
      params={{ ingredientId: group.id }}
      style={{ textDecoration: 'none', color: 'inherit' }}
    >
      {row}
    </Link>
  ) : (
    <Link
      to="/stock/prepared-meals/$preparedMealId"
      params={{ preparedMealId: group.id }}
      style={{ textDecoration: 'none', color: 'inherit' }}
    >
      {row}
    </Link>
  );
}
