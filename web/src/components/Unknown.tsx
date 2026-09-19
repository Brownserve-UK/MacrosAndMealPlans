import Box from '@mui/material/Box';
import Tooltip from '@mui/material/Tooltip';

export function Unknown({ label = 'Not known' }: { label?: string }) {
  return (
    <Tooltip title={label}>
      <Box
        component="span"
        aria-label={label}
        sx={{ color: 'text.disabled', userSelect: 'none' }}
      >
        &ndash;
      </Box>
    </Tooltip>
  );
}

export function MaybeNumber({
  value,
  suffix,
  fractionDigits = 1,
}: {
  value: number | null | undefined;
  suffix?: string;
  fractionDigits?: number;
}) {
  if (value === null || value === undefined) return <Unknown />;
  const formatted = Number.isInteger(value)
    ? value.toLocaleString('en-GB')
    : value.toLocaleString('en-GB', { maximumFractionDigits: fractionDigits });
  return (
    <>
      {formatted}
      {suffix ? <Box component="span" sx={{ color: 'text.secondary' }}>{suffix}</Box> : null}
    </>
  );
}
