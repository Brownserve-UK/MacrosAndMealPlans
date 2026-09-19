import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { CookingItem } from '../../../api/client';
import { CookingItemRow } from './CookingItemRow';
import { formatMinutes } from './plannerWeek';
import type { OccasionView, PlannerMember } from './types';

function cookingItemKey(item: CookingItem): string {
  const withKind = item as CookingItem & Record<string, string>;
  const idField = ['product_id', 'recipe_id', 'dish_recipe_id', 'ingredient_id', 'prepared_meal_id'].find(
    (field) => typeof withKind[field] === 'string',
  );
  const id = idField ? withKind[idField] : item.name;
  return `${item.item_kind}:${id}`;
}

export function CookingList({ occasion, members }: { occasion: OccasionView; members: PlannerMember[] }) {
  if (occasion.cooking.length === 0) return null;
  const longest = occasion.cooking.reduce<number | null>((max, item) => {
    if (item.cook_minutes == null) return max;
    return max == null ? item.cook_minutes : Math.max(max, item.cook_minutes);
  }, null);
  const showFor = occasion.groups.length > 1;

  return (
    <Stack>
      <Stack direction="row" sx={{ justifyContent: 'space-between', alignItems: 'baseline', pb: 1 }}>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{ letterSpacing: '0.06em', textTransform: 'uppercase', fontWeight: 600 }}
        >
          Cooking
        </Typography>
        {longest != null ? (
          <Typography variant="caption" color="text.secondary" className="numeral">
            {formatMinutes(longest)}
          </Typography>
        ) : null}
      </Stack>
      {occasion.cooking.map((item) => (
        <CookingItemRow key={cookingItemKey(item)} item={item} members={members} showFor={showFor} />
      ))}
    </Stack>
  );
}
