import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { CookingItem } from '../../../api/client';
import { IconTile } from '../../../components/IconTile';
import { KindChip } from '../../../components/KindChip';
import { displayUnit } from '../../../components/UnitSelect';
import { initialsOf } from './plannerWeek';
import type { PlannerMember } from './types';

function trimmed(value: string): string {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? String(parsed) : value;
}

function formatCookingAmount(amount: CookingItem['amount']): { value: string; unit: string } {
  const value = trimmed(amount.value);
  if (amount.kind === 'measure') return { value, unit: displayUnit(amount.unit ?? '') };
  if (amount.kind === 'servings') return { value, unit: value === '1' ? 'serving' : 'servings' };
  return { value, unit: value === '1' ? 'pack' : 'packs' };
}

function tileFor(kind: CookingItem['kind']) {
  switch (kind) {
    case 'recipe':
      return { concept: 'recipe' as const, tone: 'primary' as const, chip: 'recipe' as const };
    case 'cooked_food':
      return { concept: 'dish' as const, tone: 'secondary' as const, chip: 'dish' as const };
    default:
      return { concept: 'food' as const, tone: 'neutral' as const, chip: 'food' as const };
  }
}

export function CookingItemRow({
  item,
  members,
  showFor,
}: {
  item: CookingItem;
  members: PlannerMember[];
  showFor: boolean;
}) {
  const { concept, tone, chip } = tileFor(item.kind);
  const { value, unit } = formatCookingAmount(item.amount);
  const diners = item.member_ids
    .map((id: string) => members.find((member) => member.id === id))
    .filter((member): member is PlannerMember => member !== undefined);
  const caption = item.group_ids.length > 1 ? `${item.group_ids.length} meals` : null;

  return (
    <Box
      sx={{
        display: 'grid',
        gridTemplateColumns: '26px 1fr auto auto',
        gap: 1.5,
        alignItems: 'center',
        py: 1.375,
        borderTop: '1px solid',
        borderColor: 'divider',
      }}
    >
      <IconTile concept={concept} tone={tone} size={26} />
      <Box sx={{ minWidth: 0 }}>
        <Stack direction="row" spacing={1} sx={{ alignItems: 'center', flexWrap: 'wrap' }}>
          <Typography sx={{ fontWeight: 500 }}>{item.name}</Typography>
          <KindChip kind={chip} />
        </Stack>
        {caption ? (
          <Typography variant="caption" color="text.secondary" className="numeral" sx={{ display: 'block' }}>
            {caption}
          </Typography>
        ) : null}
      </Box>
      <Typography variant="body2" className="numeral" sx={{ whiteSpace: 'nowrap', textAlign: 'right' }}>
        <Box component="b" sx={{ color: 'text.primary', fontWeight: 600 }}>
          {value}
        </Box>{' '}
        {unit}
        {item.extra_servings ? (
          <Typography component="span" variant="body2" color="text.secondary">
            {' '}
            (+{item.extra_servings})
          </Typography>
        ) : null}
      </Typography>
      <Box sx={{ justifySelf: 'end', minWidth: 34, display: 'flex', flexDirection: 'row-reverse' }}>
        {showFor
          ? diners.map((member, index) => (
              <Box
                key={member.id}
                aria-hidden
                sx={{
                  width: 21,
                  height: 21,
                  borderRadius: '50%',
                  display: 'grid',
                  placeItems: 'center',
                  fontSize: '0.6rem',
                  fontWeight: 600,
                  backgroundColor: 'divider',
                  color: 'text.secondary',
                  border: '2px solid',
                  borderColor: 'background.paper',
                  marginRight: index > 0 ? '-7px' : 0,
                }}
              >
                {initialsOf(member)}
              </Box>
            ))
          : null}
      </Box>
    </Box>
  );
}
