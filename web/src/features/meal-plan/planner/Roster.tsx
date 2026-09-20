import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { GuestRows } from './GuestRow';
import { MemberRow } from './MemberRow';
import { groupDiners, memberStatus } from './plannerWeek';
import type { GroupView, OccasionView, PlannerGuest, PlannerMember, PlannerWeek } from './types';

function summary(occasion: OccasionView, members: PlannerMember[]): string {
  if (occasion.unaccounted_member_ids.length > 0) {
    return `${occasion.unaccounted_member_ids.length} still to sort`;
  }
  const groupsWithDiners = occasion.groups
    .map((group) => groupDiners(occasion, group, members))
    .filter((diners) => diners.length > 0);
  if (groupsWithDiners.length === 1 && groupsWithDiners[0]!.length === members.length) {
    return 'Everyone';
  }
  const eating = members.length - occasion.absent_member_ids.length - occasion.unaccounted_member_ids.length;
  const guests = occasion.groups.reduce((total, group) => total + group.guest_count, 0);
  const total = eating + guests;
  return `${total} ${total === 1 ? 'person' : 'people'}`;
}

export function Roster({
  occasion,
  week,
  busy,
  onOpenMember,
  onAddGuest,
  onOpenGuests,
  onRenameGuest,
  onEditMeal,
}: {
  occasion: OccasionView;
  week: PlannerWeek;
  busy: boolean;
  onOpenMember: (member: PlannerMember, anchor: HTMLElement) => void;
  onAddGuest: (anchor: HTMLElement) => void;
  onOpenGuests: (group: GroupView, guest: PlannerGuest, anchor: HTMLElement) => void;
  onRenameGuest: (guest: PlannerGuest, name: string | null) => Promise<boolean>;
  onEditMeal: (group: GroupView) => void;
}) {
  const members = week.members;
  return (
    <Stack>
      <Stack direction="row" sx={{ justifyContent: 'space-between', alignItems: 'baseline', pb: 1 }}>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{ letterSpacing: '0.06em', textTransform: 'uppercase', fontWeight: 600 }}
        >
          Eating
        </Typography>
        <Typography variant="caption" color="text.secondary" className="numeral">
          {summary(occasion, members)}
        </Typography>
      </Stack>
      {members.map((member) => (
        <MemberRow
          key={member.id}
          member={member}
          status={memberStatus(occasion, member, members)}
          busy={busy}
          onOpen={(anchor) => onOpenMember(member, anchor)}
          onEditMeal={onEditMeal}
        />
      ))}
      <GuestRows occasion={occasion} onAdd={onAddGuest} onOpen={onOpenGuests} onRename={onRenameGuest} />
    </Stack>
  );
}
