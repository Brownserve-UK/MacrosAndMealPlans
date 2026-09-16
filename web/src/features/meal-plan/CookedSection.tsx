import Button from '@mui/material/Button';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { ThingRow } from '../../components/ThingRow';
import { SlotSection } from './SlotSection';

export type CookedFoodToPutAway = {
  recipeId: string;
  name: string;
  servings: number;
};

function servingsLabel(servings: number): string {
  const value = Number.isInteger(servings) ? String(servings) : servings.toFixed(1);
  return servings === 1 ? '1 serving' : `${value} servings`;
}

export function CookedSection({
  items,
  onPutAway,
}: {
  items: CookedFoodToPutAway[];
  onPutAway: (item: CookedFoodToPutAway) => void;
}) {
  return (
    <SlotSection id="leftovers" title="Leftovers to put away">
      {items.length > 0 ? (
        <Stack spacing={1.5}>
          {items.map((item) => (
            <ThingRow
              key={item.recipeId}
              concept="dish"
              tone="secondary"
              title={item.name}
              caption={<span className="numeral">{servingsLabel(item.servings)}</span>}
              action={
                <Button variant="outlined" size="small" onClick={() => onPutAway(item)}>
                  Put away
                </Button>
              }
            />
          ))}
        </Stack>
      ) : (
        <Typography variant="body2" color="text.secondary">
          Nothing to put away.
        </Typography>
      )}
    </SlotSection>
  );
}
