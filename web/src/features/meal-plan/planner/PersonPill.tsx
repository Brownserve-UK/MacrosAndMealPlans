import AddIcon from '@mui/icons-material/AddOutlined';
import Box from '@mui/material/Box';
import ButtonBase from '@mui/material/ButtonBase';
import Typography from '@mui/material/Typography';
import type { MouseEvent, ReactNode } from 'react';
import { initialsOf } from './plannerWeek';
import type { PlannerMember } from './types';

function Circle({ children, off }: { children: ReactNode; off?: boolean }) {
  return (
    <Box
      aria-hidden
      sx={{
        width: 24,
        height: 24,
        borderRadius: '50%',
        flexShrink: 0,
        display: 'grid',
        placeItems: 'center',
        fontSize: '0.68rem',
        fontWeight: 600,
        backgroundColor: off ? 'transparent' : 'divider',
        border: off ? '1px dashed' : 'none',
        borderColor: off ? 'text.disabled' : undefined,
        color: off ? 'text.disabled' : 'text.secondary',
      }}
    >
      {children}
    </Box>
  );
}

const pillSx = (off: boolean, ghost: boolean) => ({
  display: 'inline-flex',
  alignItems: 'center',
  gap: 1,
  pl: ghost ? 1.25 : 0.5,
  pr: 1.5,
  py: 0.5,
  borderRadius: 999,
  border: '1px solid',
  borderStyle: off || ghost ? 'dashed' : 'solid',
  borderColor: 'divider',
  backgroundColor: off || ghost ? 'transparent' : 'background.paper',
  color: ghost ? 'primary.main' : off ? 'text.disabled' : 'text.primary',
  fontSize: '0.875rem',
  fontWeight: 500,
  textAlign: 'left',
});

export function PersonPill({
  member,
  variation,
  off = false,
  onClick,
}: {
  member: PlannerMember;
  variation?: string | null;
  off?: boolean;
  onClick: (event: MouseEvent<HTMLButtonElement>) => void;
}) {
  return (
    <ButtonBase onClick={onClick} aria-label={off ? `${member.name}, not eating this` : member.name} sx={pillSx(off, false)}>
      <Circle off={off}>{initialsOf(member)}</Circle>
      <span>
        {member.name}
        {variation ? (
          <Typography component="span" variant="body2" sx={{ color: 'text.secondary', ml: 0.5 }}>
            · {variation}
          </Typography>
        ) : null}
      </span>
    </ButtonBase>
  );
}

export function EveryonePill({
  members,
  onClick,
}: {
  members: PlannerMember[];
  onClick: (event: MouseEvent<HTMLButtonElement>) => void;
}) {
  return (
    <ButtonBase onClick={onClick} aria-label="Everyone" sx={{ ...pillSx(false, false), pl: 0.75 }}>
      <Box sx={{ display: 'inline-flex' }}>
        {members.map((member, index) => (
          <Box
            key={member.id}
            sx={{
              mr: index === members.length - 1 ? 0 : '-8px',
              borderRadius: '50%',
              border: '2px solid',
              borderColor: 'background.paper',
            }}
          >
            <Circle>{initialsOf(member)}</Circle>
          </Box>
        ))}
      </Box>
      Everyone
    </ButtonBase>
  );
}

export function GhostPill({
  label,
  onClick,
}: {
  label: string;
  onClick: (event: MouseEvent<HTMLButtonElement>) => void;
}) {
  return (
    <ButtonBase onClick={onClick} sx={pillSx(false, true)}>
      <AddIcon sx={{ fontSize: 14 }} />
      {label}
    </ButtonBase>
  );
}
