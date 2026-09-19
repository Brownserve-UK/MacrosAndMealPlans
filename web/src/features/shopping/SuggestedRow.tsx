import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import CloseIcon from '@mui/icons-material/CloseOutlined';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { ShoppingRequirement } from '../../api/client';
import { amountOf } from './RequirementCard';

export function SuggestedRow({
  requirement,
  onAdd,
  onDismiss,
}: {
  requirement: ShoppingRequirement;
  onAdd: () => void;
  onDismiss: () => void;
}) {
  const amount = amountOf(requirement);

  return (
    <Stack
      direction="row"
      spacing={1}
      sx={{ alignItems: 'center', pl: 7, pr: 1, py: 1, borderTop: 1, borderColor: 'divider' }}
    >
      <Typography variant="body1" sx={{ flexGrow: 1, minWidth: 0, color: 'text.secondary' }}>
        {requirement.name}
      </Typography>
      {amount ? (
        <Typography variant="body1" color="text.secondary" className="numeral">
          {amount}
        </Typography>
      ) : null}
      <Button size="small" onClick={onAdd}>
        Add
      </Button>
      <IconButton size="small" aria-label={`Not ${requirement.name}`} onClick={onDismiss}>
        <CloseIcon fontSize="small" sx={{ color: 'text.disabled' }} />
      </IconButton>
    </Stack>
  );
}
