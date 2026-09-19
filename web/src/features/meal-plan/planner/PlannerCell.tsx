import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { DragEvent, KeyboardEvent, MouseEvent } from 'react';
import { forwardRef } from 'react';
import type { MealSlot } from '../../../api/client';
import { labelForSlot } from '../slots';
import type { CellSummary } from './plannerWeek';
import { shortDate } from './plannerWeek';
import type { OccasionView } from './types';

export type CellProps = {
  date: string;
  slot: MealSlot;
  occasion: OccasionView | null;
  summary: CellSummary | null;
  past: boolean;
  tabIndex: number;
  onOpen: (occasion: OccasionView, anchor: HTMLElement) => void;
  onAdd: (date: string, slot: MealSlot, initial: string, anchor: HTMLElement) => void;
  onContextMenu: (occasion: OccasionView, position: { left: number; top: number }) => void;
  onNavigate: (deltaRow: number, deltaCol: number) => void;
  onDrop: (occasionId: string, date: string, slot: MealSlot, copy: boolean) => void;
  onFocus: () => void;
};

function isPrintable(event: KeyboardEvent) {
  return event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey;
}

export const PlannerCell = forwardRef<HTMLButtonElement, CellProps>(function PlannerCell(
  { date, slot, occasion, summary, past, tabIndex, onOpen, onAdd, onContextMenu, onNavigate, onDrop, onFocus },
  ref,
) {
  const label = occasion
    ? `${labelForSlot(slot)} on ${shortDate(date)}: ${summary?.name ?? ''}`
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
        minHeight: 82,
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
      {occasion && summary ? (
        <>
          <Typography
            component="span"
            variant="body1"
            title={summary.tail ? `${summary.name} ${summary.tail}` : summary.name}
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
            {summary.name}
            {summary.tail ? (
              <Typography component="span" variant="body1" sx={{ color: past ? 'inherit' : 'text.secondary' }}>
                {' '}
                {summary.tail}
              </Typography>
            ) : null}
          </Typography>
          {summary.meta.length > 0 ? (
            <Stack direction="row" spacing={1.25} sx={{ flexWrap: 'wrap', rowGap: 0.25 }}>
              {summary.meta.map((item) => (
                <Typography
                  key={item.text}
                  component="span"
                  variant="caption"
                  className="numeral"
                  sx={{ color: past ? 'inherit' : item.tone === 'buy' ? 'warning.main' : 'text.secondary' }}
                >
                  {item.text}
                </Typography>
              ))}
            </Stack>
          ) : null}
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
