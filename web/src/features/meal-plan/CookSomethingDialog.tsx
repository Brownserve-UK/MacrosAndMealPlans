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
import { ApiError, type RecipeSummary } from '../../api/client';
import { useRecordPreparation } from '../../api/queries';
import { ConceptIcon } from '../../components/ConceptIcon';
import { FormDialog } from '../../components/FormDialog';
import { RecipePicker } from './RecipePicker';

export function CookSomethingDialog({ onClose }: { onClose: () => void }) {
  const record = useRecordPreparation();
  const [recipe, setRecipe] = useState<RecipeSummary | null>(null);
  const [made, setMade] = useState(4);
  const [chilled, setChilled] = useState(0);
  const [useBy, setUseBy] = useState('');
  const [error, setError] = useState<string | null>(null);

  const inFridge = Math.min(chilled, made);
  const inFreezer = made - inFridge;

  async function cook() {
    if (!recipe) {
      setError('Pick what you cooked.');
      return;
    }
    setError(null);
    const placements = [
      ...(inFridge > 0
        ? [{
            storage_location: 'chilled' as const,
            servings: inFridge,
            ...(useBy ? { usability_deadline: { date: useBy } } : {}),
          }]
        : []),
      ...(inFreezer > 0 ? [{ storage_location: 'frozen' as const, servings: inFreezer }] : []),
    ];

    try {
      await record.mutateAsync({
        recipe_id: recipe.id,
        servings_produced: made,
        placements,
      });
      onClose();
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not record this.');
    }
  }

  return (
    <FormDialog open onClose={record.isPending ? undefined : onClose} fullWidth maxWidth="sm">
      <DialogTitle sx={{ pb: 1 }}>
        <Typography component="span" variant="h2">Cooked something</Typography>
        <Typography component="span" variant="body2" color="text.secondary" sx={{ display: 'block', mt: 0.5 }}>
          Not for a planned meal
        </Typography>
      </DialogTitle>
      <DialogContent dividers>
        <Stack spacing={2.5}>
          {error ? <Alert severity="error">{error}</Alert> : null}

          <RecipePicker value={recipe} onChange={setRecipe} autoFocus />

          <Stack direction="row" spacing={2} sx={{ alignItems: 'center', bgcolor: 'action.hover', borderRadius: 2.5, px: 1.75, py: 1.5 }}>
            <Box sx={{ flex: 1, minWidth: 0 }}>
              <Typography variant="subtitle2">How much did you make?</Typography>
              {recipe ? (
                <Typography variant="caption" color="text.secondary" className="numeral" sx={{ display: 'block' }}>
                  {`Recipe serves ${recipe.servings}`}
                </Typography>
              ) : null}
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
            <Stack direction="row" spacing={3} sx={{ flexWrap: 'wrap' }}>
              {(['chilled', 'frozen'] as const).map((where) => {
                const label = where === 'chilled' ? 'Fridge' : 'Freezer';
                const value = where === 'chilled' ? inFridge : inFreezer;
                return (
                  <Stack key={where} direction="row" spacing={0.75} sx={{ alignItems: 'center' }}>
                    <ConceptIcon concept={where === 'chilled' ? 'fridge' : 'freezer'} size={16} />
                    <Typography variant="body2">{label}</Typography>
                    <IconButton
                      size="small"
                      aria-label={`Less in the ${label.toLowerCase()}`}
                      disabled={value <= 0}
                      onClick={() => setChilled(where === 'chilled' ? inFridge - 1 : inFridge + 1)}
                    >
                      <RemoveIcon fontSize="small" />
                    </IconButton>
                    <Typography className="numeral" sx={{ minWidth: 22, textAlign: 'center', fontWeight: 600 }}>
                      {value}
                    </Typography>
                    <IconButton
                      size="small"
                      aria-label={`More in the ${label.toLowerCase()}`}
                      disabled={value >= made}
                      onClick={() => setChilled(where === 'chilled' ? inFridge + 1 : inFridge - 1)}
                    >
                      <AddIcon fontSize="small" />
                    </IconButton>
                  </Stack>
                );
              })}
            </Stack>
            {inFridge > 0 ? (
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
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={record.isPending}>Cancel</Button>
        <Button variant="contained" onClick={() => void cook()} disabled={record.isPending || !recipe}>
          {record.isPending ? 'Saving…' : 'Cooked it'}
        </Button>
      </DialogActions>
    </FormDialog>
  );
}
