import CheckIcon from '@mui/icons-material/CheckOutlined';
import CloseIcon from '@mui/icons-material/CloseOutlined';
import EditIcon from '@mui/icons-material/EditOutlined';
import ListAltIcon from '@mui/icons-material/ListAltOutlined';
import Drawer from '@mui/material/Drawer';
import Popover from '@mui/material/Popover';
import Stack from '@mui/material/Stack';
import type { KeyboardEvent, ReactNode } from 'react';
import { useMemo, useState } from 'react';
import { PickerPromptStep, PickerRowButton, PickerSearchField, SectionHeading, type PickerRowContent } from './pickerParts';
import { conceptFor, groupDisplayName, groupFoodCaption, groupShortName, type MemberStatus, memberVariation } from './plannerWeek';
import type { GroupView, NewGroup, PlannerMember } from './types';
import { leftoversRow, useFridgeDishes, usePickerRows } from './usePickerRows';

type Row = PickerRowContent & {
  key: string;
  icon?: ReactNode;
  danger?: boolean;
  ticked?: boolean;
  onSelect: () => void;
};

function PersonPickerBody({
  member,
  groups,
  status,
  onRetick,
  onElsewhere,
  onAddFor,
  onVariation,
  onEditMeal,
  onClose,
}: {
  member: PlannerMember;
  groups: GroupView[];
  status: MemberStatus;
  onRetick: (group: GroupView) => void;
  onElsewhere: () => void;
  onAddFor: (group: NewGroup) => void;
  onVariation: (group: GroupView, note: string | null) => void;
  onEditMeal: (group: GroupView) => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState(0);
  const [prompting, setPrompting] = useState(false);
  const { rows } = usePickerRows(query);
  const dishes = useFridgeDishes();
  const typed = query.trim();

  const currentGroupId = status.kind === 'eating' ? status.group.id : null;
  const note = status.kind === 'eating' ? memberVariation(status.group, member.id) : null;

  const thisMeal = useMemo<Row[]>(
    () =>
      groups.map((group) => {
        const name = groupShortName(group);
        const full = groupDisplayName(group);
        const joined = full.tail ? `${full.head} ${full.tail}` : full.head;
        const short = name.tail ? `${name.head} ${name.tail}` : name.head;
        return {
          key: `group:${group.id}`,
          title: short,
          caption: groupFoodCaption(group) ?? (joined === short ? null : joined),
          concept: conceptFor(group),
          ticked: group.id === currentGroupId,
          onSelect: () => onRetick(group),
        };
      }),
    [groups, currentGroupId, onRetick],
  );

  const found = useMemo<Row[]>(() => {
    if (!typed) return [];
    const head: Row[] = [
      {
        key: 'label',
        title: `Add “${typed}”`,
        caption: 'Just a name, link a recipe later',
        concept: 'meal',
        onSelect: () => onAddFor({ label: typed }),
      },
    ];
    const matches: Row[] = rows
      .filter((row) => row.section === 'matches')
      .map((row) => ({
        key: row.id,
        title: row.title,
        caption: row.caption,
        concept: row.concept,
        onSelect: () => onAddFor(row.group),
      }));
    return [...head, ...matches];
  }, [typed, rows, onAddFor]);

  const quick = useMemo<Row[]>(() => {
    if (typed) return [];
    const leftovers: Row[] = dishes.slice(0, 3).map((dish) => {
      const row = leftoversRow(dish);
      return {
        key: row.id,
        title: row.title,
        caption: row.caption,
        concept: row.concept,
        onSelect: () => onAddFor(row.group),
      };
    });
    return [
      ...leftovers,
      {
        key: 'elsewhere',
        title: 'Eating elsewhere',
        caption: null,
        concept: 'out',
        onSelect: onElsewhere,
      },
    ];
  }, [typed, dishes, onElsewhere, onAddFor]);

  const editRow = useMemo<Row[]>(() => {
    if (typed || status.kind !== 'eating') return [];
    const group = status.group;
    return [
      {
        key: 'edit-meal',
        title: 'Edit this meal',
        caption: null,
        concept: 'meal',
        icon: <ListAltIcon sx={{ fontSize: 16 }} />,
        onSelect: () => {
          onEditMeal(group);
          onClose();
        },
      },
    ];
  }, [typed, status, onEditMeal, onClose]);

  const variationRows = useMemo<Row[]>(() => {
    if (typed || status.kind !== 'eating') return [];
    const group = status.group;
    const rowsArr: Row[] = [
      {
        key: 'variation',
        title: note ? 'Change the variation' : 'Add a variation',
        caption: null,
        concept: 'meal',
        icon: <EditIcon sx={{ fontSize: 16 }} />,
        onSelect: () => setPrompting(true),
      },
    ];
    if (note) {
      rowsArr.push({
        key: 'clear-variation',
        title: 'Remove the variation',
        caption: null,
        concept: 'meal',
        icon: <CloseIcon sx={{ fontSize: 16 }} />,
        danger: true,
        onSelect: () => onVariation(group, null),
      });
    }
    return rowsArr;
  }, [typed, status, note, onVariation]);

  const visible = typed ? found : [...thisMeal, ...quick, ...editRow, ...variationRows];
  const current = Math.min(selected, Math.max(0, visible.length - 1));

  function keyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      setSelected(Math.min(visible.length - 1, current + 1));
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      setSelected(Math.max(0, current - 1));
    } else if (event.key === 'Enter') {
      event.preventDefault();
      visible[current]?.onSelect();
    } else if (event.key === 'Escape') {
      onClose();
    }
  }

  if (prompting && status.kind === 'eating') {
    const group = status.group;
    return (
      <PickerPromptStep
        label={`How ${member.name} has it`}
        initial={note ?? ''}
        onSubmit={(value) => onVariation(group, value)}
        onClose={onClose}
      />
    );
  }

  return (
    <Stack spacing={0.25} sx={{ p: 1 }}>
      <PickerSearchField
        value={query}
        onChange={(value) => {
          setQuery(value);
          setSelected(0);
        }}
        onKeyDown={keyDown}
        placeholder={`Something for ${member.name}`}
      />

      {typed
        ? found.map((row, index) => (
            <PickerRowButton key={row.key} row={row} selected={index === current} onPick={row.onSelect} />
          ))
        : (
            <>
              {thisMeal.length > 0 ? (
                <>
                  <SectionHeading>This meal</SectionHeading>
                  {thisMeal.map((row, index) => (
                    <PickerRowButton
                      key={row.key}
                      row={row}
                      selected={index === current}
                      trailing={row.ticked ? <CheckIcon sx={{ fontSize: 17, color: 'primary.main', display: 'block' }} /> : null}
                      onPick={row.onSelect}
                    />
                  ))}
                </>
              ) : null}

              {quick.length > 0 ? (
                <>
                  <SectionHeading>Quick picks</SectionHeading>
                  {quick.map((row, index) => (
                    <PickerRowButton
                      key={row.key}
                      row={row}
                      selected={thisMeal.length + index === current}
                      onPick={row.onSelect}
                    />
                  ))}
                </>
              ) : null}

              {editRow.map((row, index) => (
                <PickerRowButton
                  key={row.key}
                  row={row}
                  selected={thisMeal.length + quick.length + index === current}
                  icon={row.icon}
                  onPick={row.onSelect}
                />
              ))}

              {variationRows.map((row, index) => (
                <PickerRowButton
                  key={row.key}
                  row={row}
                  selected={thisMeal.length + quick.length + editRow.length + index === current}
                  icon={row.icon}
                  danger={row.danger}
                  onPick={row.onSelect}
                />
              ))}
            </>
          )}
    </Stack>
  );
}

export function PersonPicker({
  open,
  anchorEl,
  sheet,
  member,
  groups,
  status,
  onRetick,
  onElsewhere,
  onAddFor,
  onVariation,
  onEditMeal,
  onClose,
}: {
  open: boolean;
  anchorEl: HTMLElement | null;
  sheet: boolean;
  member: PlannerMember | null;
  groups: GroupView[];
  status: MemberStatus | null;
  onRetick: (group: GroupView) => void;
  onElsewhere: () => void;
  onAddFor: (group: NewGroup) => void;
  onVariation: (group: GroupView, note: string | null) => void;
  onEditMeal: (group: GroupView) => void;
  onClose: () => void;
}) {
  const body =
    open && member && status ? (
      <PersonPickerBody
        member={member}
        groups={groups}
        status={status}
        onRetick={onRetick}
        onElsewhere={onElsewhere}
        onAddFor={onAddFor}
        onVariation={onVariation}
        onEditMeal={onEditMeal}
        onClose={onClose}
      />
    ) : null;

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
