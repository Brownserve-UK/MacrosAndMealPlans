import MoreVertIcon from '@mui/icons-material/MoreVertOutlined';
import Box from '@mui/material/Box';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import { groupShortName, initialsOf, type MemberStatus, memberVariation } from './plannerWeek';
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
  const dish = status.kind === 'eating' ? groupShortName(status.group) : null;

  return (
    <Box
      sx={{
        display: 'grid',
        gridTemplateColumns: '26px auto minmax(0, 1fr) 28px',
        gap: 1.5,
        alignItems: 'center',
        py: 1.5,
        borderTop: '1px solid',
        borderColor: 'divider',
      }}
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
        {status.kind === 'eating' && dish ? (
          <Typography
            component="button"
            type="button"
            variant="body2"
            onClick={() => onEditMeal(status.group)}
            title={dish.tail ? `${dish.head} ${dish.tail}` : dish.head}
            sx={{
              background: 'none',
              border: 0,
              p: 0,
              font: 'inherit',
              cursor: 'pointer',
              color: 'inherit',
              display: 'block',
              width: '100%',
              textAlign: 'right',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {dish.head}
            {dish.tail ? (
              <Typography component="span" variant="body2" color="text.secondary">
                {' '}
                {dish.tail}
              </Typography>
            ) : null}
          </Typography>
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
