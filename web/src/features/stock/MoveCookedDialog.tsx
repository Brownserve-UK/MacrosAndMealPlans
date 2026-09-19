import AddIcon from '@mui/icons-material/AddOutlined';
import RemoveIcon from '@mui/icons-material/RemoveOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import IconButton from '@mui/material/IconButton';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import { ApiError, type StockItem } from '../../api/client';
import { useMoveCookedFood } from '../../api/queries';
import { ConceptIcon } from '../../components/ConceptIcon';
import { FormDialog } from '../../components/FormDialog';

type Place = StockItem['storage_location'];

const PLACE_LABEL: Record<Place, string> = {
  ambient: 'the cupboard',
  chilled: 'the fridge',
  frozen: 'the freezer',
};

export function MoveCookedDialog({
  recipeId,
  name,
  from,
  to,
  available,
  onClose,
}: {
  recipeId: string;
  name: string;
  from: Place;
  to: Place;
  available: number;
  onClose: () => void;
}) {
  const move = useMoveCookedFood();
  const [servings, setServings] = useState(Math.min(1, available));
  const [error, setError] = useState<string | null>(null);

  async function submit() {
    setError(null);
    try {
      await move.mutateAsync({ recipeId, body: { from, to, servings } });
      onClose();
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not move this.');
    }
  }

  return (
    <FormDialog open onClose={move.isPending ? undefined : onClose} fullWidth maxWidth="xs">
      <DialogTitle sx={{ pb: 1 }}>
        <Typography component="span" variant="h2">{`Move ${name}`}</Typography>
        <Typography
          component="span"
          variant="body2"
          color="text.secondary"
          sx={{ display: 'block', mt: 0.5 }}
        >
          {`From ${PLACE_LABEL[from]} to ${PLACE_LABEL[to]}`}
        </Typography>
      </DialogTitle>
      <DialogContent dividers>
        <Stack spacing={2.5}>
          {error ? <Alert severity="error">{error}</Alert> : null}

          <Stack
            direction="row"
            spacing={2}
            sx={{
              alignItems: 'center',
              bgcolor: 'action.hover',
              borderRadius: 2.5,
              px: 1.75,
              py: 1.5,
            }}
          >
            <ConceptIcon concept={to === 'frozen' ? 'freezer' : 'fridge'} size={20} />
            <Box sx={{ flex: 1, minWidth: 0 }}>
              <Typography variant="subtitle2">How many servings?</Typography>
              <Typography
                variant="caption"
                color="text.secondary"
                className="numeral"
                sx={{ display: 'block' }}
              >
                {available === 1 ? '1 there' : `${available} there`}
              </Typography>
            </Box>
            <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center' }}>
              <IconButton
                size="small"
                aria-label="Fewer servings"
                disabled={servings <= 1}
                onClick={() => setServings(servings - 1)}
              >
                <RemoveIcon fontSize="small" />
              </IconButton>
              <Typography
                className="numeral"
                sx={{ minWidth: 24, textAlign: 'center', fontWeight: 600 }}
              >
                {servings}
              </Typography>
              <IconButton
                size="small"
                aria-label="More servings"
                disabled={servings >= available}
                onClick={() => setServings(servings + 1)}
              >
                <AddIcon fontSize="small" />
              </IconButton>
            </Stack>
          </Stack>

          <Typography variant="body2" color="text.secondary">
            {`It gets a fresh use-by date for ${PLACE_LABEL[to]}, and the oldest cook moves first.`}
          </Typography>
        </Stack>
      </DialogContent>
      <DialogActions sx={{ px: 3, py: 2 }}>
        <Button onClick={onClose} disabled={move.isPending}>Cancel</Button>
        <Button variant="contained" onClick={() => void submit()} disabled={move.isPending}>
          {move.isPending ? 'Moving…' : 'Move'}
        </Button>
      </DialogActions>
    </FormDialog>
  );
}
