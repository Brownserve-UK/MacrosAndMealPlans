import ChevronLeftIcon from '@mui/icons-material/ChevronLeftOutlined';
import ChevronRightIcon from '@mui/icons-material/ChevronRightOutlined';
import ShoppingCartIcon from '@mui/icons-material/ShoppingCartOutlined';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import Link from '@mui/material/Link';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Link as RouterLink } from '@tanstack/react-router';
import { addDays } from '../date';
import { shortWeekRange } from './plannerWeek';

export function WeekHeader({
  weekStart,
  currentMonday,
  onWeekChange,
}: {
  weekStart: string;
  currentMonday: string;
  onWeekChange: (weekStart: string) => void;
}) {
  const away = weekStart !== currentMonday;
  const nav = (
    <Stack direction="row" spacing={1} sx={{ alignItems: 'center', justifyContent: 'center' }}>
      <IconButton
        size="small"
        aria-label="Previous week"
        onClick={() => onWeekChange(addDays(weekStart, -7))}
        sx={{ border: '1px solid', borderColor: 'divider', borderRadius: '10px', bgcolor: 'background.paper' }}
      >
        <ChevronLeftIcon fontSize="small" />
      </IconButton>
      <Stack sx={{ minWidth: 132, alignItems: 'center', lineHeight: 1.3 }}>
        <Typography className="numeral" sx={{ fontWeight: 500 }}>
          {shortWeekRange(weekStart)}
        </Typography>
        {away ? (
          <Link
            component="button"
            type="button"
            variant="caption"
            underline="none"
            onClick={() => onWeekChange(currentMonday)}
            sx={{ fontWeight: 500 }}
          >
            Back to this week
          </Link>
        ) : null}
      </Stack>
      <IconButton
        size="small"
        aria-label="Next week"
        onClick={() => onWeekChange(addDays(weekStart, 7))}
        sx={{ border: '1px solid', borderColor: 'divider', borderRadius: '10px', bgcolor: 'background.paper' }}
      >
        <ChevronRightIcon fontSize="small" />
      </IconButton>
    </Stack>
  );

  return (
    <Box sx={{ mb: 3 }}>
      <Box
        sx={{
          display: 'grid',
          gridTemplateColumns: { xs: '1fr auto', md: '1fr auto 1fr' },
          alignItems: 'center',
          gap: 2,
        }}
      >
        <Typography variant="h1">Planner</Typography>
        <Box sx={{ display: { xs: 'none', md: 'block' } }}>{nav}</Box>
        <Box sx={{ justifySelf: 'end' }}>
          <Button
            component={RouterLink}
            to="/shopping"
            startIcon={<ShoppingCartIcon fontSize="small" />}
            sx={{ display: { xs: 'none', md: 'inline-flex' } }}
          >
            Shop for this week
          </Button>
          <IconButton
            component={RouterLink}
            to="/shopping"
            aria-label="Shop for this week"
            sx={{ display: { xs: 'inline-flex', md: 'none' }, border: '1px solid', borderColor: 'divider', borderRadius: '10px' }}
          >
            <ShoppingCartIcon fontSize="small" />
          </IconButton>
        </Box>
      </Box>
      <Box sx={{ display: { xs: 'block', md: 'none' }, mt: 2 }}>{nav}</Box>
    </Box>
  );
}
