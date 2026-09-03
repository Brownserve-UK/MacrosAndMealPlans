import Paper from '@mui/material/Paper';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import type { DemandClaim, Quantity } from '../../api/client';
import { displayUnit } from '../../components/UnitSelect';
import { labelForSlot } from '../meal-plan/slots';

export function formatQuantity(quantity: Quantity): string {
  const amount = Number.isInteger(quantity.amount)
    ? quantity.amount.toLocaleString('en-GB')
    : quantity.amount.toLocaleString('en-GB', { maximumFractionDigits: 2 });
  return `${amount} ${displayUnit(quantity.unit)}`;
}

function whenLabel(claim: DemandClaim): string {
  const date = new Date(`${claim.planned_on}T00:00:00`).toLocaleDateString('en-GB', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  });
  return `${labelForSlot(claim.slot)} · ${date}`;
}

function ClaimRow({ claim }: { claim: DemandClaim }) {
  return (
    <Stack
      direction="row"
      spacing={2}
      sx={{ alignItems: 'baseline', justifyContent: 'space-between', py: 0.75 }}
    >
      <Stack spacing={0.25} sx={{ minWidth: 0 }}>
        <Typography variant="body2" sx={{ fontWeight: 600 }} noWrap>
          {claim.recipe_name ?? whenLabel(claim)}
        </Typography>
        {claim.recipe_name ? (
          <Typography variant="caption" color="text.secondary" noWrap>
            {whenLabel(claim)}
          </Typography>
        ) : null}
      </Stack>
      <Typography variant="body2" className="numeral" sx={{ flexShrink: 0 }}>
        {formatQuantity(claim.quantity)}
      </Typography>
    </Stack>
  );
}

function Group({
  title,
  description,
  claims,
}: {
  title: string;
  description: string;
  claims: DemandClaim[];
}) {
  if (claims.length === 0) return null;
  return (
    <Stack spacing={0.5}>
      <Typography variant="overline" color="text.secondary">
        {title}
      </Typography>
      <Typography variant="body2" color="text.secondary">
        {description}
      </Typography>
      <Stack divider={<div />} sx={{ mt: 0.5 }}>
        {claims.map((claim, index) => (
          <ClaimRow key={`${claim.entry_id}-${index}`} claim={claim} />
        ))}
      </Stack>
    </Stack>
  );
}

export function shareSentence(
  shared: DemandClaim[],
  expectedFromHere: Quantity | null,
  alternatives: string[],
): string | null {
  if (shared.length === 0) return null;
  const unit = shared[0]?.quantity.unit;
  if (!unit || shared.some((claim) => claim.quantity.unit !== unit)) return null;
  const total = shared.reduce((sum, claim) => sum + claim.quantity.amount, 0);
  const expected = expectedFromHere?.unit === unit ? expectedFromHere.amount : 0;
  const elsewhere = alternatives.length > 0 ? ` It can come from ${listOut(alternatives)}.` : '';

  if (expected <= 0) {
    return `None of that is expected to come from here, because this product's own planned meals already account for what it has.${elsewhere}`;
  }
  if (expected >= total) {
    return 'We expect all of that to come from here.';
  }
  return `We expect ${formatQuantity({ amount: expected, unit })} of that to come from here. The rest can come from ${listOut(alternatives)}.`;
}

export function SpokenFor({
  pinned,
  shared,
  ingredientName,
  expectedFromHere,
  alternatives,
}: {
  pinned: DemandClaim[];
  shared: DemandClaim[];
  ingredientName: string | null;
  expectedFromHere: Quantity | null;
  alternatives: string[];
}) {
  if (pinned.length === 0 && shared.length === 0) return null;

  const sharedDescription = ingredientName
    ? `These asked for ${ingredientName} rather than this product, so they can be met from any of it you have.`
    : 'These asked for the ingredient rather than this product.';
  const share = shareSentence(shared, expectedFromHere, alternatives);

  return (
    <Paper variant="outlined" sx={{ p: 3, mb: 3 }}>
      <Typography variant="h2" sx={{ mb: 2 }}>
        Spoken for
      </Typography>
      <Stack spacing={3}>
        <Group
          title="Planned for this product"
          description="Meals that asked for this product by name. They cannot be met with anything else."
          claims={pinned}
        />
        <Group title="Could come from here or elsewhere" description={sharedDescription} claims={shared} />
        {share ? (
          <Typography variant="body2" color="text.secondary">
            {share}
          </Typography>
        ) : null}
      </Stack>
    </Paper>
  );
}

function listOut(names: string[]): string {
  const last = names[names.length - 1] ?? '';
  if (names.length === 1) return last;
  return `${names.slice(0, -1).join(', ')} or ${last}`;
}
