import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import MenuItem from '@mui/material/MenuItem';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import { ApiError, type Ingredient, type Product, type Unit } from '../../api/client';
import { useCreateIngredient, useResolveRecipeComponent } from '../../api/queries';
import { IngredientPicker } from '../products/IngredientPicker';
import { ProductPicker } from '../meal-plan/ProductPicker';

type Mode = 'ingredient' | 'product' | 'create';

export function ResolveComponentDialog({
  recipeId,
  revision,
  componentId,
  text,
  onClose,
  onConflict,
}: {
  recipeId: string;
  revision: number;
  componentId: string;
  text: string;
  onClose: () => void;
  onConflict: (error: ApiError) => void;
}) {
  const resolve = useResolveRecipeComponent();
  const createIngredient = useCreateIngredient();
  const [mode, setMode] = useState<Mode>('ingredient');
  const [ingredient, setIngredient] = useState<Ingredient | null>(null);
  const [product, setProduct] = useState<Product | null>(null);
  const [newName, setNewName] = useState(text);
  const [failure, setFailure] = useState<string | null>(null);

  async function submit() {
    setFailure(null);
    try {
      if (mode === 'ingredient' && ingredient) {
        await resolve.mutateAsync({
          id: recipeId,
          componentId,
          revision,
          body: { kind: 'ingredient', ingredient_id: ingredient.id },
        });
      } else if (mode === 'product' && product) {
        await resolve.mutateAsync({
          id: recipeId,
          componentId,
          revision,
          body: { kind: 'product', product_id: product.id },
        });
      } else if (mode === 'create' && newName.trim()) {
        const created = await createIngredient.mutateAsync({
          name: newName.trim(),
          default_unit: 'g' as Unit,
        });
        await resolve.mutateAsync({
          id: recipeId,
          componentId,
          revision,
          body: { kind: 'ingredient', ingredient_id: created.id },
        });
      } else {
        return;
      }
      onClose();
    } catch (caught) {
      if (caught instanceof ApiError) {
        if (caught.isConflict) {
          onConflict(caught);
          onClose();
          return;
        }
        setFailure(caught.message);
      } else {
        setFailure('Could not match this line.');
      }
    }
  }

  const canSubmit =
    (mode === 'ingredient' && ingredient != null) ||
    (mode === 'product' && product != null) ||
    (mode === 'create' && newName.trim().length > 0);

  return (
    <Dialog open onClose={onClose} fullWidth maxWidth="sm">
      <DialogTitle>Match "{text}"</DialogTitle>
      <DialogContent>
        <Stack spacing={2} sx={{ mt: 1 }}>
          {failure ? <Alert severity="error">{failure}</Alert> : null}
          <TextField
            select
            label="Match to"
            value={mode}
            onChange={(event) => setMode(event.target.value as Mode)}
          >
            <MenuItem value="ingredient">An existing ingredient</MenuItem>
            <MenuItem value="product">A specific product</MenuItem>
            <MenuItem value="create">A new ingredient</MenuItem>
          </TextField>

          {mode === 'ingredient' ? (
            <IngredientPicker value={ingredient} onChange={setIngredient} helperText="" />
          ) : mode === 'product' ? (
            <ProductPicker value={product} onChange={setProduct} excludeIds={[]} />
          ) : (
            <>
              <TextField
                label="Name"
                value={newName}
                onChange={(event) => setNewName(event.target.value)}
                fullWidth
              />
              <Typography variant="caption" color="text.secondary">
                Added to the shared catalogue.
              </Typography>
            </>
          )}
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Not now</Button>
        <Button
          variant="contained"
          disabled={!canSubmit || resolve.isPending || createIngredient.isPending}
          onClick={submit}
        >
          Match
        </Button>
      </DialogActions>
    </Dialog>
  );
}
