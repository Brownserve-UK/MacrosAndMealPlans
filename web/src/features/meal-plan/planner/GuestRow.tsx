import AddIcon from '@mui/icons-material/AddOutlined';
import MoreVertIcon from '@mui/icons-material/MoreVertOutlined';
import Box from '@mui/material/Box';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import { Fragment } from 'react';
import { dishLabel } from './plannerWeek';
import type { GroupView, OccasionView } from './types';

const rowSx = {
  display: 'grid',
  gridTemplateColumns: '26px 1fr auto 28px',
  gap: 1.5,
  alignItems: 'center',
  py: 1.5,
  borderTop: '1px solid',
  borderColor: 'divider',
} as const;

function GuestAvatar({ label }: { label: string }) {
  return (
    <Box
      aria-hidden
      sx={{
        width: 26,
        height: 26,
        borderRadius: '50%',
        border: '1px dashed',
        borderColor: 'divider',
        display: 'grid',
        placeItems: 'center',
        fontSize: '0.7rem',
        fontWeight: 600,
        color: 'primary.main',
      }}
    >
      {label}
    </Box>
  );
}

export function GuestRows({
  occasion,
  onAdd,
  onOpen,
}: {
  occasion: OccasionView;
  onAdd: (group: GroupView) => void;
  onOpen: (group: GroupView, anchor: HTMLElement) => void;
}) {
  const withGuests = occasion.groups.filter((group) => group.guest_count > 0);
  const showDish = occasion.groups.length > 1;

  if (withGuests.length > 0) {
    return (
      <Fragment>
        {withGuests.map((group) => (
          <Box key={group.id} sx={rowSx}>
            <GuestAvatar label={String(group.guest_count)} />
            <Typography sx={{ color: 'text.secondary' }}>
              {group.guest_count === 1 ? '1 guest' : `${group.guest_count} guests`}
            </Typography>
            <Box sx={{ justifySelf: 'end', textAlign: 'right' }}>
              {showDish ? <Typography variant="body2">{dishLabel(group)}</Typography> : null}
            </Box>
            <IconButton
              size="small"
              aria-label={`Change guests on ${group.name}`}
              onClick={(event) => onOpen(group, event.currentTarget)}
              sx={{ color: 'text.disabled' }}
            >
              <MoreVertIcon sx={{ fontSize: 18 }} />
            </IconButton>
          </Box>
        ))}
      </Fragment>
    );
  }

  const first = occasion.groups[0];

  return (
    <Box sx={rowSx}>
      <GuestAvatar label="" />
      <Typography sx={{ color: 'text.secondary' }}>Guest</Typography>
      <Box />
      <IconButton
        size="small"
        aria-label="Add a guest"
        disabled={!first}
        onClick={(event) => {
          if (!first) return;
          if (occasion.groups.length === 1) onAdd(first);
          else onOpen(first, event.currentTarget);
        }}
        sx={{ color: 'text.disabled' }}
      >
        <AddIcon sx={{ fontSize: 16 }} />
      </IconButton>
    </Box>
  );
}
