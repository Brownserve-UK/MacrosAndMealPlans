import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { CookingRow } from './CookingRow';
import { groupDiners } from './plannerWeek';
import type { GroupView, OccasionView, PlannerMember } from './types';

function isServed(occasion: OccasionView, group: GroupView, members: PlannerMember[]): boolean {
  return groupDiners(occasion, group, members).length > 0 || group.guest_count > 0;
}

export function CookingList({
  occasion,
  members,
  busy,
  onCooking,
}: {
  occasion: OccasionView;
  members: PlannerMember[];
  busy: boolean;
  onCooking: (group: GroupView, value: number | null) => void;
}) {
  const groups = occasion.groups.filter((group) => isServed(occasion, group, members));
  return (
    <Stack>
      <Stack direction="row" sx={{ justifyContent: 'space-between', alignItems: 'baseline', pb: 1 }}>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{ letterSpacing: '0.06em', textTransform: 'uppercase', fontWeight: 600 }}
        >
          Cooking
        </Typography>
        {groups.length > 1 ? (
          <Typography variant="caption" color="text.secondary">
            {`${groups.length} dishes`}
          </Typography>
        ) : null}
      </Stack>
      {groups.map((group) => (
        <CookingRow
          key={group.id}
          occasion={occasion}
          group={group}
          members={members}
          busy={busy}
          onCooking={(value) => onCooking(group, value)}
        />
      ))}
    </Stack>
  );
}
