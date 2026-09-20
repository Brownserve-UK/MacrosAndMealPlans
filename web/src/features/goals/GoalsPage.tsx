import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Divider from '@mui/material/Divider';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import type { NutritionPlan, WeightDisplay, WeightRecord, WeightSummary } from '../../api/client';
import { useDeleteWeighIn, useBodyProfile, useMember, useNutritionPlan, useUpdateMember, useWeightRecords, useWeightSummary } from '../../api/queries';
import { useAuth } from '../../auth/AuthProvider';
import { FormDialog } from '../../components/FormDialog';
import { PageHeader } from '../../components/PageHeader';
import { ErrorState, Loading } from '../../components/States';
import { GuidedSetupDialog } from './GuidedSetupDialog';
import { ManualTargetDialog } from './ManualTargetDialog';
import { WeighInDialog, type WeighInDialogState } from './WeighInDialog';
import { WeightChart } from './WeightChart';
import { formatWeight } from './weightFormat';

const DISPLAYS: { value: WeightDisplay; label: string }[] = [{ value: 'kilograms', label: 'kg' }, { value: 'stones_pounds', label: 'st + lb' }, { value: 'pounds', label: 'lb' }];
type DialogState = 'guided' | 'manual' | null;
type Range = '1M' | '3M' | 'All';

function formatDate(iso: string) { return new Date(`${iso}T00:00:00`).toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' }); }
function formatShortDate(iso: string) { return new Date(`${iso}T00:00:00`).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' }); }
function rangeStart(range: Range) {
  if (range === 'All') return undefined;
  const date = new Date();
  date.setUTCMonth(date.getUTCMonth() - (range === '1M' ? 1 : 3));
  return date.toISOString().slice(0, 10);
}

function MacroStats({ plan }: { plan: NutritionPlan }) {
  const stats = [['Protein', plan.target?.protein_g], ['Carbs', plan.target?.carbohydrate_g], ['Fat', plan.target?.fat_g]] as const;
  return <Stack direction="row" spacing={1}>{stats.map(([label, value]) => <Paper elevation={0} key={label} sx={{ flex: 1, p: 2, textAlign: 'center' }}><Typography className="numeral" variant="h3">{value?.toLocaleString('en-GB') ?? '–'} g</Typography><Typography variant="caption" color="text.secondary">{label}</Typography></Paper>)}</Stack>;
}

export function GoalsPage() {
  const { principal } = useAuth(); const memberId = principal?.member_id ?? '';
  const [range, setRange] = useState<Range>('3M');
  const member = useMember(memberId); const plan = useNutritionPlan(memberId); const summary = useWeightSummary(memberId, rangeStart(range)); const records = useWeightRecords(memberId, 3); const profile = useBodyProfile(memberId);
  if (!principal) return null;
  if (!memberId) return <><PageHeader title="Goals" /><Typography variant="body2" color="text.secondary">Your account is not linked to a household member yet.</Typography></>;
  if (member.isLoading || plan.isLoading || summary.isLoading || records.isLoading) return <Loading label="Loading goals" />;
  if (member.isError) return <ErrorState error={member.error} onRetry={() => member.refetch()} />;
  if (plan.isError) return <ErrorState error={plan.error} onRetry={() => plan.refetch()} />;
  if (summary.isError) return <ErrorState error={summary.error} onRetry={() => summary.refetch()} />;
  if (records.isError) return <ErrorState error={records.error} onRetry={() => records.refetch()} />;
  return <GoalsView memberId={memberId} display={member.data?.weight_display ?? 'kilograms'} memberRevision={member.data?.revision ?? 0} plan={plan.data as NutritionPlan} summary={summary.data as WeightSummary} records={records.data ?? []} profile={profile.data} range={range} onRangeChange={setRange} />;
}

function GoalsView({ memberId, display, memberRevision, plan, summary, records, profile, range, onRangeChange }: { memberId: string; display: WeightDisplay; memberRevision: number; plan: NutritionPlan; summary: WeightSummary; records: WeightRecord[]; profile: ReturnType<typeof useBodyProfile>['data']; range: Range; onRangeChange: (range: Range) => void }) {
  const updateMember = useUpdateMember(); const [dialog, setDialog] = useState<DialogState>(null); const [weighIn, setWeighIn] = useState<WeighInDialogState | null>(null); const [removing, setRemoving] = useState<WeightRecord | null>(null); const [allOpen, setAllOpen] = useState(false); const allRecords = useWeightRecords(memberId, undefined, allOpen);
  const points = summary.series;
  const first = points[0]; const latest = points.at(-1); const change = first && latest ? latest.weight_kg - first.weight_kg : null;
  const favourable = change != null && (summary.goal?.objective === 'lose' ? change < 0 : summary.goal?.objective === 'gain' ? change > 0 : change === 0);
  const changeColour = change === 0 || summary.goal == null ? 'text.secondary' : favourable ? 'success' : 'warning';
  const projectionOn = summary.projection?.status === 'projected' ? summary.projection.on : null;

  return <><PageHeader title="Goals" actions={<ToggleButtonGroup size="small" exclusive value={display} onChange={(_event, next: WeightDisplay | null) => { if (next && next !== display) updateMember.mutate({ id: memberId, revision: memberRevision, body: { weight_display: next } }); }} aria-label="Show weights in">{DISPLAYS.map((option) => <ToggleButton key={option.value} value={option.value}>{option.label}</ToggleButton>)}</ToggleButtonGroup>} />
    <Stack spacing={3}>
      {plan.target?.energy_kcal == null ? <Paper elevation={0} sx={{ p: 3 }}><Stack spacing={2}><Typography variant="body2" color="text.secondary">Set calorie and macro targets that work for you.</Typography><Stack spacing={1} sx={{ alignItems: 'flex-start' }}><Button variant="contained" onClick={() => setDialog('guided')}>Get started</Button><Button onClick={() => setDialog('manual')}>Set my own numbers</Button></Stack></Stack></Paper> : <Paper elevation={0} sx={{ p: 3 }}><Stack spacing={2.5}><BoxTarget calories={plan.target.energy_kcal} /><MacroStats plan={plan} /><Stack direction={{ xs: 'column', sm: 'row' }} spacing={1} sx={{ alignItems: { xs: 'stretch', sm: 'center' } }}><Button variant="contained" onClick={() => setDialog('guided')}>Review targets</Button><Button onClick={() => setDialog('manual')}>Set a different number</Button></Stack></Stack></Paper>}

      <Paper elevation={0} sx={{ p: 3 }}><Stack spacing={2.5}>
        <Stack direction={{ xs: 'column', sm: 'row' }} sx={{ justifyContent: 'space-between', gap: 2, alignItems: { sm: 'flex-start' } }}><Stack spacing={0.5}><Typography variant="h3">Weight</Typography>{summary.latest ? <><Typography className="numeral" variant="h3" sx={{ fontSize: '1.75rem' }}>{formatWeight(summary.latest.weight_kg, display)}</Typography>{change != null && first ? <Typography className="numeral" variant="body2" color={changeColour}>{change === 0 ? 'No change' : `${change < 0 ? '↓' : '↑'} ${formatWeight(Math.abs(change), display)}`} since {formatShortDate(first.on)}</Typography> : null}</> : <Typography variant="body2" color="text.secondary">Add your first weigh-in to see your trend.</Typography>}</Stack><Button variant="contained" onClick={() => setWeighIn({ mode: 'create' })}>Add weigh-in</Button></Stack>
        {summary.series.length > 0 ? <><Stack direction={{ xs: 'column', sm: 'row' }} sx={{ justifyContent: 'space-between', gap: 1, alignItems: { sm: 'center' } }}><ToggleButtonGroup size="small" exclusive value={range} onChange={(_event, next: Range | null) => next && onRangeChange(next)} aria-label="Weight chart range" sx={{ alignSelf: 'flex-start' }}><ToggleButton value="1M">1M</ToggleButton><ToggleButton value="3M">3M</ToggleButton><ToggleButton value="All">All</ToggleButton></ToggleButtonGroup>{projectionOn && summary.goal?.target_weight_kg != null ? <Typography className="numeral" variant="caption" color="text.secondary">Projected {formatWeight(summary.goal.target_weight_kg, display)} by {formatShortDate(projectionOn)}</Typography> : null}</Stack><WeightChart points={points} goalKg={summary.goal?.target_weight_kg} display={display} /></> : null}
        {records.length > 0 ? <><Divider /><Typography variant="h3">Recent weigh-ins</Typography><RecordList records={records.slice(0, 3)} display={display} onEdit={(record) => setWeighIn({ mode: 'edit', record })} onDelete={setRemoving} /><Button onClick={() => setAllOpen(true)} sx={{ alignSelf: 'flex-start' }}>See all weigh-ins</Button></> : null}
      </Stack></Paper>
    </Stack>
    {dialog === 'guided' ? <GuidedSetupDialog memberId={memberId} profile={profile} summary={summary} emphasis={plan.calculation?.emphasis} onClose={() => setDialog(null)} onManual={() => setDialog('manual')} /> : null}
    {dialog === 'manual' ? <ManualTargetDialog memberId={memberId} target={plan.target} floorKcal={plan.calculation?.floor_kcal ?? (profile?.sex === 'male' ? 1500 : profile?.sex === 'female' ? 1200 : null)} onClose={() => setDialog(null)} /> : null}
    {weighIn ? <WeighInDialog memberId={memberId} display={display} state={weighIn} onClose={() => setWeighIn(null)} /> : null}
    {removing ? <DeleteWeighInDialog memberId={memberId} display={display} record={removing} onClose={() => setRemoving(null)} /> : null}
    {allOpen ? <FormDialog open onClose={() => setAllOpen(false)} fullWidth maxWidth="sm"><DialogTitle>All weigh-ins</DialogTitle><DialogContent dividers><RecordList records={allRecords.data ?? records} display={display} onEdit={(record) => { setAllOpen(false); setWeighIn({ mode: 'edit', record }); }} onDelete={(record) => { setAllOpen(false); setRemoving(record); }} /></DialogContent><DialogActions><Button onClick={() => setAllOpen(false)}>Close</Button></DialogActions></FormDialog> : null}
  </>;
}

function BoxTarget({ calories }: { calories: number }) { return <Stack spacing={0.5}><Typography variant="caption" color="text.secondary">Daily calories</Typography><Typography className="numeral" sx={{ typography: 'h1', fontSize: { xs: '2.75rem', sm: '3.5rem' } }}>{calories.toLocaleString('en-GB')} kcal</Typography></Stack>; }

function RecordList({ records, display, onEdit, onDelete }: { records: WeightRecord[]; display: WeightDisplay; onEdit: (record: WeightRecord) => void; onDelete: (record: WeightRecord) => void }) {
  return <Stack divider={<Divider />}>{records.map((record) => <Stack key={record.id} direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between', py: 1, gap: 1 }}><Stack><Typography className="numeral" variant="body2">{formatWeight(record.weight_kg, display)}</Typography><Typography className="numeral" variant="caption" color="text.secondary">{formatDate(record.recorded_on)}</Typography></Stack><Stack direction="row" spacing={0.5}><Button size="small" aria-label={`Edit weigh-in from ${formatDate(record.recorded_on)}`} onClick={() => onEdit(record)}>Edit</Button><Button size="small" aria-label={`Delete weigh-in from ${formatDate(record.recorded_on)}`} onClick={() => onDelete(record)}>Delete</Button></Stack></Stack>)}</Stack>;
}

function DeleteWeighInDialog({ memberId, display, record, onClose }: { memberId: string; display: WeightDisplay; record: WeightRecord; onClose: () => void }) {
  const remove = useDeleteWeighIn(); const [error, setError] = useState<string | null>(null);
  async function confirm() { setError(null); try { await remove.mutateAsync({ id: record.id, revision: record.revision, memberId }); onClose(); } catch { setError('Could not delete this weigh-in.'); } }
  return <FormDialog open onClose={onClose} fullWidth maxWidth="xs"><DialogTitle>Delete this weigh-in?</DialogTitle><DialogContent dividers><Stack spacing={2}><Typography variant="body2"><span className="numeral">{formatWeight(record.weight_kg, display)}</span> on <span className="numeral">{formatDate(record.recorded_on)}</span> will be removed from your trend.</Typography>{error ? <Alert severity="error">{error}</Alert> : null}</Stack></DialogContent><DialogActions><Button onClick={onClose}>Cancel</Button><Button onClick={() => void confirm()} disabled={remove.isPending}>Delete</Button></DialogActions></FormDialog>;
}
