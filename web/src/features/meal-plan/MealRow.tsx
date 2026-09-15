import MoreHorizIcon from '@mui/icons-material/MoreHorizOutlined';
import WarningIcon from '@mui/icons-material/WarningAmberOutlined';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import IconButton from '@mui/material/IconButton';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import type { PlannerMeal } from '../../api/client';
import { IconTile } from '../../components/IconTile';
import { formatAmount } from './format';

export type MealAction = { label: string; onClick: () => void };

function servingsOf(meal: PlannerMeal): number | null {
  const recipe = meal.foods.find((food) => food.amount.kind === 'servings');
  if (!recipe) return null;
  return recipe.cooked ? recipe.cooked.servings_produced : recipe.amount.value;
}

export function mealDetail(meal: PlannerMeal): string | null {
  const recipe = meal.foods.find((food) => food.amount.kind === 'servings');
  if (recipe?.cooked) return null;
  const servings = servingsOf(meal);
  if (servings != null) {
    return servings === 1 ? 'about 1 serving' : `about ${servings} servings`;
  }
  return meal.foods.map((food) => formatAmount(food.amount)).join(' · ') || null;
}

export type MealRowChip = { key: string; label: string; muted?: boolean };

export type MealRowModel = {
  time?: string | null;
  title: string;
  chips: MealRowChip[];
  detail?: string | null;
  tag?: { label: string; strong?: boolean } | null;
};

export function plannerMealRow(meal: PlannerMeal): MealRowModel {
  const guests = meal.guest_groups.reduce((sum, group) => sum + group.count, 0);
  const cooked = meal.foods.find((food) => food.cooked)?.cooked;
  const chips: MealRowChip[] = meal.people.map((person) => ({
    key: person.member_id,
    label: person.display_name,
  }));
  if (guests > 0) chips.push({ key: 'guests', label: guests === 1 ? '1 guest' : `${guests} guests` });
  if (meal.opted_out.length > 0) {
    chips.push({
      key: 'opted-out',
      label: meal.opted_out.length === 1 ? 'Opted out' : `${meal.opted_out.length} opted out`,
      muted: true,
    });
  }

  return {
    time: meal.planned_time,
    title: meal.foods.map((food) => food.item_name).join(', ') || 'Nothing planned',
    chips,
    detail: mealDetail(meal),
    tag: meal.status === 'eaten'
      ? { label: 'Recorded' }
      : cooked
        ? { label: `Made ${cooked.servings_produced}`, strong: true }
        : meal.status === 'partially_resolved'
          ? { label: 'Partly recorded' }
          : null,
  };
}

export function MealRow({
  model,
  primary,
  secondary,
  extras = [],
  warning,
}: {
  model: MealRowModel;
  primary?: MealAction | null;
  secondary?: MealAction | null;
  extras?: MealAction[];
  warning?: string | null;
}) {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  const { time, title, chips, detail, tag } = model;

  return (
    <Paper sx={{ px: 2.25, py: 1.75 }}>
      <Stack direction="row" spacing={2} sx={{ alignItems: 'center' }}>
        <IconTile concept="meal" />

        <Stack sx={{ flexGrow: 1, minWidth: 0 }} spacing={0.75}>
          <Stack direction="row" spacing={1.25} sx={{ alignItems: 'baseline' }}>
            {time ? (
              <Typography variant="caption" color="text.secondary" className="numeral" sx={{ flexShrink: 0 }}>
                {time}
              </Typography>
            ) : null}
            <Typography variant="subtitle1" sx={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {title}
            </Typography>
          </Stack>

          <Stack direction="row" sx={{ flexWrap: 'wrap', gap: 0.5, alignItems: 'center' }}>
            {chips.map((chip) => (
              <Chip
                key={chip.key}
                size="small"
                variant="outlined"
                label={chip.label}
                sx={chip.muted ? { borderStyle: 'dashed', color: 'text.disabled' } : undefined}
              />
            ))}
            {detail ? (
              <Typography variant="caption" color="text.secondary" className="numeral" sx={{ ml: 0.5 }}>
                {detail}
              </Typography>
            ) : null}
          </Stack>
        </Stack>

        {tag ? (
          <Chip
            size="small"
            label={tag.label}
            variant={tag.strong ? 'filled' : 'outlined'}
            color={tag.strong ? 'success' : 'default'}
            sx={{ flexShrink: 0 }}
          />
        ) : null}

        {secondary ? (
          <Button size="small" onClick={secondary.onClick} sx={{ flexShrink: 0 }}>
            {secondary.label}
          </Button>
        ) : null}

        {primary ? (
          <Button variant="contained" size="small" onClick={primary.onClick} sx={{ flexShrink: 0 }}>
            {primary.label}
          </Button>
        ) : null}

        {extras.length > 0 ? (
          <>
            <IconButton
              size="small"
              aria-label={`More for ${title}`}
              onClick={(event) => setAnchor(event.currentTarget)}
            >
              <MoreHorizIcon fontSize="small" />
            </IconButton>
            <Menu anchorEl={anchor} open={Boolean(anchor)} onClose={() => setAnchor(null)}>
              {extras.map((extra) => (
                <MenuItem
                  key={extra.label}
                  onClick={() => {
                    setAnchor(null);
                    extra.onClick();
                  }}
                >
                  {extra.label}
                </MenuItem>
              ))}
            </Menu>
          </>
        ) : null}
      </Stack>

      {warning ? (
        <Stack direction="row" spacing={1} sx={{ alignItems: 'center', mt: 1.25, color: 'warning.main' }}>
          <WarningIcon fontSize="small" />
          <Typography variant="caption">{warning}</Typography>
        </Stack>
      ) : null}

      <Box />
    </Paper>
  );
}
