import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { ReactNode } from 'react';
import { MARKER_COLOR, MARKER_ICON, MARKER_TONE, MemberAvatar } from './cellLanguage';
import { initialsOf } from './plannerWeek';
import type { AvatarState, MarkerKind } from './plannerWeek';
import type { PlannerMember } from './types';

const PEOPLE: { state: AvatarState; label: string }[] = [
  { state: 'eating', label: 'Eating this' },
  { state: 'separate', label: 'Eating something else' },
  { state: 'unaccounted', label: 'Needs a meal' },
  { state: 'elsewhere', label: 'Out' },
];

const MEALS: { kind: MarkerKind; label: string }[] = [
  { kind: 'cook', label: 'Needs cooking' },
  { kind: 'buy', label: 'Not in the cupboard' },
  { kind: 'guests', label: 'Guests' },
  { kind: 'separate', label: 'Multiple meals' },
  { kind: 'elsewhere', label: 'Someone out' },
];

function LegendItem({ label, children }: { label: string; children: ReactNode }) {
  return (
    <Stack direction="row" spacing={0.75} sx={{ alignItems: 'center' }}>
      {children}
      <Typography variant="caption" sx={{ color: 'text.secondary' }}>
        {label}
      </Typography>
    </Stack>
  );
}

export function WeekLegend({ members }: { members: PlannerMember[] }) {
  const sample = members[0];
  const initials = sample ? initialsOf(sample) : '';
  return (
    <Stack
      component="aside"
      aria-label="What the symbols mean"
      spacing={1}
      sx={{ mt: 3, alignItems: 'flex-start' }}
    >
      <Stack direction="row" spacing={2} sx={{ flexWrap: 'wrap', rowGap: 1, alignItems: 'center' }}>
        {PEOPLE.map((item) => (
          <LegendItem key={item.state} label={item.label}>
            <MemberAvatar
              avatar={{ memberId: item.state, name: item.label, initials, state: item.state }}
              past={false}
            />
          </LegendItem>
        ))}
      </Stack>
      <Stack direction="row" spacing={2} sx={{ flexWrap: 'wrap', rowGap: 1, alignItems: 'center' }}>
        {MEALS.map((item) => {
          const Icon = MARKER_ICON[item.kind];
          return (
            <LegendItem key={item.kind} label={item.label}>
              <Icon sx={{ fontSize: 14, color: MARKER_COLOR[MARKER_TONE[item.kind]] }} />
            </LegendItem>
          );
        })}
      </Stack>
    </Stack>
  );
}
