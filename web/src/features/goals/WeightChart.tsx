import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import type { WeightDisplay, WeightPoint } from '../../api/client';
import { formatWeight } from './weightFormat';

const WIDTH = 640;
const HEIGHT = 220;
const PADDING = { top: 14, right: 14, bottom: 24, left: 14 };
const MIN_SPAN_KG = 1;

type Props = {
  points: WeightPoint[];
  goalKg?: number | null;
  display: WeightDisplay;
};

function dayNumber(iso: string): number {
  return Math.round(new Date(`${iso}T00:00:00Z`).getTime() / 86_400_000);
}


function formatDate(iso: string): string {
  return new Date(`${iso}T00:00:00`).toLocaleDateString('en-GB', {
    day: 'numeric',
    month: 'short',
  });
}

export function WeightChart({ points, goalKg, display }: Props) {
  const first = points[0];
  const last = points[points.length - 1];

  if (points.length < 2 || !first || !last) {
    return (
      <Typography variant="body2" color="text.secondary">
        Not enough data. Weigh in again to see your trend.
      </Typography>
    );
  }

  const startDay = dayNumber(first.on);
  const endDay = dayNumber(last.on);
  const daySpan = Math.max(endDay - startDay, 1);

  const weights = points.map((point) => point.weight_kg);
  const candidates =
    goalKg != null && Math.abs(goalKg - last.weight_kg) < 25 ? [...weights, goalKg] : weights;
  let low = Math.min(...candidates);
  let high = Math.max(...candidates);
  if (high - low < MIN_SPAN_KG) {
    const middle = (high + low) / 2;
    low = middle - MIN_SPAN_KG / 2;
    high = middle + MIN_SPAN_KG / 2;
  }
  const pad = (high - low) * 0.15;
  low -= pad;
  high += pad;

  const plotWidth = WIDTH - PADDING.left - PADDING.right;
  const plotHeight = HEIGHT - PADDING.top - PADDING.bottom;
  const x = (iso: string) =>
    PADDING.left + ((dayNumber(iso) - startDay) / daySpan) * plotWidth;
  const y = (kg: number) =>
    PADDING.top + ((high - kg) / (high - low)) * plotHeight;

  const line = points.map((point) => `${x(point.on)},${y(point.weight_kg)}`).join(' ');
  const goalY = goalKg == null ? null : y(goalKg);
  const goalOnChart = goalY != null && goalY >= PADDING.top && goalY <= PADDING.top + plotHeight;

  const change = last.weight_kg - first.weight_kg;
  const direction = change === 0 ? 'unchanged' : change < 0 ? 'down' : 'up';
  const label =
    `Weight ${direction} from ${formatWeight(first.weight_kg, display)} on ${formatDate(first.on)} ` +
    `to ${formatWeight(last.weight_kg, display)} on ${formatDate(last.on)}`;

  return (
    <Box>
      <Box role="img" aria-label={label} sx={{ width: '100%' }}>
        <Box
          component="svg"
          viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
          aria-hidden
          sx={{ display: 'block', width: '100%', height: 'auto', overflow: 'visible' }}
        >
          {goalOnChart ? (
            <Box
              component="line"
              x1={PADDING.left}
              x2={WIDTH - PADDING.right}
              y1={goalY}
              y2={goalY}
              sx={{
                color: 'success.main',
                stroke: 'currentColor',
                strokeWidth: 1.5,
                strokeDasharray: '5 5',
              }}
            />
          ) : null}

          <Box
            component="polyline"
            points={line}
            sx={{
              color: 'primary.main',
              fill: 'none',
              stroke: 'currentColor',
              strokeWidth: 2.5,
              strokeLinecap: 'round',
              strokeLinejoin: 'round',
            }}
          />

          {points.length <= 30
            ? points.map((point) => (
                <Box
                  key={`${point.on}-${point.weight_kg}`}
                  component="circle"
                  cx={x(point.on)}
                  cy={y(point.weight_kg)}
                  r={3}
                  sx={{ color: 'primary.main', fill: 'currentColor' }}
                />
              ))
            : null}
        </Box>
      </Box>

      <Box sx={{ display: 'flex', justifyContent: 'space-between', mt: 0.5 }}>
        <Typography variant="caption" color="text.secondary">
          {formatDate(first.on)}
        </Typography>
        {goalOnChart ? (
          <Typography variant="caption" color="success.main">
            Goal {formatWeight(goalKg as number, display)}
          </Typography>
        ) : null}
        <Typography variant="caption" color="text.secondary">
          {formatDate(last.on)}
        </Typography>
      </Box>
    </Box>
  );
}
