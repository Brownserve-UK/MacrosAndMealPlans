import ChevronRightIcon from '@mui/icons-material/ChevronRightOutlined';
import WarningIcon from '@mui/icons-material/WarningAmberOutlined';
import Box from '@mui/material/Box';
import ButtonBase from '@mui/material/ButtonBase';
import Chip from '@mui/material/Chip';
import Divider from '@mui/material/Divider';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { PlannerMeal } from '../../api/client';
import { IconTile } from '../../components/IconTile';
import { InitialsAvatar } from '../../components/InitialsAvatar';

export type MealRowModel = {
  title: string;
  caption: string;
  chip?: string | null;
};

function guestCount(meal: PlannerMeal) {
  return meal.guest_groups.reduce((sum, group) => sum + group.count, 0);
}

function attendeeNames(meal: PlannerMeal, memberId?: string | null) {
  const names = meal.people.map((person) => person.member_id === memberId ? 'you' : person.display_name);
  const guests = guestCount(meal);
  if (names.length === 0 && meal.owner_name) names.push(meal.owner_name);
  if (names.length === 1 && names[0] === 'you' && guests === 0) return 'just you';
  if (names.length > 3) {
    const others = names.length - 3 + guests;
    return `${names.slice(0, 3).join(', ')} and ${others} ${others === 1 ? 'other' : 'others'}`;
  }
  const people = names.join(', ') || 'No attendees';
  return guests > 0 ? `${people} + ${guests} ${guests === 1 ? 'guest' : 'guests'}` : people;
}

export function mealTitle(meal: PlannerMeal) {
  return meal.foods.map((food) => food.item_name).join(', ') || 'Nothing planned';
}

export function plannerMealRow(meal: PlannerMeal, memberId?: string | null): MealRowModel {
  const attendance = attendeeNames(meal, memberId);
  return {
    title: mealTitle(meal),
    caption: meal.planned_time ? `${meal.planned_time} · ${attendance}` : attendance,
  };
}

function Chevron() {
  return <ChevronRightIcon sx={{ color: 'text.disabled', flexShrink: 0 }} />;
}

export function MealRow({
  model,
  onClick,
  warning,
}: {
  model: MealRowModel;
  onClick: () => void;
  warning?: string | null;
}) {
  return (
    <Paper sx={{ overflow: 'hidden' }}>
      <ButtonBase onClick={onClick} sx={{ display: 'block', width: '100%', textAlign: 'left' }}>
        <Stack direction="row" spacing={2} sx={{ alignItems: 'center', px: 2.25, py: 1.75 }}>
          <IconTile concept="meal" />
          <Stack sx={{ flexGrow: 1, minWidth: 0 }}>
            <Typography variant="body2" sx={{ fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {model.title}
            </Typography>
            <Typography variant="caption" color="text.secondary" className="numeral" sx={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {model.caption}
            </Typography>
          </Stack>
          {model.chip ? <Chip size="small" label={model.chip} variant="outlined" /> : null}
          <Chevron />
        </Stack>
      </ButtonBase>
      {warning ? (
        <Stack direction="row" spacing={1} sx={{ alignItems: 'center', px: 2.25, pb: 1.5, color: 'warning.main' }}>
          <WarningIcon sx={{ fontSize: 17 }} />
          <Typography variant="caption">{warning}</Typography>
        </Stack>
      ) : null}
    </Paper>
  );
}

function AvatarStack({ meal }: { meal: PlannerMeal }) {
  const names = meal.people.map((person) => person.display_name);
  if (names.length === 0 && meal.owner_name) names.push(meal.owner_name);
  const shown = names.slice(0, 2);
  return (
    <Box sx={{ display: 'flex', width: shown.length > 1 ? 48 : 32, flexShrink: 0 }}>
      {shown.map((name, index) => (
        <Box key={`${name}-${index}`} sx={index === 0 ? undefined : { ml: -1, zIndex: 1, border: '2px solid', borderColor: 'common.white', borderRadius: '30%' }}>
          <InitialsAvatar name={name} size={32} />
        </Box>
      ))}
    </Box>
  );
}

export function OtherMealsRoster({
  meals,
  onSelect,
}: {
  meals: PlannerMeal[];
  onSelect: (meal: PlannerMeal) => void;
}) {
  return (
    <Paper sx={{ overflow: 'hidden' }}>
      <Stack divider={<Divider flexItem />}>
        {meals.map((meal) => (
          <ButtonBase key={meal.id} onClick={() => onSelect(meal)} sx={{ display: 'block', width: '100%', textAlign: 'left' }}>
            <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center', px: 2, py: 1.25 }}>
              <AvatarStack meal={meal} />
              <Stack sx={{ flexGrow: 1, minWidth: 0 }}>
                <Typography variant="body2" sx={{ fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {mealTitle(meal)}
                </Typography>
                <Typography variant="caption" color="text.secondary" sx={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {attendeeNames(meal)}
                </Typography>
              </Stack>
              <Chevron />
            </Stack>
          </ButtonBase>
        ))}
      </Stack>
    </Paper>
  );
}
