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
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import { ApiError, type Amount } from '../../api/client';
import { useRecipe, useRecordPreparation } from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';
import { ConceptIcon, type Concept } from '../../components/ConceptIcon';
import { displayUnit } from '../../components/UnitSelect';

type Place = 'serving' | 'fridge' | 'freezer';

const PLACES: { value: Place; label: string; hint: string; concept: Concept; location: 'ambient' | 'chilled' | 'frozen' }[] = [
  { value: 'serving', label: 'Serving now', hint: 'Stays out for the meal', concept: 'dish', location: 'ambient' },
  { value: 'fridge', label: 'Fridge', hint: 'Eat within a few days', concept: 'fridge', location: 'chilled' },
  { value: 'freezer', label: 'Freezer', hint: 'Keeps for months', concept: 'freezer', location: 'frozen' },
];

export type PlannedCook = {
  entryId: string;
  componentId: string;
  recipeId: string;
  name: string;
  planned: number;
};

export function scaledAmount(amount: Amount, factor: number): string {
  const value = Math.round(amount.value * factor * 10) / 10;
  if (amount.kind === 'measure') return `${value} ${displayUnit(amount.unit)}`;
  if (amount.kind === 'packs') return value === 1 ? '1 pack' : `${value} packs`;
  return value === 1 ? '1 serving' : `${value} servings`;
}

function ComingOutOfStock({ recipeId, made }: { recipeId: string; made: number }) {
  const recipe = useRecipe(recipeId);
  const yields = recipe.data?.servings ?? 0;
  const lines = recipe.data?.components ?? [];
  if (!recipe.data || yields <= 0 || lines.length === 0) return null;

  return (
    <Box sx={{ borderTop: '1px solid', borderColor: 'divider', pt: 2 }}>
      <Typography
        variant="caption"
        sx={{ fontWeight: 600, letterSpacing: '0.04em', textTransform: 'uppercase', color: 'text.secondary' }}
      >
        Coming out of stock
      </Typography>
      <Stack spacing={0.75} sx={{ mt: 1 }}>
        {lines.map((line) => (
          <Stack key={line.id} direction="row" spacing={2} sx={{ justifyContent: 'space-between' }}>
            <Typography variant="body2">{line.name}</Typography>
            <Typography variant="body2" color="text.secondary" className="numeral">
              {scaledAmount(line.amount, made / yields)}
            </Typography>
          </Stack>
        ))}
      </Stack>
    </Box>
  );
}

export function CookDialog({ cook: planned, onClose }: { cook: PlannedCook; onClose: () => void }) {
  const record = useRecordPreparation();
  const expected = Math.max(1, Math.round(planned.planned));

  const [made, setMade] = useState(expected);
  const [place, setPlace] = useState<Place>('serving');
  const [useBy, setUseBy] = useState('');
  const [error, setError] = useState<string | null>(null);

  const chosen = PLACES.find((candidate) => candidate.value === place)!;

  async function cook() {
    setError(null);
    try {
      await record.mutateAsync({
        recipe_id: planned.recipeId,
        servings_produced: made,
        placements: [{
          storage_location: chosen.location,
          servings: made,
          ...(useBy && place !== 'serving' ? { usability_deadline: { date: useBy } } : {}),
        }],
        meal_plan_entry_id: planned.entryId,
        meal_plan_component_id: planned.componentId,
      });
      onClose();
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not record this.');
    }
  }

  return (
    <FormDialog open onClose={record.isPending ? undefined : onClose} fullWidth maxWidth="sm">
      <DialogTitle sx={{ pb: 1 }}>
        <Typography component="span" variant="h2">Cooked it</Typography>
        <Typography component="span" variant="body2" color="text.secondary" sx={{ display: 'block', mt: 0.5 }}>
          {planned.name}
        </Typography>
      </DialogTitle>
      <DialogContent dividers>
        <Stack spacing={2.5}>
          {error ? <Alert severity="error">{error}</Alert> : null}

          <Stack direction="row" spacing={2} sx={{ alignItems: 'center', bgcolor: 'action.hover', borderRadius: 2.5, px: 1.75, py: 1.5 }}>
            <Box sx={{ flex: 1, minWidth: 0 }}>
              <Typography variant="subtitle2">How much did you make?</Typography>
              <Typography variant="caption" color="text.secondary" className="numeral" sx={{ display: 'block' }}>
                {`You planned ${expected}`}
              </Typography>
            </Box>
            <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center' }}>
              <IconButton size="small" aria-label="Fewer made" disabled={made <= 1} onClick={() => setMade(made - 1)}>
                <RemoveIcon fontSize="small" />
              </IconButton>
              <Typography className="numeral" sx={{ minWidth: 24, textAlign: 'center', fontWeight: 600 }}>{made}</Typography>
              <IconButton size="small" aria-label="More made" onClick={() => setMade(made + 1)}>
                <AddIcon fontSize="small" />
              </IconButton>
            </Stack>
          </Stack>

          <Box>
            <Typography variant="h3" sx={{ mb: 1 }}>Where is it going?</Typography>
            <Stack spacing={1}>
              {PLACES.map((candidate) => {
                const active = candidate.value === place;
                return (
                  <Stack
                    key={candidate.value}
                    component="button"
                    type="button"
                    aria-pressed={active}
                    onClick={() => setPlace(candidate.value)}
                    direction="row"
                    spacing={2}
                    sx={{
                      alignItems: 'center',
                      textAlign: 'left',
                      width: '100%',
                      cursor: 'pointer',
                      font: 'inherit',
                      border: '1px solid',
                      borderColor: active ? 'primary.main' : 'divider',
                      bgcolor: active ? 'action.selected' : 'transparent',
                      color: 'text.primary',
                      borderRadius: 2.5,
                      px: 1.75,
                      py: 1.25,
                    }}
                  >
                    <Box
                      sx={{
                        width: 32,
                        height: 32,
                        borderRadius: '10px',
                        display: 'grid',
                        placeItems: 'center',
                        flexShrink: 0,
                        bgcolor: active ? 'action.selected' : 'background.default',
                        color: active ? 'primary.main' : 'text.disabled',
                      }}
                    >
                      <ConceptIcon concept={candidate.concept} />
                    </Box>
                    <Box sx={{ flex: 1, minWidth: 0 }}>
                      <Typography variant="subtitle2">{candidate.label}</Typography>
                      <Typography variant="caption" color="text.secondary">{candidate.hint}</Typography>
                    </Box>
                  </Stack>
                );
              })}
            </Stack>
            {place !== 'serving' ? (
              <TextField
                label="Use by"
                type="date"
                value={useBy}
                onChange={(event) => setUseBy(event.target.value)}
                slotProps={{ inputLabel: { shrink: true } }}
                helperText="Optional"
                sx={{ mt: 1.5, width: { xs: '100%', sm: 200 } }}
              />
            ) : null}
          </Box>

          <ComingOutOfStock recipeId={planned.recipeId} made={made} />
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={record.isPending}>Cancel</Button>
        <Button variant="contained" onClick={() => void cook()} disabled={record.isPending}>
          {record.isPending ? 'Saving…' : 'Cooked it'}
        </Button>
      </DialogActions>
    </FormDialog>
  );
}
