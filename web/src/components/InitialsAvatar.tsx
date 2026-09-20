import Box from '@mui/material/Box';

const TINTS = [
  ['#E7F0EA', '#2E6B4F'],
  ['#F3EAE0', '#8A6244'],
  ['#E8EDF2', '#3F5A73'],
  ['#F1EAF0', '#74476B'],
  ['#EFF0E4', '#5E6B32'],
] as const;

export function InitialsAvatar({ name, size = 40 }: { name: string; size?: number }) {
  const seed = [...name].reduce((total, char) => total + char.charCodeAt(0), 0);
  const [bg, fg] = TINTS[seed % TINTS.length]!;
  const initials = name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((word) => word[0]?.toUpperCase() ?? '')
    .join('');

  return (
    <Box
      aria-hidden
      sx={{
        width: size,
        height: size,
        flexShrink: 0,
        borderRadius: '30%',
        display: 'grid',
        placeItems: 'center',
        backgroundColor: bg,
        color: fg,
        fontSize: size * 0.34,
        fontWeight: 600,
        letterSpacing: '0.02em',
      }}
    >
      {initials || '?'}
    </Box>
  );
}
