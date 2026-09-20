import Typography from '@mui/material/Typography';
import { groupShortName } from './plannerWeek';
import type { GroupView } from './types';

export const dinerRowSx = {
  display: 'grid', gridTemplateColumns: '26px minmax(0, 1fr) minmax(0, 1fr) 28px', gap: 1.5,
  alignItems: 'center', py: 1.5, borderTop: '1px solid', borderColor: 'divider',
} as const;

export function DinerMeal({ group, onClick }: { group: GroupView; onClick?: () => void }) {
  const dish = groupShortName(group);
  return <Typography component={onClick ? 'button' : 'span'} type={onClick ? 'button' : undefined}
    variant="body1" onClick={onClick} title={dish.tail ? `${dish.head} ${dish.tail}` : dish.head}
    sx={{ background: 'none', border: 0, p: 0, cursor: onClick ? 'pointer' : undefined,
      color: 'text.primary', display: 'block', width: '100%', minWidth: 0, textAlign: 'right',
      overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
    {dish.head}
    {dish.tail ? <Typography component="span" variant="body2" color="text.secondary">{' '}{dish.tail}</Typography> : null}
  </Typography>;
}
