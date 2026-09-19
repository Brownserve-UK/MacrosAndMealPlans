import CloseIcon from '@mui/icons-material/CloseOutlined';
import EditIcon from '@mui/icons-material/EditOutlined';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Drawer from '@mui/material/Drawer';
import IconButton from '@mui/material/IconButton';
import InputBase from '@mui/material/InputBase';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import { ApiError } from '../../../api/client';
import { useAddGroup, useAddPlannerGuest, useChangePlannerGuest, useRemovePlannerGuest, useSetAttendance, useSplitPlannerGuests, useUpdateOccasion } from '../../../api/queries';
import { FormDialog } from '../../../components/FormDialog';
import { CookingList } from './CookingList';
import { GuestMenu, type GuestTarget } from './GuestMenu';
import { MealSheet } from './MealSheet';
import { PersonPicker } from './PersonPicker';
import { Roster } from './Roster';
import { memberStatus, occasionTitle, shortDate } from './plannerWeek';
import type { GroupView, NewGroup, OccasionView, PlannerGuest, PlannerMember, PlannerWeek } from './types';

type Picker =
  | { anchor: HTMLElement; kind: 'member'; member: PlannerMember }
  | { anchor: HTMLElement; kind: 'guests'; group: GroupView | null; guest: PlannerGuest | null };

export function OccasionCard({
  occasion,
  week,
  sheet,
  onClose,
  onMove,
  onCopy,
  onDelete,
}: {
  occasion: OccasionView;
  week: PlannerWeek;
  sheet: boolean;
  onClose: () => void;
  onMove: () => void;
  onCopy: () => void;
  onDelete: () => void;
}) {
  const updateOccasion = useUpdateOccasion();
  const addGroup = useAddGroup();
  const setAttendance = useSetAttendance();
  const addGuest = useAddPlannerGuest();
  const changeGuest = useChangePlannerGuest();
  const removeGuest = useRemovePlannerGuest();
  const splitGuests = useSplitPlannerGuests();
  const [error, setError] = useState<string | null>(null);
  const [editingTime, setEditingTime] = useState(false);
  const [timeText, setTimeText] = useState('');
  const [editingNote, setEditingNote] = useState(false);
  const [noteText, setNoteText] = useState('');
  const [picker, setPicker] = useState<Picker | null>(null);
  const [editingGroupId, setEditingGroupId] = useState<string | null>(null);

  const busy = updateOccasion.isPending || addGroup.isPending || setAttendance.isPending || addGuest.isPending || changeGuest.isPending || removeGuest.isPending || splitGuests.isPending;
  const editingGroup = occasion.groups.find((group) => group.id === editingGroupId) ?? null;
  const members = week.members;
  const toBuy = occasion.groups.reduce((total, group) => total + group.to_buy, 0);
  const hasFood = occasion.groups.some((group) => group.components.length > 0);

  async function run(action: () => Promise<unknown>, fallback: string) {
    try {
      setError(null);
      await action();
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : fallback);
    }
  }

  function retick(group: GroupView, member: PlannerMember) {
    void run(
      () => setAttendance.mutateAsync({ occasionId: occasion.id, memberId: member.id, attendance: { kind: 'eating', group_id: group.id } }),
      'Could not change who is eating.',
    );
  }

  function variation(group: GroupView, member: PlannerMember, note: string | null) {
    void run(
      () =>
        setAttendance.mutateAsync({
          occasionId: occasion.id,
          memberId: member.id,
          attendance: { kind: 'eating', group_id: group.id, note },
        }),
      'Could not save the variation.',
    );
  }

  function addFor(member: PlannerMember, group: NewGroup) {
    setPicker(null);
    void run(
      () =>
        addGroup.mutateAsync({
          occasionId: occasion.id,
          body: { ...group, everyone: false, participants: [{ member_id: member.id }] },
        }),
      `Could not add a meal for ${member.name}.`,
    );
  }

  function elsewhere(member: PlannerMember) {
    void run(
      () => setAttendance.mutateAsync({ occasionId: occasion.id, memberId: member.id, attendance: { kind: 'elsewhere' } }),
      'Could not mark them as out.',
    );
  }

  function chooseGuest(target: GuestTarget) {
    if (picker?.kind !== 'guests') return;
    const guest = picker.guest;
    setPicker(null);
    void run(() => guest
      ? changeGuest.mutateAsync({ occasionId: occasion.id, guestId: guest.id, revision: occasion.revision, target })
      : addGuest.mutateAsync({ occasionId: occasion.id, revision: occasion.revision, name: null, target }),
    'Could not change the guest.');
  }

  function commitTime() {
    setEditingTime(false);
    const value = timeText.trim() === '' ? null : timeText;
    if (value === occasion.planned_time) return;
    void run(
      () => updateOccasion.mutateAsync({ id: occasion.id, body: { planned_time: value, revision: occasion.revision } }),
      'Could not change the time.',
    );
  }

  function commitNote() {
    setEditingNote(false);
    const value = noteText.trim() === '' ? null : noteText.trim();
    if (value === occasion.note) return;
    void run(
      () => updateOccasion.mutateAsync({ id: occasion.id, body: { note: value, revision: occasion.revision } }),
      'Could not save the note.',
    );
  }

  const header = (
    <Stack direction="row" sx={{ alignItems: 'flex-start', justifyContent: 'space-between', gap: 2 }}>
      <Box>
        <Typography variant="h2" component="h2">
          {occasionTitle(occasion)}
        </Typography>
        <Stack direction="row" spacing={1} sx={{ alignItems: 'center', mt: 0.5 }}>
          <Typography variant="body2" color="text.secondary" className="numeral">
            {shortDate(occasion.planned_on)}
          </Typography>
          <Typography variant="body2" color="text.secondary">
            ·
          </Typography>
          {editingTime ? (
            <InputBase
              autoFocus
              type="time"
              value={timeText}
              inputProps={{ 'aria-label': 'Meal time' }}
              onChange={(event) => setTimeText(event.target.value)}
              onBlur={commitTime}
              onKeyDown={(event) => {
                if (event.key === 'Enter') commitTime();
                if (event.key === 'Escape') setEditingTime(false);
              }}
              sx={{ fontSize: '0.875rem', border: '1px solid', borderColor: 'primary.main', borderRadius: '8px', px: 1 }}
            />
          ) : (
            <>
              <Typography variant="body2" color="text.secondary" className="numeral">
                {occasion.effective_time ?? 'No set time'}
              </Typography>
              <IconButton
                size="small"
                aria-label="Change the time"
                onClick={() => {
                  setTimeText(occasion.effective_time ?? '');
                  setEditingTime(true);
                }}
                sx={{ color: 'text.disabled', p: 0.25 }}
              >
                <EditIcon sx={{ fontSize: 14 }} />
              </IconButton>
            </>
          )}
        </Stack>
      </Box>
      <IconButton aria-label="Close" onClick={onClose} sx={{ border: '1px solid', borderColor: 'divider', borderRadius: '10px' }}>
        <CloseIcon fontSize="small" />
      </IconButton>
    </Stack>
  );

  const body = (
    <Stack spacing={1.5}>
      {error ? <Alert severity="error" onClose={() => setError(null)}>{error}</Alert> : null}
      <Roster
        occasion={occasion}
        week={week}
        busy={busy}
        onOpenMember={(member, anchor) => setPicker({ anchor, kind: 'member', member })}
        onAddGuest={(anchor) => setPicker({ anchor, kind: 'guests', group: null, guest: null })}
        onOpenGuests={(group, guest, anchor) => setPicker({ anchor, kind: 'guests', group, guest })}
        onRenameGuest={(guest, name) => { void run(() => changeGuest.mutateAsync({ occasionId: occasion.id, guestId: guest.id, revision: occasion.revision, name }), 'Could not change the guest name.'); }}
        onEditMeal={(group) => setEditingGroupId(group.id)}
      />
      <CookingList occasion={occasion} members={members} />
      <Stack>
        <Stack direction="row" sx={{ justifyContent: 'space-between', gap: 2, py: 1.5, borderTop: '1px solid', borderColor: 'divider' }}>
          <Typography variant="body2" color="text.secondary">
            Note
          </Typography>
          {editingNote ? (
            <InputBase
              autoFocus
              value={noteText}
              inputProps={{ 'aria-label': 'Note' }}
              onChange={(event) => setNoteText(event.target.value)}
              onBlur={commitNote}
              onKeyDown={(event) => {
                if (event.key === 'Enter') commitNote();
                if (event.key === 'Escape') setEditingNote(false);
              }}
              sx={{ fontSize: '0.875rem', flexGrow: 1, textAlign: 'right', border: '1px solid', borderColor: 'primary.main', borderRadius: '8px', px: 1 }}
            />
          ) : (
            <Box
              component="button"
              type="button"
              onClick={() => {
                setNoteText(occasion.note ?? '');
                setEditingNote(true);
              }}
              sx={{ background: 'none', border: 0, p: 0, font: 'inherit', cursor: 'pointer', textAlign: 'right' }}
            >
              <Typography variant="body2" sx={{ color: occasion.note ? 'text.primary' : 'text.disabled' }}>
                {occasion.note ?? 'Add a note'}
              </Typography>
            </Box>
          )}
        </Stack>
        <Stack direction="row" sx={{ justifyContent: 'space-between', gap: 2, py: 1.5, borderTop: '1px solid', borderColor: 'divider' }}>
          <Typography variant="body2" color="text.secondary">
            Shopping
          </Typography>
          <Typography variant="body2" className="numeral" sx={{ color: toBuy > 0 ? 'warning.main' : 'text.primary' }}>
            {toBuy > 0 ? `${toBuy} to buy` : hasFood ? 'Everything in stock' : 'Nothing to buy'}
          </Typography>
        </Stack>
      </Stack>
    </Stack>
  );

  const footer = (
    <Stack direction="row" sx={{ justifyContent: 'space-between', width: '100%', borderTop: '1px solid', borderColor: 'divider', pt: 1.5 }}>
      <Stack direction="row" spacing={0.5} sx={{ ml: -1.25 }}>
        <Button onClick={onMove}>Move</Button>
        <Button onClick={onCopy}>Copy to day</Button>
      </Stack>
      <Button color="error" onClick={onDelete} sx={{ mr: -1.25 }}>
        Delete
      </Button>
    </Stack>
  );

  const pickerElement = (
    <>
      <PersonPicker
        open={picker?.kind === 'member'}
        anchorEl={picker?.anchor ?? null}
        sheet={sheet}
        member={picker?.kind === 'member' ? picker.member : null}
        groups={occasion.groups}
        status={picker?.kind === 'member' ? memberStatus(occasion, picker.member, members) : null}
        onRetick={(group) => {
          if (picker?.kind === 'member') retick(group, picker.member);
          setPicker(null);
        }}
        onElsewhere={() => {
          if (picker?.kind === 'member') elsewhere(picker.member);
          setPicker(null);
        }}
        onAddFor={(group) => {
          if (picker?.kind === 'member') addFor(picker.member, group);
        }}
        onVariation={(group, note) => {
          if (picker?.kind === 'member') variation(group, picker.member, note);
          setPicker(null);
        }}
        onEditMeal={(group) => {
          setEditingGroupId(group.id);
          setPicker(null);
        }}
        onClose={() => setPicker(null)}
      />
      <MealSheet
        open={editingGroupId !== null}
        group={editingGroup}
        occasion={occasion}
        sheet={sheet}
        onClose={() => setEditingGroupId(null)}
      />
      <GuestMenu
        open={picker?.kind === 'guests'}
        anchorEl={picker?.anchor ?? null}
        sheet={sheet}
        group={picker?.kind === 'guests' ? picker.group : null}
        guest={picker?.kind === 'guests' ? picker.guest : null}
        groups={occasion.groups}
        onSelect={chooseGuest}
        onVariation={(note) => {
          if (picker?.kind === 'guests' && picker.guest) void run(() => changeGuest.mutateAsync({ occasionId: occasion.id, guestId: picker.guest!.id, revision: occasion.revision, note }), 'Could not save the variation.');
          setPicker(null);
        }}
        onRemove={() => {
          if (picker?.kind === 'guests' && picker.guest) void run(() => removeGuest.mutateAsync({ occasionId: occasion.id, guestId: picker.guest!.id, revision: occasion.revision }), 'Could not remove the guest.');
          setPicker(null);
        }}
        onSplit={() => {
          if (picker?.kind === 'guests' && picker.guest) void run(() => splitGuests.mutateAsync({ occasionId: occasion.id, guestId: picker.guest!.id, revision: occasion.revision }), 'Could not split the guests.');
          setPicker(null);
        }}
        onClose={() => setPicker(null)}
      />
    </>
  );

  if (sheet) {
    return (
      <Drawer anchor="bottom" open onClose={onClose} slotProps={{ paper: { sx: { borderRadius: '14px 14px 0 0', maxHeight: '92vh' } } }}>
        <Stack spacing={3} sx={{ px: 2.5, pt: 3, pb: 2 }}>
          {header}
          {body}
          {footer}
        </Stack>
        {pickerElement}
      </Drawer>
    );
  }

  return (
    <FormDialog open onClose={onClose} fullWidth maxWidth={false} slotProps={{ paper: { sx: { width: 600, maxWidth: 'calc(100vw - 32px)' } } }}>
      <DialogTitle component="div" sx={{ px: 4, pt: 3.5, pb: 1 }}>
        {header}
      </DialogTitle>
      <DialogContent sx={{ px: 4, pt: 2 }}>{body}</DialogContent>
      <DialogActions sx={{ px: 4, pb: 3, pt: 0 }}>{footer}</DialogActions>
      {pickerElement}
    </FormDialog>
  );
}
