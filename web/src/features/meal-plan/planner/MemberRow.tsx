import MoreVertIcon from '@mui/icons-material/MoreVertOutlined';
import Box from '@mui/material/Box';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import { initialsOf, type MemberStatus, memberVariation } from './plannerWeek';
import { DinerMeal, dinerRowSx } from './DinerRow';
import type { GroupView, PlannerMember } from './types';

export function MemberRow({
  member,
  status,
  busy,
  onOpen,
  onEditMeal,
}: {
  member: PlannerMember;
  status: MemberStatus;
  busy: boolean;
  onOpen: (anchor: HTMLElement) => void;
  onEditMeal: (group: GroupView) => void;
}) {
  const variation = status.kind === 'eating' ? memberVariation(status.group, member.id) : null;

  return (
    <Box
      sx={dinerRowSx}
    >
      <Box
        aria-hidden
        sx={{
          width: 26,
          height: 26,
          borderRadius: '50%',
          display: 'grid',
          placeItems: 'center',
          fontSize: '0.7rem',
          fontWeight: 600,
          backgroundColor: status.kind === 'unaccounted' ? 'transparent' : 'divider',
          border: status.kind === 'unaccounted' ? '1px dashed' : 'none',
          borderColor: status.kind === 'unaccounted' ? 'text.disabled' : undefined,
          color: status.kind === 'elsewhere' || status.kind === 'unaccounted' ? 'text.disabled' : 'text.secondary',
        }}
      >
        {initialsOf(member)}
      </Box>
      <Typography sx={{ fontWeight: 500, lineHeight: 1.25, minWidth: 0 }} noWrap>
        {member.name}
        {variation ? (
          <Typography component="span" variant="body2" color="text.secondary" sx={{ ml: 0.5 }}>
            · {variation}
          </Typography>
        ) : null}
      </Typography>
      <Box sx={{ textAlign: 'right', minWidth: 0 }}>
        {status.kind === 'eating' ? (
          <DinerMeal group={status.group} onClick={() => onEditMeal(status.group)} />
        ) : status.kind === 'elsewhere' ? (
          <Typography variant="body2" color="text.secondary">
            Eating elsewhere
          </Typography>
        ) : (
          <Typography variant="body2" sx={{ fontWeight: 500, color: 'warning.main' }}>
            Needs a meal
          </Typography>
        )}
      </Box>
      <IconButton
        size="small"
        aria-label={`Change what ${member.name} is eating`}
        disabled={busy}
        onClick={(event) => onOpen(event.currentTarget)}
        sx={{ color: 'text.disabled' }}
      >
        <MoreVertIcon sx={{ fontSize: 18 }} />
      </IconButton>
    </Box>
  );
}
