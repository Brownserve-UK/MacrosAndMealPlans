import Box from '@mui/material/Box';
import ButtonBase from '@mui/material/ButtonBase';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useEffect, useRef } from 'react';
import type { MealSlot } from '../../../api/client';
import { labelForSlot } from '../slots';
import { cellSummary, dayHeading, occasionAt, SLOT_ORDER, weekDates } from './plannerWeek';
import type { OccasionView, PlannerWeek } from './types';

const LONG_PRESS_MS = 500;

export function DayCards({
  week,
  weekStart,
  today,
  onOpen,
  onAdd,
  onLongPress,
}: {
  week: PlannerWeek;
  weekStart: string;
  today: string;
  onOpen: (occasion: OccasionView, anchor: HTMLElement) => void;
  onAdd: (date: string, slot: MealSlot, initial: string, anchor: HTMLElement) => void;
  onLongPress: (occasion: OccasionView, position: { left: number; top: number }) => void;
}) {
  const todayRef = useRef<HTMLDivElement | null>(null);
  const timer = useRef<number | null>(null);

  useEffect(() => {
    todayRef.current?.scrollIntoView({ block: 'start' });
  }, [weekStart]);

  function pressStart(occasion: OccasionView | null, position: { left: number; top: number }) {
    if (!occasion) return;
    timer.current = window.setTimeout(() => {
      timer.current = null;
      onLongPress(occasion, position);
    }, LONG_PRESS_MS);
  }

  function pressEnd() {
    if (timer.current !== null) {
      window.clearTimeout(timer.current);
      timer.current = null;
    }
  }

  return (
    <Stack spacing={2}>
      {weekDates(weekStart).map((date) => {
        const past = date < today;
        const isToday = date === today;
        return (
          <Paper
            key={date}
            ref={isToday ? todayRef : undefined}
            sx={{ px: 2, py: 1.75, backgroundColor: past ? 'transparent' : 'background.paper', scrollMarginTop: 16 }}
          >
            <Stack direction="row" spacing={1} sx={{ alignItems: 'baseline', mb: 1 }}>
              <Typography sx={{ fontWeight: 600, color: past ? 'text.disabled' : 'text.primary' }}>
                {dayHeading(date)}
              </Typography>
              {isToday ? (
                <Typography variant="caption" sx={{ color: 'primary.main', fontWeight: 600 }}>
                  Today
                </Typography>
              ) : null}
            </Stack>
            {SLOT_ORDER.map((slot) => {
              const occasion = occasionAt(week, date, slot);
              const summary = occasion ? cellSummary(occasion, week.members) : null;
              return (
                <ButtonBase
                  key={slot}
                  aria-label={
                    occasion
                      ? `${labelForSlot(slot)}: ${summary?.name ?? ''}`
                      : `Plan ${labelForSlot(slot).toLowerCase()} on ${dayHeading(date)}`
                  }
                  onClick={(event) =>
                    occasion ? onOpen(occasion, event.currentTarget) : onAdd(date, slot, '', event.currentTarget)
                  }
                  onTouchStart={(event) => {
                    const touch = event.touches[0];
                    pressStart(occasion, { left: touch?.clientX ?? 0, top: touch?.clientY ?? 0 });
                  }}
                  onTouchEnd={pressEnd}
                  onTouchMove={pressEnd}
                  onContextMenu={(event) => {
                    if (!occasion) return;
                    event.preventDefault();
                    onLongPress(occasion, { left: event.clientX, top: event.clientY });
                  }}
                  sx={{
                    display: 'grid',
                    gridTemplateColumns: '76px 1fr',
                    gap: 1.5,
                    width: '100%',
                    alignItems: 'baseline',
                    textAlign: 'left',
                    py: 0.875,
                    borderTop: '1px solid',
                    borderColor: 'divider',
                    borderRadius: 0,
                  }}
                >
                  <Typography variant="caption" sx={{ color: past ? 'text.disabled' : 'text.secondary' }}>
                    {labelForSlot(slot)}
                  </Typography>
                  {occasion && summary ? (
                    <Box sx={{ minWidth: 0 }}>
                      <Typography component="span" sx={{ fontWeight: 500, color: past ? 'text.disabled' : 'text.primary' }}>
                        {summary.name}
                      </Typography>
                      {summary.meta.map((item) => (
                        <Typography
                          key={item.text}
                          component="span"
                          variant="caption"
                          className="numeral"
                          sx={{ ml: 1, color: past ? 'text.disabled' : item.tone === 'buy' ? 'warning.main' : 'text.secondary' }}
                        >
                          {item.text}
                        </Typography>
                      ))}
                    </Box>
                  ) : (
                    <Typography component="span" sx={{ color: 'text.disabled' }}>
                      –
                    </Typography>
                  )}
                </ButtonBase>
              );
            })}
          </Paper>
        );
      })}
    </Stack>
  );
}
