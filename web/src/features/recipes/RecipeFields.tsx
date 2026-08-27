import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';

export type RecipeDraft = {
  name: string;
  servings: string;
};

export function RecipeFields({
  draft,
  errors,
  onChange,
  autoFocus,
}: {
  draft: RecipeDraft;
  errors: Record<string, string>;
  onChange: (next: RecipeDraft) => void;
  autoFocus?: boolean;
}) {
  return (
    <Stack spacing={3}>
      <TextField
        label="Name"
        value={draft.name}
        onChange={(event) => onChange({ ...draft, name: event.target.value })}
        error={Boolean(errors.name)}
        helperText={errors.name}
        autoFocus={autoFocus}
        fullWidth
      />
      <TextField
        type="number"
        label="Serves"
        value={draft.servings}
        onChange={(event) => onChange({ ...draft, servings: event.target.value })}
        error={Boolean(errors.servings)}
        helperText={errors.servings ?? 'How many servings the recipe makes.'}
        slotProps={{ htmlInput: { min: 1, step: 1 } }}
        sx={{ maxWidth: 160 }}
      />
    </Stack>
  );
}
