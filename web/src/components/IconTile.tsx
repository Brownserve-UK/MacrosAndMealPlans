import Box from '@mui/material/Box';
import { alpha } from '@mui/material/styles';
import { ConceptIcon, type Concept } from './ConceptIcon';

export type Tone = 'primary' | 'secondary' | 'warning' | 'neutral';

export function IconTile({
  concept,
  tone = 'neutral',
  size = 34,
}: {
  concept: Concept;
  tone?: Tone;
  size?: number;
}) {
  return (
    <Box
      sx={(theme) => ({
        width: size,
        height: size,
        flexShrink: 0,
        borderRadius: '10px',
        display: 'grid',
        placeItems: 'center',
        backgroundColor:
          tone === 'neutral'
            ? 'background.default'
            : theme.vars
              ? `rgba(${theme.vars.palette[tone].mainChannel} / 0.12)`
              : alpha(theme.palette[tone].main, 0.12),
        color: tone === 'neutral' ? 'text.secondary' : `${tone}.main`,
      })}
    >
      <ConceptIcon concept={concept} size={Math.round(size / 2)} />
    </Box>
  );
}
