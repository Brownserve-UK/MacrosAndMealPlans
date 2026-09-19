import Box from '@mui/material/Box';
import Checkbox from '@mui/material/Checkbox';
import IconButton from '@mui/material/IconButton';
import EditIcon from '@mui/icons-material/EditOutlined';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { ShoppingListItem } from '../../api/client';
import { formatQuantity } from '../stock/SpokenFor';

export function ManualRow({
  item,
  bought,
  onToggle,
  onEdit,
}: {
  item: ShoppingListItem;
  bought: boolean;
  onToggle: (next: boolean) => void;
  onEdit: () => void;
}) {
  const amount = item.quantity ? formatQuantity(item.quantity) : null;

  return (
    <Stack
      direction="row"
      spacing={1.5}
      sx={{ alignItems: 'center', px: 1, py: 0.5, borderTop: 1, borderColor: 'divider' }}
    >
      <Checkbox
        checked={bought}
        onChange={(_, checked) => onToggle(checked)}
        slotProps={{ input: { 'aria-label': `Bought ${item.name}` } }}
      />

      <Box sx={{ flex: 1, minWidth: 0, display: 'flex', alignItems: 'center', gap: 1.5, py: 1 }}>
        <Typography
          variant="body1"
          sx={{
            flexGrow: 1,
            minWidth: 0,
            fontWeight: 500,
            color: bought ? 'text.disabled' : 'text.primary',
            textDecoration: bought ? 'line-through' : 'none',
          }}
        >
          {item.name}
        </Typography>

        {amount ? (
          <Typography
            variant="body1"
            className="numeral"
            sx={{ fontWeight: 500, color: bought ? 'text.disabled' : 'text.primary' }}
          >
            {amount}
          </Typography>
        ) : null}
      </Box>

      <IconButton size="small" aria-label={`Edit ${item.name}`} onClick={onEdit}>
        <EditIcon fontSize="small" />
      </IconButton>
    </Stack>
  );
}
