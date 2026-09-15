import { useHouseholdSettings } from '../api/queries';

export function useHouseholdTimeZone(): string | undefined {
  const settings = useHouseholdSettings();
  return settings.data?.timezone;
}
