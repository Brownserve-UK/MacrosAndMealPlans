import AddIcon from '@mui/icons-material/AddOutlined';
import CheckIcon from '@mui/icons-material/CheckOutlined';
import RemoveIcon from '@mui/icons-material/RemoveOutlined';
import Drawer from '@mui/material/Drawer';
import Popover from '@mui/material/Popover';
import Stack from '@mui/material/Stack';
import { PickerRowButton, SectionHeading } from './pickerParts';
import { conceptFor, dishLabel } from './plannerWeek';
import type { GroupView } from './types';

function GuestMenuBody({
  group,
  groups,
  onMove,
  onAdd,
  onRemove,
}: {
  group: GroupView;
  groups: GroupView[];
  onMove: (target: GroupView) => void;
  onAdd: () => void;
  onRemove: () => void;
}) {
  return (
    <Stack spacing={0.25} sx={{ p: 1 }}>
      {groups.length > 0 ? (
        <>
          <SectionHeading>This meal</SectionHeading>
          {groups.map((candidate) => (
            <PickerRowButton
              key={candidate.id}
              row={{ title: dishLabel(candidate), caption: null, concept: conceptFor(candidate) }}
              selected={false}
              trailing={candidate.id === group.id ? <CheckIcon sx={{ fontSize: 17, color: 'primary.main', display: 'block' }} /> : null}
              onPick={() => onMove(candidate)}
            />
          ))}
        </>
      ) : null}

      <PickerRowButton
        row={{ title: 'Add another guest', caption: null, concept: 'meal' }}
        selected={false}
        icon={<AddIcon sx={{ fontSize: 17 }} />}
        onPick={onAdd}
      />
      <PickerRowButton
        row={{ title: 'Remove a guest', caption: null, concept: 'meal' }}
        selected={false}
        icon={<RemoveIcon sx={{ fontSize: 17 }} />}
        danger
        onPick={onRemove}
      />
    </Stack>
  );
}

export function GuestMenu({
  open,
  anchorEl,
  sheet,
  group,
  groups,
  onMove,
  onAdd,
  onRemove,
  onClose,
}: {
  open: boolean;
  anchorEl: HTMLElement | null;
  sheet: boolean;
  group: GroupView | null;
  groups: GroupView[];
  onMove: (target: GroupView) => void;
  onAdd: () => void;
  onRemove: () => void;
  onClose: () => void;
}) {
  const body =
    open && group ? <GuestMenuBody group={group} groups={groups} onMove={onMove} onAdd={onAdd} onRemove={onRemove} /> : null;

  if (sheet) {
    return (
      <Drawer anchor="bottom" open={open} onClose={onClose} slotProps={{ paper: { sx: { borderRadius: '14px 14px 0 0', maxHeight: '85vh' } } }}>
        {body}
      </Drawer>
    );
  }
  return (
    <Popover
      open={open && Boolean(anchorEl)}
      anchorEl={anchorEl}
      onClose={onClose}
      anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
      transformOrigin={{ vertical: 'top', horizontal: 'left' }}
      slotProps={{ paper: { sx: { width: 400, mt: 1, borderRadius: '14px', maxHeight: 'min(70vh, 560px)' } } }}
    >
      {body}
    </Popover>
  );
}
