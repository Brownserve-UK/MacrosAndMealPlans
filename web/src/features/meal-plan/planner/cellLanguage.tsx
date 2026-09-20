import CallSplitOutlinedIcon from '@mui/icons-material/CallSplitOutlined';
import MeetingRoomOutlinedIcon from '@mui/icons-material/MeetingRoomOutlined';
import PersonAddAltOutlinedIcon from '@mui/icons-material/PersonAddAltOutlined';
import ShoppingCartOutlinedIcon from '@mui/icons-material/ShoppingCartOutlined';
import SoupKitchenOutlinedIcon from '@mui/icons-material/SoupKitchenOutlined';
import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import type { SvgIconProps } from '@mui/material/SvgIcon';
import Typography from '@mui/material/Typography';
import { alpha } from '@mui/material/styles';
import type { ComponentType } from 'react';
import type { AvatarState, CellAvatar, CellMarker, MarkerKind } from './plannerWeek';

export const AVATAR_STATE_TEXT: Record<AvatarState, string> = {
  eating: 'eating this',
  separate: 'eating something else',
  unaccounted: 'needs a meal',
  elsewhere: 'eating elsewhere',
};

export const MARKER_ICON: Record<MarkerKind, ComponentType<SvgIconProps>> = {
  separate: CallSplitOutlinedIcon,
  cook: SoupKitchenOutlinedIcon,
  guests: PersonAddAltOutlinedIcon,
  buy: ShoppingCartOutlinedIcon,
  elsewhere: MeetingRoomOutlinedIcon,
};

export const MARKER_COLOR: Record<CellMarker['tone'], string> = {
  secondary: 'text.secondary',
  brown: 'secondary.main',
  amber: 'warning.main',
};

export const MARKER_TONE: Record<MarkerKind, CellMarker['tone']> = {
  separate: 'secondary',
  cook: 'brown',
  guests: 'secondary',
  buy: 'amber',
  elsewhere: 'secondary',
};

export function MemberAvatar({ avatar, past }: { avatar: CellAvatar; past: boolean }) {
  return (
    <Box
      aria-hidden
      title={`${avatar.name} · ${AVATAR_STATE_TEXT[avatar.state]}`}
      sx={{
        width: 24,
        height: 24,
        flexShrink: 0,
        borderRadius: '50%',
        display: 'grid',
        placeItems: 'center',
        fontSize: '0.6875rem',
        fontWeight: 600,
        lineHeight: 1,
        boxSizing: 'border-box',
        ...(past
          ? { backgroundColor: 'divider', border: '1px solid transparent', color: 'text.disabled' }
          : avatar.state === 'eating'
            ? { backgroundColor: (theme) => alpha(theme.palette.primary.main, 0.18), border: '1px solid transparent', color: 'primary.main' }
            : avatar.state === 'separate'
              ? { backgroundColor: (theme) => alpha(theme.palette.text.secondary, 0.18), border: '1px solid transparent', color: 'text.secondary' }
              : avatar.state === 'unaccounted'
                ? { backgroundColor: 'transparent', border: '1px dashed', borderColor: 'warning.main', color: 'warning.main' }
                : { backgroundColor: 'transparent', border: '1px solid', borderColor: 'divider', color: 'text.disabled' }),
      }}
    >
      {avatar.initials}
    </Box>
  );
}

export function MealMarker({ marker, past }: { marker: CellMarker; past: boolean }) {
  const Icon = MARKER_ICON[marker.kind];
  return (
    <Stack
      direction="row"
      spacing={0.375}
      sx={{ alignItems: 'center', color: past ? 'text.disabled' : MARKER_COLOR[marker.tone] }}
    >
      <Icon sx={{ fontSize: 14 }} />
      <Typography component="span" variant="caption" className="numeral">
        {marker.value}
      </Typography>
    </Stack>
  );
}
