import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { useTheme } from '@mui/material/styles';
import { useState } from 'react';
import type { WeightDisplay, WeightPoint } from '../../api/client';
import { formatWeight } from './weightFormat';

const WIDTH = 640;
const HEIGHT = 220;
const PADDING = { top: 16, right: 16, bottom: 28, left: 50 };

type Props = { points: WeightPoint[]; goalKg?: number | null; display: WeightDisplay };

function dayNumber(iso: string) {
  return Math.round(new Date(`${iso}T00:00:00Z`).getTime() / 86_400_000);
}
function formatDate(iso: string) {
  return new Date(`${iso}T00:00:00`).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' });
}
function niceStep(range: number) {
  return [0.5, 1, 2, 5, 10, 20, 50].find((step) => range / step <= 5) ?? 100;
}

function smoothPath(coordinates: { x: number; y: number }[]) {
  const first = coordinates[0];
  if (!first) return '';
  return coordinates.slice(1).reduce((path, point, index) => {
    const previous = coordinates[index] as { x: number; y: number };
    const middle = (previous.x + point.x) / 2;
    return `${path} C ${middle} ${previous.y}, ${middle} ${point.y}, ${point.x} ${point.y}`;
  }, `M ${first.x} ${first.y}`);
}

export function WeightChart({ points, goalKg, display }: Props) {
  const theme = useTheme();
  const palette = (theme.vars ?? theme).palette;
  const [selected, setSelected] = useState<number | null>(null);
  const first = points[0];
  const last = points[points.length - 1];
  if (!first || !last) {
    return (
      <Typography variant="body2" color="text.secondary">
        Add your first weigh-in to see your trend.
      </Typography>
    );
  }

  const startDay = dayNumber(first.on);
  const lastDay = dayNumber(last.on);
  const daySpan = Math.max(lastDay - startDay, 1);

  const weights = goalKg == null ? points.map((point) => point.weight_kg) : [...points.map((point) => point.weight_kg), goalKg];
  const dataLow = Math.min(...weights);
  const dataHigh = Math.max(...weights);
  const pad = Math.max((dataHigh - dataLow) * 0.2, 1);
  const low = dataLow - pad;
  const high = dataHigh + pad;
  const step = niceStep(high - low);

  const plotWidth = WIDTH - PADDING.left - PADDING.right;
  const plotHeight = HEIGHT - PADDING.top - PADDING.bottom;
  const x = (iso: string) => PADDING.left + ((dayNumber(iso) - startDay) / daySpan) * plotWidth;
  const y = (kg: number) => PADDING.top + ((high - kg) / (high - low)) * plotHeight;
  const coordinates = points.map((point) => ({ x: x(point.on), y: y(point.weight_kg) }));
  const line = smoothPath(coordinates);
  const area = `${line} L ${coordinates.at(-1)?.x ?? PADDING.left} ${HEIGHT - PADDING.bottom} L ${PADDING.left} ${HEIGHT - PADDING.bottom} Z`;

  const ticks: number[] = [];
  for (let value = Math.ceil(low / step) * step; value <= high + step / 2; value += step) {
    ticks.push(Number(value.toFixed(2)));
  }

  const selectedPoint = selected == null ? null : points[selected];
  const selectedCoordinate = selected == null ? null : coordinates[selected];
  const change = last.weight_kg - first.weight_kg;
  const label = `Weight ${change === 0 ? 'unchanged' : change < 0 ? 'down' : 'up'} from ${formatWeight(first.weight_kg, display)} on ${formatDate(first.on)} to ${formatWeight(last.weight_kg, display)} on ${formatDate(last.on)}`;

  return (
    <Box role="img" aria-label={label} sx={{ width: '100%' }} onClick={() => setSelected(null)}>
      <Box component="svg" viewBox={`0 0 ${WIDTH} ${HEIGHT}`} sx={{ display: 'block', width: '100%', height: 'auto', overflow: 'visible' }}>
        {ticks.map((tick) => (
          <g key={tick}>
            <Box component="line" x1={PADDING.left} x2={WIDTH - PADDING.right} y1={y(tick)} y2={y(tick)} stroke={palette.divider} strokeWidth={1} />
            <Box component="text" x={PADDING.left - 10} y={y(tick) + 4} textAnchor="end" fill={palette.text.secondary} fontSize={12} sx={{ fontVariantNumeric: 'tabular-nums' }}>
              {tick.toLocaleString('en-GB')} kg
            </Box>
          </g>
        ))}
        {goalKg != null ? (
          <Box component="line" x1={PADDING.left} x2={WIDTH - PADDING.right} y1={y(goalKg)} y2={y(goalKg)} stroke={palette.success.main} strokeWidth={1.5} strokeDasharray="6 5" />
        ) : null}
        <Box component="path" d={area} fill={palette.primary.main} sx={{ opacity: 0.1 }} />
        <Box component="path" d={line} fill="none" stroke={palette.primary.main} strokeWidth={3} strokeLinecap="round" />
        {points.map((point, index) => (
          <Box
            key={`${point.on}-${point.weight_kg}`}
            component="circle"
            role="button"
            tabIndex={0}
            aria-label={`${formatWeight(point.weight_kg, display)} on ${formatDate(point.on)}`}
            cx={x(point.on)}
            cy={y(point.weight_kg)}
            r={selected === index ? 6 : 4}
            onClick={(event) => {
              event.stopPropagation();
              setSelected((current) => (current === index ? null : index));
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                setSelected((current) => (current === index ? null : index));
              }
            }}
            fill={palette.background.paper}
            stroke={palette.primary.main}
            strokeWidth={2}
            sx={{ cursor: 'pointer', outline: 'none' }}
          />
        ))}
        {selectedPoint && selectedCoordinate ? (
          <g pointerEvents="none">
            <Box
              component="rect"
              x={Math.min(Math.max(selectedCoordinate.x - 65, PADDING.left), WIDTH - 150)}
              y={Math.max(selectedCoordinate.y - 58, 4)}
              width={130}
              height={42}
              rx={8}
              fill={palette.background.paper}
              stroke={palette.divider}
            />
            <Box
              component="text"
              x={Math.min(Math.max(selectedCoordinate.x, PADDING.left + 65), WIDTH - 85)}
              y={Math.max(selectedCoordinate.y - 38, 24)}
              textAnchor="middle"
              fill={palette.text.primary}
              fontSize={13}
              fontWeight={600}
              sx={{ fontVariantNumeric: 'tabular-nums' }}
            >
              {formatWeight(selectedPoint.weight_kg, display)}
            </Box>
            <Box
              component="text"
              x={Math.min(Math.max(selectedCoordinate.x, PADDING.left + 65), WIDTH - 85)}
              y={Math.max(selectedCoordinate.y - 22, 40)}
              textAnchor="middle"
              fill={palette.text.secondary}
              fontSize={11}
            >
              {formatDate(selectedPoint.on)}
            </Box>
          </g>
        ) : null}
      </Box>
      <StackLabels first={first.on} last={last.on} goalKg={goalKg} display={display} />
    </Box>
  );
}

function StackLabels({ first, last, goalKg, display }: { first: string; last: string; goalKg?: number | null; display: WeightDisplay }) {
  return (
    <Box sx={{ display: 'flex', justifyContent: 'space-between', mt: 0.5 }}>
      <Typography className="numeral" variant="caption" color="text.secondary">
        {formatDate(first)}
      </Typography>
      {goalKg != null ? (
        <Typography className="numeral" variant="caption" color="success">
          Goal {formatWeight(goalKg, display)}
        </Typography>
      ) : null}
      <Typography className="numeral" variant="caption" color="text.secondary">
        {formatDate(last)}
      </Typography>
    </Box>
  );
}
