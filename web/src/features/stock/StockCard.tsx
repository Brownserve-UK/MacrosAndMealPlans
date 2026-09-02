import ChevronRightIcon from '@mui/icons-material/ChevronRight';
import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link } from '@tanstack/react-router';
import Chip from '@mui/material/Chip';
import type { Availability, DemandGap, StockItem } from '../../api/client';
import { InitialsAvatar } from '../../components/InitialsAvatar';
import { gapLabel } from './demandGap';
import { levelFor } from './stockLevel';

export type StockGroup = {
  productId: string;
  productName: string;
  items: StockItem[];
  availability: Availability | null;
  gaps: DemandGap[];
};

export function groupSortDate(group: StockGroup): string | null {
  const dates = group.items
    .map((item) => item.usability_deadline?.date ?? item.source_date?.date ?? null)
    .filter((value): value is string => value !== null)
    .sort();
  return dates[0] ?? null;
}

function subtitle(items: StockItem[]): string {
  if (items.length > 1) return `${items.length} lots`;
  return items[0]?.storage_location ?? '';
}

export function StockCard({ group }: { group: StockGroup }) {
  const level = levelFor(group.availability);
  const gap = gapLabel(group.gaps);
  const target = group.items[0];
  if (!target) return null;

  return (
    <Link
      to="/stock/$id"
      params={{ id: target.id }}
      style={{ textDecoration: 'none', color: 'inherit' }}
    >
      <Box
        data-testid={`stock-card-${group.productId}`}
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
        <InitialsAvatar name={group.productName} size={40} />

        <Stack sx={{ minWidth: 0, flexGrow: 1 }} spacing={0.25}>
          <Typography variant="subtitle1" sx={{ fontWeight: 600 }} noWrap>
            {group.productName}
          </Typography>
          <Typography variant="caption" color="text.secondary" noWrap>
            {subtitle(group.items)}
          </Typography>
          {gap && (
            <Chip
              size="small"
              color="warning"
              variant="outlined"
              label={gap}
              sx={{ alignSelf: 'flex-start', mt: 0.25 }}
            />
          )}
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
    </Link>
  );
}
