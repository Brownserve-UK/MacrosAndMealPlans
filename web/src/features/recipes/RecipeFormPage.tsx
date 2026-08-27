import AddPhotoIcon from '@mui/icons-material/AddPhotoAlternateOutlined';
import ArrowDownIcon from '@mui/icons-material/ArrowDownwardOutlined';
import ArrowUpIcon from '@mui/icons-material/ArrowUpwardOutlined';
import DeleteIcon from '@mui/icons-material/DeleteOutlineOutlined';
import AddIcon from '@mui/icons-material/AddOutlined';
import Alert from '@mui/material/Alert';
import Autocomplete from '@mui/material/Autocomplete';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Divider from '@mui/material/Divider';
import Grid from '@mui/material/Grid';
import IconButton from '@mui/material/IconButton';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { Link, useBlocker, useNavigate } from '@tanstack/react-router';
import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent } from 'react';
import { ApiError, type Recipe } from '../../api/client';
import {
  useCreateRecipe,
  useDeleteRecipePhoto,
  useRecipe,
  useUpdateRecipe,
  useUploadRecipePhoto,
} from '../../api/queries';
import type { components } from '../../api/schema';
import { BackLabel } from '../../components/BackLink';
import { ConflictDialog } from '../../components/ConflictDialog';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import {
  RecipeComponentsEditor,
  linesAreValid,
  linesFromComponents,
  linesToComponents,
  newLine,
  type ComponentLine,
} from './RecipeComponentsEditor';
import { countryLabel, labelMealCategory } from './RecipesPage';
import { RecipeImage } from './RecipeImage';

type MealCategory = components['schemas']['MealCategory'];
type StepDraft = { key: string; id?: string; text: string };
type Draft = {
  name: string;
  description: string;
  servings: string;
  preparationMinutes: string;
  cookingMinutes: string;
  notes: string;
  mealCategories: MealCategory[];
  countries: string[];
  tags: string[];
};

const MEAL_CATEGORIES: MealCategory[] = ['breakfast', 'lunch', 'dinner', 'snack'];
const COUNTRY_CODES = 'AD AE AF AG AI AL AM AO AQ AR AS AT AU AW AX AZ BA BB BD BE BF BG BH BI BJ BL BM BN BO BQ BR BS BT BV BW BY BZ CA CC CD CF CG CH CI CK CL CM CN CO CR CU CV CW CX CY CZ DE DJ DK DM DO DZ EC EE EG EH ER ES ET FI FJ FK FM FO FR GA GB GD GE GF GG GH GI GL GM GN GP GQ GR GS GT GU GW GY HK HM HN HR HT HU ID IE IL IM IN IO IQ IR IS IT JE JM JO JP KE KG KH KI KM KN KP KR KW KY KZ LA LB LC LI LK LR LS LT LU LV LY MA MC MD ME MF MG MH MK ML MM MN MO MP MQ MR MS MT MU MV MW MX MY MZ NA NC NE NF NG NI NL NO NP NR NU NZ OM PA PE PF PG PH PK PL PM PN PR PS PT PW PY QA RE RO RS RU RW SA SB SC SD SE SG SH SI SJ SK SL SM SN SO SR SS ST SV SX SY SZ TC TD TF TG TH TJ TK TL TM TN TO TR TT TV TW TZ UA UG UM US UY UZ VA VC VE VG VI VN VU WF WS YE YT ZA ZM ZW'.split(' ');

export function NewRecipePage() {
  return <RecipeFormPage />;
}

export function EditRecipePage({ id }: { id: string }) {
  const query = useRecipe(id);
  if (query.isLoading) return <Loading label="Loading recipe" />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;
  return query.data ? <RecipeFormPage key={`${query.data.id}:${query.data.revision}`} recipe={query.data} /> : null;
}

function RecipeFormPage({ recipe }: { recipe?: Recipe }) {
  const navigate = useNavigate();
  const create = useCreateRecipe();
  const update = useUpdateRecipe();
  const upload = useUploadRecipePhoto();
  const deletePhoto = useDeleteRecipePhoto();
  const initialDraft = useMemo(() => draftFrom(recipe), [recipe]);
  const initialLines = useMemo(() => recipe ? linesFromComponents(recipe.components) : [newLine()], [recipe]);
  const initialSteps = useMemo(() => recipe ? recipe.instructions.map((step) => ({ key: step.id, id: step.id, text: step.text })) : [], [recipe]);
  const [draft, setDraft] = useState(initialDraft);
  const [lines, setLines] = useState<ComponentLine[]>(initialLines);
  const [steps, setSteps] = useState<StepDraft[]>(initialSteps);
  const [photo, setPhoto] = useState<File | null>(null);
  const [removePhoto, setRemovePhoto] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [failure, setFailure] = useState<string | null>(null);
  const [partial, setPartial] = useState<string | null>(null);
  const [conflict, setConflict] = useState<ApiError | null>(null);
  const [savedId, setSavedId] = useState(recipe?.id ?? null);
  const [baseRevision, setBaseRevision] = useState(recipe?.revision ?? null);
  const allowNavigation = useRef(false);
  const initialState = JSON.stringify({ draft: initialDraft, lines: serialiseLines(initialLines), steps: initialSteps });
  const currentState = JSON.stringify({ draft, lines: serialiseLines(lines), steps });
  const dirty = initialState !== currentState || photo !== null || removePhoto;
  const shouldBlock = useCallback(() => dirty && !allowNavigation.current, [dirty]);
  const blocker = useBlocker({ shouldBlockFn: shouldBlock, enableBeforeUnload: dirty, withResolver: true });
  const photoUrl = useMemo(() => photo ? URL.createObjectURL(photo) : null, [photo]);

  useEffect(() => {
    return () => {
      if (photoUrl) URL.revokeObjectURL(photoUrl);
    };
  }, [photoUrl]);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFailure(null);
    setPartial(null);
    const found = validate(draft, lines, steps);
    setErrors(found);
    if (Object.keys(found).length > 0) return;
    const components = linesToComponents(lines);
    if (!components) return;
    const body = requestFrom(draft, components, steps);

    try {
      let saved: Recipe;
      if (savedId && baseRevision != null) {
        saved = await update.mutateAsync({ id: savedId, revision: baseRevision, body });
      } else {
        saved = await create.mutateAsync(body);
        setSavedId(saved.id);
      }
      setBaseRevision(saved.revision);

      try {
        if (photo) {
          saved = await upload.mutateAsync({ id: saved.id, revision: saved.revision, file: photo });
          setBaseRevision(saved.revision);
        } else if (removePhoto && saved.photo_version) {
          saved = await deletePhoto.mutateAsync({ id: saved.id, revision: saved.revision });
          setBaseRevision(saved.revision);
        }
      } catch (caught) {
        if (caught instanceof ApiError && caught.isConflict) setConflict(caught);
        else setPartial('Recipe saved, but the photo was not changed. Your selected photo is ready to retry.');
        return;
      }

      allowNavigation.current = true;
      void navigate({ to: '/recipes/$id', params: { id: saved.id } });
    } catch (caught) {
      if (caught instanceof ApiError) {
        if (caught.isConflict) setConflict(caught);
        else if (Object.keys(caught.fieldErrors).length > 0) setErrors(caught.fieldErrors);
        else setFailure(caught.message);
      } else {
        setFailure('Could not save the recipe.');
      }
    }
  }

  function choosePhoto(file: File | null) {
    setFailure(null);
    if (!file) return;
    if (!['image/jpeg', 'image/png', 'image/webp'].includes(file.type)) {
      setFailure('Choose a JPEG, PNG, or WebP image.');
      return;
    }
    if (file.size > 20 * 1024 * 1024) {
      setFailure('Choose an image up to 20 MB.');
      return;
    }
    setPhoto(file);
    setRemovePhoto(false);
  }

  const pending = create.isPending || update.isPending || upload.isPending || deletePhoto.isPending;
  const backId = savedId ?? recipe?.id;

  return (
    <>
      <PageHeader
        back={
          <Link to={backId ? '/recipes/$id' : '/recipes'} params={backId ? { id: backId } : {}} className="app-link">
            <BackLabel>{backId ? 'Recipe' : 'Recipes'}</BackLabel>
          </Link>
        }
        title={recipe ? `Edit ${recipe.name}` : 'New recipe'}
        subtitle="Build the recipe in a few clear sections."
      />
      <Box component="form" onSubmit={onSubmit}>
        <Stack spacing={3}>
          {failure ? <Alert severity="error">{failure}</Alert> : null}
          {partial ? <Alert severity="warning">{partial}</Alert> : null}

          <Section title="Basics">
            <TextField label="Name" value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} error={Boolean(errors.name)} helperText={errors.name} autoFocus fullWidth />
            <TextField label="Short description" value={draft.description} onChange={(event) => setDraft({ ...draft, description: event.target.value })} error={Boolean(errors.description)} helperText={errors.description} multiline minRows={2} fullWidth />
            <Grid container spacing={2}>
              <Grid size={{ xs: 12, sm: 4 }}><TextField type="number" label="Serves" value={draft.servings} onChange={(event) => setDraft({ ...draft, servings: event.target.value })} error={Boolean(errors.servings)} helperText={errors.servings} slotProps={{ htmlInput: { min: 1, step: 1 } }} fullWidth /></Grid>
              <Grid size={{ xs: 12, sm: 4 }}><TextField type="number" label="Preparation time (min)" value={draft.preparationMinutes} onChange={(event) => setDraft({ ...draft, preparationMinutes: event.target.value })} error={Boolean(errors.preparation_minutes)} helperText={errors.preparation_minutes} slotProps={{ htmlInput: { min: 1, step: 1 } }} fullWidth /></Grid>
              <Grid size={{ xs: 12, sm: 4 }}><TextField type="number" label="Cooking time (min)" value={draft.cookingMinutes} onChange={(event) => setDraft({ ...draft, cookingMinutes: event.target.value })} error={Boolean(errors.cooking_minutes)} helperText={errors.cooking_minutes} slotProps={{ htmlInput: { min: 1, step: 1 } }} fullWidth /></Grid>
            </Grid>
          </Section>

          <Section title="Photo">
            <Box sx={{ width: '100%', maxWidth: 560, aspectRatio: '16 / 9', borderRadius: 2, overflow: 'hidden', bgcolor: 'action.hover', display: 'grid', placeItems: 'center' }}>
              {photoUrl ? <Box component="img" src={photoUrl} alt="Selected recipe" sx={{ width: '100%', height: '100%', objectFit: 'cover' }} /> : recipe?.photo_version && !removePhoto ? <RecipeImage id={recipe.id} version={recipe.photo_version} size="card" alt={recipe.name} sx={{ width: '100%', height: '100%', objectFit: 'cover' }} /> : <Typography color="text.secondary">No photo</Typography>}
            </Box>
            <Stack direction="row" spacing={1} useFlexGap sx={{ flexWrap: 'wrap' }}>
              <Button component="label" variant="outlined" startIcon={<AddPhotoIcon />}>
                {recipe?.photo_version || photo ? 'Replace photo' : 'Choose photo'}
                <input hidden type="file" accept="image/jpeg,image/png,image/webp" onChange={(event) => choosePhoto(event.target.files?.[0] ?? null)} />
              </Button>
              {photo || (recipe?.photo_version && !removePhoto) ? <Button color="error" onClick={() => { setPhoto(null); setRemovePhoto(Boolean(recipe?.photo_version)); }}>Remove photo</Button> : null}
            </Stack>
            <Typography variant="caption" color="text.secondary">JPEG, PNG, or WebP. Up to 20 MB.</Typography>
          </Section>

          <Section title="Categories and tags">
            <Box>
              <Typography variant="body2" sx={{ mb: 1 }}>Meal</Typography>
              <Stack direction="row" spacing={1} useFlexGap sx={{ flexWrap: 'wrap' }}>
                {MEAL_CATEGORIES.map((category) => <Chip key={category} label={labelMealCategory(category)} color={draft.mealCategories.includes(category) ? 'primary' : 'default'} variant={draft.mealCategories.includes(category) ? 'filled' : 'outlined'} onClick={() => setDraft({ ...draft, mealCategories: toggle(draft.mealCategories, category) })} />)}
              </Stack>
            </Box>
            <Autocomplete multiple options={COUNTRY_CODES} value={draft.countries} getOptionLabel={countryLabel} onChange={(_, countries) => setDraft({ ...draft, countries })} renderInput={(params) => <TextField {...params} label="Countries" error={Boolean(errors.country_categories)} helperText={errors.country_categories} />} />
            <Autocomplete multiple freeSolo options={[]} value={draft.tags} onChange={(_, tags) => setDraft({ ...draft, tags })} renderInput={(params) => <TextField {...params} label="Tags" helperText="Type a tag and press Enter." error={Boolean(errors.tags)} />} />
          </Section>

          <Section title="Products">
            <RecipeComponentsEditor lines={lines} onChange={setLines} />
            {errors.components ? <Alert severity="error">{errors.components}</Alert> : null}
          </Section>

          <Section title="Instructions">
            <Stack spacing={2} divider={<Divider flexItem />}>
              {steps.map((step, index) => <Stack key={step.key} direction="row" spacing={1} sx={{ alignItems: 'flex-start' }}><Box sx={{ width: 30, height: 30, borderRadius: '50%', bgcolor: 'primary.main', color: 'primary.contrastText', display: 'grid', placeItems: 'center', flexShrink: 0, mt: 1 }}>{index + 1}</Box><TextField label={`Step ${index + 1}`} value={step.text} onChange={(event) => setSteps(steps.map((item) => item.key === step.key ? { ...item, text: event.target.value } : item))} error={Boolean(errors[`instructions.${index}.text`])} helperText={errors[`instructions.${index}.text`]} multiline minRows={2} fullWidth /><Stack><IconButton aria-label={`Move step ${index + 1} up`} disabled={index === 0} onClick={() => setSteps(move(steps, index, index - 1))}><ArrowUpIcon /></IconButton><IconButton aria-label={`Move step ${index + 1} down`} disabled={index === steps.length - 1} onClick={() => setSteps(move(steps, index, index + 1))}><ArrowDownIcon /></IconButton><IconButton aria-label={`Remove step ${index + 1}`} onClick={() => setSteps(steps.filter((item) => item.key !== step.key))}><DeleteIcon /></IconButton></Stack></Stack>)}
              <Button startIcon={<AddIcon />} onClick={() => setSteps([...steps, { key: crypto.randomUUID(), text: '' }])} sx={{ alignSelf: 'flex-start' }}>Add step</Button>
            </Stack>
          </Section>

          <Section title="Notes">
            <TextField label="Notes" value={draft.notes} onChange={(event) => setDraft({ ...draft, notes: event.target.value })} error={Boolean(errors.notes)} helperText={errors.notes} multiline minRows={4} fullWidth />
          </Section>

          <Stack direction="row" spacing={1.5} sx={{ justifyContent: 'flex-end' }}>
            {backId ? <Link to="/recipes/$id" params={{ id: backId }}><Button>Cancel</Button></Link> : <Link to="/recipes"><Button>Cancel</Button></Link>}
            <Button type="submit" variant="contained" disabled={pending}>{pending ? 'Saving…' : 'Save recipe'}</Button>
          </Stack>
        </Stack>
      </Box>

      <Dialog open={blocker.status === 'blocked'} onClose={() => blocker.status === 'blocked' && blocker.reset()}>
        <DialogTitle>Leave without saving?</DialogTitle>
        <DialogContent>Your changes will be lost.</DialogContent>
        <DialogActions><Button onClick={() => blocker.status === 'blocked' && blocker.reset()}>Stay</Button><Button color="error" onClick={() => blocker.status === 'blocked' && blocker.proceed()}>Leave</Button></DialogActions>
      </Dialog>
      <ConflictDialog error={conflict} onDismiss={() => setConflict(null)} onReload={() => window.location.reload()} />
    </>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return <Paper sx={{ p: { xs: 2.5, md: 3.5 } }}><Typography variant="h2" sx={{ mb: 2.5 }}>{title}</Typography><Stack spacing={2.5}>{children}</Stack></Paper>;
}

function draftFrom(recipe?: Recipe): Draft {
  return { name: recipe?.name ?? '', description: recipe?.description ?? '', servings: String(recipe?.servings ?? 4), preparationMinutes: recipe?.preparation_minutes ? String(recipe.preparation_minutes) : '', cookingMinutes: recipe?.cooking_minutes ? String(recipe.cooking_minutes) : '', notes: recipe?.notes ?? '', mealCategories: recipe?.meal_categories ?? [], countries: recipe?.country_categories ?? [], tags: recipe?.tags ?? [] };
}

function serialiseLines(lines: ComponentLine[]) {
  return lines.map(({ id, productId, amount }) => ({ id, productId, amount }));
}

function validate(draft: Draft, lines: ComponentLine[], steps: StepDraft[]) {
  const errors: Record<string, string> = {};
  if (!draft.name.trim()) errors.name = 'Give it a name';
  if (draft.description.length > 2000) errors.description = 'Keep this under 2,000 characters';
  const servings = Number(draft.servings);
  if (!Number.isInteger(servings) || servings <= 0) errors.servings = 'Enter a whole number above zero';
  for (const [field, value] of [['preparation_minutes', draft.preparationMinutes], ['cooking_minutes', draft.cookingMinutes]] as const) {
    if (value && (!Number.isInteger(Number(value)) || Number(value) <= 0)) errors[field] = 'Enter whole minutes above zero';
  }
  if (!linesAreValid(lines)) errors.components = 'Add at least one product, each with an amount.';
  steps.forEach((step, index) => { if (!step.text.trim()) errors[`instructions.${index}.text`] = 'Add the instruction'; });
  if (draft.notes.length > 20000) errors.notes = 'Keep this under 20,000 characters';
  if (draft.tags.some((tag) => !tag.trim())) errors.tags = 'Remove empty tags';
  return errors;
}

function requestFrom(draft: Draft, components: NonNullable<ReturnType<typeof linesToComponents>>, steps: StepDraft[]): components['schemas']['CreateRecipeRequest'] {
  return { name: draft.name.trim(), description: draft.description.trim() || null, servings: Number(draft.servings), preparation_minutes: draft.preparationMinutes ? Number(draft.preparationMinutes) : null, cooking_minutes: draft.cookingMinutes ? Number(draft.cookingMinutes) : null, notes: draft.notes.trim() || null, components, instructions: steps.map((step) => ({ ...(step.id ? { id: step.id } : {}), text: step.text.trim() })), meal_categories: draft.mealCategories, country_categories: draft.countries, tags: draft.tags.map((tag) => tag.trim()).filter(Boolean) };
}

function toggle<T>(values: T[], value: T) {
  return values.includes(value) ? values.filter((item) => item !== value) : [...values, value];
}

function move<T>(values: T[], from: number, to: number) {
  const next = [...values];
  const [value] = next.splice(from, 1);
  if (value === undefined) return values;
  next.splice(to, 0, value);
  return next;
}
