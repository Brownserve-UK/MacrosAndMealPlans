export function toIsoDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

export function todayIso(): string {
  return toIsoDate(new Date());
}

function parseIsoDate(iso: string): Date {
  const parts = iso.split('-').map(Number);
  const [year = 1970, month = 1, day = 1] = parts;
  return new Date(year, month - 1, day);
}

export function addDays(iso: string, delta: number): string {
  const date = parseIsoDate(iso);
  date.setDate(date.getDate() + delta);
  return toIsoDate(date);
}

export function formatDayLabel(iso: string): string {
  const date = parseIsoDate(iso);
  if (iso === todayIso()) return 'Today';
  if (iso === addDays(todayIso(), -1)) return 'Yesterday';
  if (iso === addDays(todayIso(), 1)) return 'Tomorrow';
  return date.toLocaleDateString('en-GB', { weekday: 'short', day: 'numeric', month: 'short' });
}

export function formatFullDate(iso: string): string {
  return parseIsoDate(iso).toLocaleDateString('en-GB', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  });
}

function formatTimeOfDay(date: Date): string {
  const hours = String(date.getHours()).padStart(2, '0');
  const minutes = String(date.getMinutes()).padStart(2, '0');
  return `${hours}:${minutes}`;
}

export function nowTime(): string {
  return formatTimeOfDay(new Date());
}

export function extractTime(iso: string): string {
  return formatTimeOfDay(new Date(iso));
}

export function combineDateTime(date: string, time: string): string {
  return new Date(`${date}T${time}`).toISOString();
}
