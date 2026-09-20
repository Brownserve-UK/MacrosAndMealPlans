import CallSplitOutlinedIcon from '@mui/icons-material/CallSplitOutlined';
import MeetingRoomOutlinedIcon from '@mui/icons-material/MeetingRoomOutlined';
import PersonAddAltOutlinedIcon from '@mui/icons-material/PersonAddAltOutlined';
import ShoppingCartOutlinedIcon from '@mui/icons-material/ShoppingCartOutlined';
import SoupKitchenOutlinedIcon from '@mui/icons-material/SoupKitchenOutlined';
import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { SvgIconProps } from '@mui/material/SvgIcon';
import { alpha } from '@mui/material/styles';
import type { ComponentType, DragEvent, KeyboardEvent, MouseEvent } from 'react';
import { forwardRef } from 'react';
import type { MealSlot } from '../../../api/client';
import { labelForSlot } from '../slots';
import type { AvatarState, CellAvatar, CellDetail, CellMarker, MarkerKind } from './plannerWeek';
import { shortDate } from './plannerWeek';
import type { OccasionView } from './types';

export type CellProps = {
  date: string;
  slot: MealSlot;
  occasion: OccasionView | null;
  detail: CellDetail | null;
  past: boolean;
  tabIndex: number;
  onOpen: (occasion: OccasionView, anchor: HTMLElement) => void;
  onAdd: (date: string, slot: MealSlot, initial: string, anchor: HTMLElement) => void;
  onContextMenu: (occasion: OccasionView, position: { left: number; top: number }) => void;
  onNavigate: (deltaRow: number, deltaCol: number) => void;
  onDrop: (occasionId: string, date: string, slot: MealSlot, copy: boolean) => void;
  onFocus: () => void;
};

const AVATAR_STATE_TEXT: Record<AvatarState, string> = {
  eating: 'eating this',
  separate: 'eating something else',
  unaccounted: 'needs a meal',
  elsewhere: 'eating elsewhere',
};

const MARKER_ICON: Record<MarkerKind, ComponentType<SvgIconProps>> = {
  separate: CallSplitOutlinedIcon,
  cook: SoupKitchenOutlinedIcon,
  guests: PersonAddAltOutlinedIcon,
  buy: ShoppingCartOutlinedIcon,
  elsewhere: MeetingRoomOutlinedIcon,
};

const MARKER_COLOR: Record<CellMarker['tone'], string> = {
  secondary: 'text.secondary',
  brown: 'secondary.main',
  amber: 'warning.main',
};

function isPrintable(event: KeyboardEvent) {
  return event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey;
}

function MemberAvatar({ avatar, past }: { avatar: CellAvatar; past: boolean }) {
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
      {avatar.initials || '?'}
    </Box>
  );
}

function MealMarker({ marker, past }: { marker: CellMarker; past: boolean }) {
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

export const PlannerCell = forwardRef<HTMLButtonElement, CellProps>(function PlannerCell(
  { date, slot, occasion, detail, past, tabIndex, onOpen, onAdd, onContextMenu, onNavigate, onDrop, onFocus },
  ref,
) {
  const needingMeal = detail?.avatars.filter((avatar) => avatar.state === 'unaccounted') ?? [];
  const needingMealSuffix =
    needingMeal.length > 0
      ? `. ${needingMeal.map((avatar) => avatar.name).join(', ')} ${needingMeal.length === 1 ? 'needs' : 'need'} a meal`
      : '';
  const label = occasion
    ? `${labelForSlot(slot)} on ${shortDate(date)}: ${detail?.name ?? ''}${needingMealSuffix}`
    : `Plan ${labelForSlot(slot).toLowerCase()} on ${shortDate(date)}`;

  function keyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (event.key === 'ArrowLeft') return step(event, 0, -1);
    if (event.key === 'ArrowRight') return step(event, 0, 1);
    if (event.key === 'ArrowUp') return step(event, -1, 0);
    if (event.key === 'ArrowDown') return step(event, 1, 0);
    if (event.key === 'Enter') {
      event.preventDefault();
      if (occasion) onOpen(occasion, event.currentTarget);
      else onAdd(date, slot, '', event.currentTarget);
      return;
    }
    if (!occasion && isPrintable(event) && event.key !== ' ') {
      event.preventDefault();
      onAdd(date, slot, event.key, event.currentTarget);
    }
  }

  function step(event: KeyboardEvent, deltaRow: number, deltaCol: number) {
    event.preventDefault();
    onNavigate(deltaRow, deltaCol);
  }

  function contextMenu(event: MouseEvent<HTMLButtonElement>) {
    if (!occasion) return;
    event.preventDefault();
    onContextMenu(occasion, { left: event.clientX, top: event.clientY });
  }

  function dragStart(event: DragEvent<HTMLButtonElement>) {
    if (!occasion) return;
    event.dataTransfer.setData('text/plain', occasion.id);
    event.dataTransfer.effectAllowed = 'copyMove';
  }

  function dragOver(event: DragEvent<HTMLButtonElement>) {
    if (occasion) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = event.altKey ? 'copy' : 'move';
  }

  function drop(event: DragEvent<HTMLButtonElement>) {
    if (occasion) return;
    event.preventDefault();
    const id = event.dataTransfer.getData('text/plain');
    if (id) onDrop(id, date, slot, event.altKey);
  }

  return (
    <Box
      ref={ref}
      component="button"
      type="button"
      tabIndex={tabIndex}
      aria-label={label}
      draggable={Boolean(occasion)}
      onClick={(event: MouseEvent<HTMLButtonElement>) =>
        occasion ? onOpen(occasion, event.currentTarget) : onAdd(date, slot, '', event.currentTarget)
      }
      onKeyDown={keyDown}
      onContextMenu={contextMenu}
      onDragStart={dragStart}
      onDragOver={dragOver}
      onDrop={drop}
      onFocus={onFocus}
      sx={{
        position: 'relative',
        minHeight: 112,
        px: 1.75,
        py: 1.5,
        borderRadius: '10px',
        border: '1px solid',
        borderColor: occasion ? 'divider' : past ? 'transparent' : 'divider',
        borderStyle: occasion ? 'solid' : 'dashed',
        backgroundColor: occasion && !past ? 'background.paper' : 'transparent',
        color: past ? 'text.disabled' : 'text.primary',
        textAlign: 'left',
        font: 'inherit',
        cursor: 'pointer',
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'space-between',
        gap: 1,
        '&:focus-visible': { outline: '2px solid', outlineColor: 'primary.main', outlineOffset: 1 },
        ...(occasion
          ? {}
          : {
              '& .plus': { opacity: 0 },
              '&:hover, &:focus-visible': {
                borderColor: 'text.disabled',
                backgroundColor: 'action.hover',
                '& .plus': { opacity: 1 },
              },
            }),
      }}
    >
      {occasion && detail ? (
        <>
          <Typography
            component="span"
            variant="body1"
            title={detail.tail ? `${detail.name} ${detail.tail}` : detail.name}
            sx={{
              fontWeight: 500,
              lineHeight: 1.3,
              color: 'inherit',
              display: '-webkit-box',
              WebkitBoxOrient: 'vertical',
              WebkitLineClamp: 3,
              overflow: 'hidden',
            }}
          >
            {detail.name}
            {detail.tail ? (
              <Typography component="span" variant="body1" sx={{ color: past ? 'inherit' : 'text.secondary' }}>
                {' '}
                {detail.tail}
              </Typography>
            ) : null}
          </Typography>
          <Stack spacing={0.75}>
            {detail.avatars.length > 0 ? (
              <Stack direction="row" spacing={0.5} sx={{ flexWrap: 'wrap', rowGap: 0.5 }}>
                {detail.avatars.map((avatar) => (
                  <MemberAvatar key={avatar.memberId} avatar={avatar} past={past} />
                ))}
              </Stack>
            ) : null}
            {detail.markers.length > 0 ? (
              <Stack direction="row" spacing={1.25} sx={{ flexWrap: 'wrap', rowGap: 0.25 }}>
                {detail.markers.map((marker) => (
                  <MealMarker key={marker.kind} marker={marker} past={past} />
                ))}
              </Stack>
            ) : null}
          </Stack>
        </>
      ) : (
        <Box
          className="plus"
          aria-hidden
          sx={{
            position: 'absolute',
            inset: 0,
            display: 'grid',
            placeItems: 'center',
            color: 'primary.main',
            fontSize: '1.5rem',
            transition: 'opacity 120ms',
          }}
        >
          +
        </Box>
      )}
    </Box>
  );
});
