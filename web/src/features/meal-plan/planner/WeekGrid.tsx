import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { useRef, useState } from 'react';
import type { MealSlot } from '../../../api/client';
import { parseIsoDate } from '../date';
import { labelForSlot } from '../slots';
import { PlannerCell } from './PlannerCell';
import { cellDetail, occasionAt, SLOT_ORDER, weekDates } from './plannerWeek';
import type { OccasionView, PlannerWeek } from './types';

export function WeekGrid({
  week,
  weekStart,
  today,
  onOpen,
  onAdd,
  onContextMenu,
  onDrop,
}: {
  week: PlannerWeek;
  weekStart: string;
  today: string;
  onOpen: (occasion: OccasionView, anchor: HTMLElement) => void;
  onAdd: (date: string, slot: MealSlot, initial: string, anchor: HTMLElement) => void;
  onContextMenu: (occasion: OccasionView, position: { left: number; top: number }) => void;
  onDrop: (occasionId: string, date: string, slot: MealSlot, copy: boolean) => void;
}) {
  const dates = weekDates(weekStart);
  const [focused, setFocused] = useState<{ row: number; col: number }>({
    row: 2,
    col: Math.max(0, dates.indexOf(today)),
  });
  const cells = useRef<(HTMLButtonElement | null)[][]>(SLOT_ORDER.map(() => dates.map(() => null)));

  function navigate(deltaRow: number, deltaCol: number) {
    const row = Math.min(SLOT_ORDER.length - 1, Math.max(0, focused.row + deltaRow));
    const col = Math.min(dates.length - 1, Math.max(0, focused.col + deltaCol));
    setFocused({ row, col });
    cells.current[row]?.[col]?.focus();
  }

  return (
    <Box
      role="grid"
      aria-label="Week plan"
      sx={{
        display: 'grid',
        gridTemplateColumns: '92px repeat(7, minmax(0, 1fr))',
        gap: 1,
        alignItems: 'stretch',
      }}
    >
      <Box />
      {dates.map((date) => {
        const parsed = parseIsoDate(date);
        const isToday = date === today;
        const past = date < today;
        return (
          <Box
            key={date}
            role="columnheader"
            sx={{
              textAlign: 'center',
              px: 0.5,
              pt: 1,
              pb: 0.75,
              borderRadius: '10px',
              lineHeight: 1.2,
              backgroundColor: isToday ? 'action.selected' : 'transparent',
            }}
          >
            <Typography
              variant="caption"
              sx={{ display: 'block', color: isToday ? 'primary.main' : past ? 'text.disabled' : 'text.secondary', fontWeight: isToday ? 600 : 400 }}
            >
              {parsed.toLocaleDateString('en-GB', { weekday: 'short' })}
            </Typography>
            <Typography className="numeral" sx={{ fontWeight: 600, fontSize: '1.125rem', color: past ? 'text.disabled' : 'text.primary' }}>
              {parsed.getDate()}
            </Typography>
          </Box>
        );
      })}

      {SLOT_ORDER.map((slot, row) => (
        <Box key={slot} sx={{ display: 'contents' }} role="row">
          <Typography variant="body2" role="rowheader" sx={{ fontWeight: 500, color: 'text.secondary', pt: 1.75 }}>
            {labelForSlot(slot)}
          </Typography>
          {dates.map((date, col) => {
            const occasion = occasionAt(week, date, slot);
            return (
              <PlannerCell
                key={`${date}-${slot}`}
                ref={(element) => {
                  const rowRefs = cells.current[row];
                  if (rowRefs) rowRefs[col] = element;
                }}
                date={date}
                slot={slot}
                occasion={occasion}
                detail={occasion ? cellDetail(occasion, week.members) : null}
                past={date < today}
                tabIndex={focused.row === row && focused.col === col ? 0 : -1}
                onOpen={onOpen}
                onAdd={onAdd}
                onContextMenu={onContextMenu}
                onNavigate={navigate}
                onDrop={onDrop}
                onFocus={() => setFocused({ row, col })}
              />
            );
          })}
        </Box>
      ))}
    </Box>
  );
}
