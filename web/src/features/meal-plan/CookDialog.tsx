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
import { ApiError, type PlannerMeal } from '../../api/client';
import { useRecordPreparation } from '../../api/queries';
import { FormDialog } from '../../components/FormDialog';

type Place = 'chilled' | 'frozen';

const PLACES: { value: Place; label: string; hint: string }[] = [
  { value: 'chilled', label: 'Fridge', hint: 'Eat within a few days' },
  { value: 'frozen', label: 'Freezer', hint: 'Keeps for months' },
];

function Stepper({
  label,
  value,
  onChange,
  min = 0,
  max,
}: {
  label: string;
  value: number;
  onChange: (next: number) => void;
  min?: number;
  max?: number;
}) {
  return (
    <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center' }}>
      <IconButton size="small" aria-label={`Fewer ${label}`} disabled={value <= min} onClick={() => onChange(value - 1)}>
        <RemoveIcon fontSize="small" />
      </IconButton>
      <Typography className="numeral" sx={{ minWidth: 22, textAlign: 'center', fontWeight: 600 }}>{value}</Typography>
      <IconButton size="small" aria-label={`More ${label}`} disabled={max != null && value >= max} onClick={() => onChange(value + 1)}>
        <AddIcon fontSize="small" />
      </IconButton>
    </Stack>
  );
}

export function CookDialog({
  meal,
  onClose,
}: {
  meal: PlannerMeal;
  onClose: () => void;
}) {
  const record = useRecordPreparation();
  const food = meal.foods.find((candidate) => candidate.item_kind === 'recipe' && candidate.needs_cooking);
  const recipe = food?.item_kind === 'recipe' ? food : null;
  const planned = Math.max(1, Math.round(recipe?.amount.value ?? 1));
  const eating = meal.people.length + meal.guest_groups.reduce((sum, group) => sum + group.count, 0);

  const [made, setMade] = useState(planned);
  const [fridge, setFridge] = useState(0);
  const [useBy, setUseBy] = useState('');
  const [error, setError] = useState<string | null>(null);

  if (!recipe) return null;

  const keeping = Math.max(0, made - eating);
  const inFridge = Math.min(fridge, keeping);
  const inFreezer = keeping - inFridge;
  const forNow = made - keeping;

  async function cook() {
    if (!recipe) return;
    setError(null);
    const wanted: { place: Place; servings: number; useBy: string }[] = [
      { place: 'chilled', servings: forNow, useBy: '' },
      { place: 'chilled', servings: inFridge, useBy },
      { place: 'frozen', servings: inFreezer, useBy },
    ];
    const merged = new Map<string, { place: Place; servings: number; useBy: string }>();
    for (const entry of wanted) {
      if (entry.servings <= 0) continue;
      const key = `${entry.place}|${entry.useBy}`;
      const existing = merged.get(key);
      if (existing) existing.servings += entry.servings;
      else merged.set(key, { ...entry });
    }
    const placements = [...merged.values()].map((entry) => ({
      storage_location: entry.place,
      servings: entry.servings,
      ...(entry.useBy ? { usability_deadline: { date: entry.useBy } } : {}),
    }));

    try {
      await record.mutateAsync({
        recipe_id: recipe.recipe_id,
        servings_produced: made,
        placements,
        meal_plan_entry_id: meal.id,
        meal_plan_component_id: recipe.id,
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
          {recipe.item_name}
        </Typography>
      </DialogTitle>
      <DialogContent dividers>
        <Stack spacing={2.5}>
          {error ? <Alert severity="error">{error}</Alert> : null}

          <Stack direction="row" spacing={2} sx={{ alignItems: 'center', bgcolor: 'action.hover', borderRadius: 2.5, px: 1.75, py: 1.5 }}>
            <Box sx={{ flex: 1, minWidth: 0 }}>
              <Typography variant="subtitle2">How many did it make?</Typography>
              <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
                {made === planned ? `You planned ${planned}` : `You planned ${planned}`}
              </Typography>
            </Box>
            <Stepper label="made" value={made} onChange={setMade} min={1} />
          </Stack>

          {keeping > 0 ? (
            <Box>
              <Typography variant="h3" sx={{ mb: 1 }}>
                {keeping === 1 ? 'Keeping 1 portion for later' : `Keeping ${keeping} portions for later`}
              </Typography>
              <Stack spacing={1}>
                {PLACES.map((place) => {
                  const count = place.value === 'chilled' ? inFridge : inFreezer;
                  return (
                    <Stack
                      key={place.value}
                      direction="row"
                      spacing={2}
                      sx={{
                        alignItems: 'center',
                        border: '1px solid',
                        borderColor: count > 0 ? 'primary.main' : 'divider',
                        bgcolor: count > 0 ? 'action.selected' : 'transparent',
                        borderRadius: 2.5,
                        px: 1.75,
                        py: 1.25,
                      }}
                    >
                      <Box sx={{ flex: 1, minWidth: 0 }}>
                        <Typography variant="subtitle2">{place.label}</Typography>
                        <Typography variant="caption" color="text.secondary">{place.hint}</Typography>
                      </Box>
                      <Stepper
                        label={place.label}
                        value={count}
                        max={keeping}
                        onChange={(next) => setFridge(place.value === 'chilled' ? next : keeping - next)}
                      />
                    </Stack>
                  );
                })}
              </Stack>
              <TextField
                label="Use by"
                type="date"
                value={useBy}
                onChange={(event) => setUseBy(event.target.value)}
                slotProps={{ inputLabel: { shrink: true } }}
                helperText="Optional"
                sx={{ mt: 1.5, width: { xs: '100%', sm: 200 } }}
              />
            </Box>
          ) : (
            <Typography variant="body2" color="text.secondary">
              Nothing left over. It all gets eaten now.
            </Typography>
          )}
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
