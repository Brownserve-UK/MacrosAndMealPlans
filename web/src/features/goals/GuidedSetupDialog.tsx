import ArrowBackIcon from '@mui/icons-material/ArrowBackIosNewOutlined';
import BalanceIcon from '@mui/icons-material/BalanceOutlined';
import CloseIcon from '@mui/icons-material/CloseOutlined';
import DirectionsRunIcon from '@mui/icons-material/DirectionsRunOutlined';
import FitnessCenterIcon from '@mui/icons-material/FitnessCenterOutlined';
import KeyboardArrowDownIcon from '@mui/icons-material/KeyboardArrowDownOutlined';
import KeyboardArrowUpIcon from '@mui/icons-material/KeyboardArrowUpOutlined';
import TrendingDownIcon from '@mui/icons-material/TrendingDownOutlined';
import TrendingUpIcon from '@mui/icons-material/TrendingUpOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import ButtonBase from '@mui/material/ButtonBase';
import Collapse from '@mui/material/Collapse';
import Dialog from '@mui/material/Dialog';
import IconButton from '@mui/material/IconButton';
import MenuItem from '@mui/material/MenuItem';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useEffect, useMemo, useRef, useState, type FormEvent, type ReactNode } from 'react';
import type { BodyProfile, HabitualActivity, NutritionEmphasis, NutritionPlanRecommendation, Pace, WeightObjective, WeightSummary } from '../../api/client';
import { ApiError } from '../../api/client';
import { usePreviewCalorieTarget, useSetGuidedCalorieTarget } from '../../api/queries';
import { ConflictDialog } from '../../components/ConflictDialog';
import { paceLabel, paceOptionsFor, pacePoundsLabel } from './pace';

type Step = 'goal' | 'emphasis' | 'about' | 'activity' | 'goalWeight' | 'targets';
type Draft = { objective: WeightObjective; emphasis: NutritionEmphasis; dateOfBirth: string; sex: 'male' | 'female' | ''; heightCm: string; currentWeight: string; habitualActivity: HabitualActivity | ''; targetWeight: string; pace: Pace };

const GOALS: { value: WeightObjective; title: string; caption: string; icon: ReactNode }[] = [
  { value: 'lose', title: 'Lose weight', caption: 'Work towards a lower weight', icon: <TrendingDownIcon /> },
  { value: 'maintain', title: 'Maintain weight', caption: 'Keep your weight around where it is', icon: <BalanceIcon /> },
  { value: 'gain', title: 'Gain weight', caption: 'Work towards a higher weight', icon: <TrendingUpIcon /> },
];
const EMPHASES: { value: NutritionEmphasis; title: string; caption: string; icon: ReactNode }[] = [
  { value: 'general', title: 'General & balanced', caption: 'A well-rounded everyday split', icon: <BalanceIcon /> },
  { value: 'muscle', title: 'Build muscle', caption: 'More protein to support training', icon: <FitnessCenterIcon /> },
  { value: 'endurance', title: 'Endurance', caption: 'More carbs to fuel training', icon: <DirectionsRunIcon /> },
];
const ACTIVITIES: { value: HabitualActivity; title: string; caption: string }[] = [
  { value: 'mostly_sedentary', title: 'Mostly sedentary', caption: 'Desk job, driving, most of the day sitting' },
  { value: 'lightly_active', title: 'Lightly active', caption: 'Teaching, shop work, some daily walking' },
  { value: 'active', title: 'Active', caption: 'Nursing, trade work, on your feet most of the day' },
  { value: 'very_active', title: 'Very active', caption: 'Labouring, farming, warehouse or delivery work' },
];

function initialDraft(profile: BodyProfile | null | undefined, summary: WeightSummary, emphasis?: NutritionEmphasis): Draft {
  return { objective: summary.goal?.objective ?? 'lose', emphasis: emphasis ?? 'general', dateOfBirth: profile?.date_of_birth ?? '', sex: profile?.sex ?? '', heightCm: profile?.height_cm == null ? '' : String(profile.height_cm), currentWeight: summary.latest?.weight_kg == null ? '' : String(summary.latest.weight_kg), habitualActivity: profile?.habitual_activity ?? '', targetWeight: summary.goal?.target_weight_kg == null ? '' : String(summary.goal.target_weight_kg), pace: 'standard' };
}

function requestBody(draft: Draft) {
  return { date_of_birth: draft.dateOfBirth, sex: draft.sex as 'male' | 'female', height_cm: Number(draft.heightCm), current_weight: { amount: Number(draft.currentWeight), unit: 'kg' as const }, habitual_activity: draft.habitualActivity as HabitualActivity, objective: draft.objective, emphasis: draft.emphasis, target_weight: draft.objective === 'maintain' ? null : { amount: Number(draft.targetWeight), unit: 'kg' as const }, pace: draft.objective === 'maintain' ? null : draft.pace };
}

function formatDate(iso: string) {
  return new Date(`${iso}T00:00:00`).toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' });
}

function OptionCard({ title, caption, icon, selected, onClick }: { title: string; caption: string; icon?: ReactNode; selected?: boolean; onClick: () => void }) {
  return <ButtonBase onClick={onClick} sx={{ display: 'block', width: '100%', textAlign: 'left', borderRadius: '14px' }}><Paper elevation={0} sx={{ width: '100%', p: 2, borderColor: selected ? 'primary.main' : 'divider' }}><Stack direction="row" spacing={2} sx={{ alignItems: 'center' }}>{icon ? <Box sx={{ width: 34, height: 34, borderRadius: '10px', bgcolor: 'action.hover', display: 'grid', placeItems: 'center', color: 'text.secondary' }}>{icon}</Box> : null}<Stack spacing={0.5}><Typography variant="body1" sx={{ fontWeight: 500 }}>{title}</Typography><Typography variant="caption" color="text.secondary">{caption}</Typography></Stack></Stack></Paper></ButtonBase>;
}

function Stat({ label, value }: { label: string; value: number }) {
  return <Paper elevation={0} sx={{ flex: 1, p: 2, textAlign: 'center' }}><Typography className="numeral" variant="h3">{value.toLocaleString('en-GB')} g</Typography><Typography variant="caption" color="text.secondary">{label}</Typography></Paper>;
}

export function GuidedSetupDialog({ memberId, profile, summary, emphasis, onClose, onManual }: { memberId: string; profile?: BodyProfile | null; summary: WeightSummary; emphasis?: NutritionEmphasis; onClose: () => void; onManual: () => void }) {
  const preview = usePreviewCalorieTarget();
  const save = useSetGuidedCalorieTarget();
  const [step, setStep] = useState<Step>('goal');
  const [direction, setDirection] = useState<'forward' | 'back'>('forward');
  const [transitioning, setTransitioning] = useState(false);
  const transitionTimer = useRef<number | null>(null);
  const [draft, setDraft] = useState(() => initialDraft(profile, summary, emphasis));
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [recommendation, setRecommendation] = useState<NutritionPlanRecommendation | null>(null);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [conflict, setConflict] = useState<ApiError | null>(null);
  const steps = useMemo<Step[]>(() => draft.objective === 'maintain' ? ['goal', 'emphasis', 'about', 'activity', 'targets'] : ['goal', 'emphasis', 'about', 'activity', 'goalWeight', 'targets'], [draft.objective]);
  const stepIndex = steps.indexOf(step);
  useEffect(() => () => { if (transitionTimer.current != null) window.clearTimeout(transitionTimer.current); }, []);

  function set<K extends keyof Draft>(key: K, value: Draft[K]) { setDraft((previous) => ({ ...previous, [key]: value })); }
  function go(next: Step, nextDirection: 'forward' | 'back' = 'forward') {
    setDirection(nextDirection); setErrors({}); setFormError(null); setTransitioning(true);
    transitionTimer.current = window.setTimeout(() => { setStep(next); setTransitioning(false); }, 150);
  }
  async function showTargets(nextDraft = draft) {
    try { setRecommendation(await preview.mutateAsync({ id: memberId, body: requestBody(nextDraft) })); go('targets'); }
    catch (caught) { setFormError(caught instanceof ApiError ? (Object.values(caught.fieldErrors)[0] ?? caught.message) : 'Something went wrong.'); }
  }
  function validateAbout(event: FormEvent) {
    event.preventDefault();
    const next: Record<string, string> = {};
    if (!draft.dateOfBirth) next.dateOfBirth = 'Enter your date of birth';
    if (!draft.sex) next.sex = 'Choose an option';
    if (!Number.isFinite(Number(draft.heightCm)) || Number(draft.heightCm) <= 0) next.heightCm = 'Enter your height';
    if (!Number.isFinite(Number(draft.currentWeight)) || Number(draft.currentWeight) <= 0) next.currentWeight = 'Enter your weight';
    setErrors(next); if (Object.keys(next).length === 0) go('activity');
  }
  function submitGoal(event: FormEvent) {
    event.preventDefault();
    const current = Number(draft.currentWeight); const target = Number(draft.targetWeight); const next: Record<string, string> = {};
    if (!Number.isFinite(target) || target <= 0) next.targetWeight = 'Enter a goal weight';
    else if (draft.objective === 'lose' && target >= current) next.targetWeight = 'Choose a lower weight than your current weight';
    else if (draft.objective === 'gain' && target <= current) next.targetWeight = 'Choose a higher weight than your current weight';
    setErrors(next); if (Object.keys(next).length === 0) void showTargets();
  }
  async function saveTargets() {
    try { await save.mutateAsync({ id: memberId, body: requestBody(draft) }); onClose(); }
    catch (caught) { if (caught instanceof ApiError && caught.isConflict) setConflict(caught); else setFormError(caught instanceof ApiError ? (Object.values(caught.fieldErrors)[0] ?? caught.message) : 'Something went wrong.'); }
  }

  return <Dialog open onClose={onClose} aria-labelledby="guided-title" fullWidth maxWidth="sm" slotProps={{ paper: { sx: { minHeight: { xs: '80vh', sm: 600 } } } }}><Stack sx={{ height: '100%' }}>
    <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between', p: 2 }}><IconButton aria-label="Back" onClick={() => stepIndex > 0 && go(steps[stepIndex - 1] as Step, 'back')} sx={{ visibility: stepIndex === 0 ? 'hidden' : 'visible' }}><ArrowBackIcon /></IconButton><Stack direction="row" spacing={1} aria-label="Progress">{steps.map((item, index) => <Box key={item} sx={{ width: index === stepIndex ? 10 : 8, height: index === stepIndex ? 10 : 8, borderRadius: '50%', bgcolor: index < stepIndex ? 'primary.dark' : index === stepIndex ? 'primary.main' : 'action.disabledBackground', transition: 'width 150ms, height 150ms, background-color 150ms' }} />)}</Stack><IconButton aria-label="Close" onClick={onClose}><CloseIcon /></IconButton></Stack>
    <Box sx={{ flex: 1, display: 'grid', placeItems: 'center', px: 3, py: { xs: 2, sm: 3 }, overflow: 'hidden' }}><Stack key={step} spacing={3} sx={{ width: '100%', maxWidth: 480, pointerEvents: transitioning ? 'none' : 'auto', animation: `${transitioning ? direction === 'forward' ? 'stepExitForward' : 'stepExitBack' : direction === 'forward' ? 'stepEnterForward' : 'stepEnterBack'} 150ms ease-in-out`, '@keyframes stepEnterForward': { from: { opacity: 0, transform: 'translateX(32px)' }, to: { opacity: 1, transform: 'translateX(0)' } }, '@keyframes stepEnterBack': { from: { opacity: 0, transform: 'translateX(-32px)' }, to: { opacity: 1, transform: 'translateX(0)' } }, '@keyframes stepExitForward': { from: { opacity: 1, transform: 'translateX(0)' }, to: { opacity: 0, transform: 'translateX(-32px)' } }, '@keyframes stepExitBack': { from: { opacity: 1, transform: 'translateX(0)' }, to: { opacity: 0, transform: 'translateX(32px)' } } }}>
      <Typography id="guided-title" variant="h1">{step === 'goal' ? 'What is your goal?' : step === 'emphasis' ? 'What matters most?' : step === 'about' ? 'About you' : step === 'activity' ? 'Your usual day' : step === 'goalWeight' ? 'Goal weight & pace' : 'Your targets'}</Typography>
      {step === 'goal' ? <Stack spacing={2}>{GOALS.map((option) => <OptionCard key={option.value} {...option} selected={draft.objective === option.value} onClick={() => { const next = { ...draft, objective: option.value, pace: option.value === 'gain' && !['steady', 'standard'].includes(draft.pace) ? 'standard' as Pace : draft.pace }; setDraft(next); go('emphasis'); }} />)}</Stack> : null}
      {step === 'emphasis' ? <Stack spacing={2}>{EMPHASES.map((option) => <OptionCard key={option.value} {...option} selected={draft.emphasis === option.value} onClick={() => { set('emphasis', option.value); go('about'); }} />)}</Stack> : null}
      {step === 'about' ? <Box component="form" onSubmit={validateAbout}><Stack spacing={2.5}><TextField label="Date of birth" type="date" value={draft.dateOfBirth} onChange={(event) => set('dateOfBirth', event.target.value)} error={Boolean(errors.dateOfBirth)} helperText={errors.dateOfBirth} slotProps={{ inputLabel: { shrink: true } }} fullWidth /><TextField select label="Sex" value={draft.sex} onChange={(event) => set('sex', event.target.value as Draft['sex'])} error={Boolean(errors.sex)} helperText={errors.sex} fullWidth><MenuItem value="male">Male</MenuItem><MenuItem value="female">Female</MenuItem></TextField><Stack direction={{ xs: 'column', sm: 'row' }} spacing={2}><TextField label="Height" value={draft.heightCm} onChange={(event) => set('heightCm', event.target.value)} error={Boolean(errors.heightCm)} helperText={errors.heightCm} inputMode="decimal" slotProps={{ input: { endAdornment: <Typography variant="caption">cm</Typography> } }} fullWidth /><TextField label="Current weight" value={draft.currentWeight} onChange={(event) => set('currentWeight', event.target.value)} error={Boolean(errors.currentWeight)} helperText={errors.currentWeight} inputMode="decimal" slotProps={{ input: { endAdornment: <Typography variant="caption">kg</Typography> } }} fullWidth /></Stack><Button type="submit" variant="contained" size="large">Continue</Button></Stack></Box> : null}
      {step === 'activity' ? <Stack spacing={2}>{ACTIVITIES.map((option) => <OptionCard key={option.value} {...option} selected={draft.habitualActivity === option.value} onClick={() => { const next = { ...draft, habitualActivity: option.value }; setDraft(next); if (draft.objective === 'maintain') void showTargets(next); else go('goalWeight'); }} />)}</Stack> : null}
      {step === 'goalWeight' ? <Box component="form" onSubmit={submitGoal}><Stack spacing={3}><TextField label="Goal weight" value={draft.targetWeight} onChange={(event) => set('targetWeight', event.target.value)} error={Boolean(errors.targetWeight)} helperText={errors.targetWeight} inputMode="decimal" slotProps={{ input: { endAdornment: <Typography variant="caption">kg</Typography> } }} fullWidth /><Stack spacing={2}>{paceOptionsFor(draft.objective).map((pace) => <OptionCard key={pace.value} title={pace.label} caption={`${paceLabel(pace.value)} · ${pacePoundsLabel(pace.value)}`} selected={draft.pace === pace.value} onClick={() => set('pace', pace.value)} />)}</Stack><Button type="submit" variant="contained" size="large" disabled={preview.isPending}>{preview.isPending ? 'Working it out…' : 'See my targets'}</Button></Stack></Box> : null}
      {step === 'targets' && recommendation ? <Stack spacing={3} sx={{ textAlign: 'center' }}><Box><Typography variant="caption" color="text.secondary">Daily calories</Typography><Typography className="numeral" sx={{ typography: 'h1', fontSize: { xs: '3rem', sm: '4rem' } }}>{recommendation.calculation.recommended_kcal.toLocaleString('en-GB')} kcal</Typography></Box><Stack direction="row" spacing={1}><Stat label="Protein" value={recommendation.macros.protein_g} /><Stat label="Carbs" value={recommendation.macros.carbohydrate_g} /><Stat label="Fat" value={recommendation.macros.fat_g} /></Stack>{recommendation.estimated_goal_date ? <Typography className="numeral" variant="body1">On track for {formatDate(recommendation.estimated_goal_date)}</Typography> : null}{recommendation.calculation.eased ? <Alert severity="warning">We eased this pace to keep your target at the safety floor of <span className="numeral">{recommendation.calculation.floor_kcal.toLocaleString('en-GB')} kcal</span>.</Alert> : null}<Box sx={{ textAlign: 'left' }}><Button onClick={() => setDetailsOpen((open) => !open)} endIcon={detailsOpen ? <KeyboardArrowUpIcon /> : <KeyboardArrowDownIcon />}>How we worked this out</Button><Collapse in={detailsOpen}><Typography className="numeral" variant="body2" color="text.secondary" sx={{ px: 1, pt: 1 }}>Maintenance {recommendation.calculation.maintenance_kcal.toLocaleString('en-GB')} kcal · {recommendation.calculation.adjustment_kcal === 0 ? 'no daily adjustment' : `${Math.abs(recommendation.calculation.adjustment_kcal).toLocaleString('en-GB')} kcal ${recommendation.calculation.adjustment_kcal < 0 ? 'less' : 'more'} a day`}</Typography></Collapse></Box><Typography variant="body2" color="text.secondary">General guidance, not advice from a nutrition professional.</Typography><Stack spacing={1}><Button variant="contained" size="large" onClick={() => void saveTargets()} disabled={save.isPending}>{save.isPending ? 'Saving…' : 'Use these targets'}</Button><Button onClick={onManual}>Set my own numbers</Button></Stack></Stack> : null}
      {formError ? <Alert severity="error">{formError}</Alert> : null}
    </Stack></Box>
  </Stack><ConflictDialog error={conflict} onReload={() => { setConflict(null); onClose(); }} onDismiss={() => setConflict(null)} /></Dialog>;
}
