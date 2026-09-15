export type Concept = 'meal' | 'cook' | 'dish' | 'recipe' | 'food' | 'fridge' | 'freezer';

const PATHS: Record<Concept, string[]> = {
  meal: ['M6 3v8a2.5 2.5 0 005 0V3M8.5 11v10', 'M17 3c-1.5 2-2 3.5-2 5.5S16 12 17 12s2-1.5 2-3.5S18.5 5 17 3zM17 12v9'],
  cook: ['M5 10.5a7 7 0 0114 0z', 'M3.5 14h17M6 17.5h12'],
  dish: ['M4 11.5a8 8 0 0116 0z', 'M2.5 15h19'],
  recipe: ['M5 5a2 2 0 012-2h12v18H7a2 2 0 01-2-2z', 'M19 17H7a2 2 0 00-2 2'],
  food: ['M5 8h14v11a2 2 0 01-2 2H7a2 2 0 01-2-2z', 'M5 12h14M9 8V4h6v4'],
  fridge: ['M4 3h16a1 1 0 011 1v16a1 1 0 01-1 1H4a1 1 0 01-1-1V4a1 1 0 011-1z', 'M3 10h18M8 6.5v1.5M8 13v2'],
  freezer: ['M12 3v18', 'M5 7l14 10M19 7L5 17'],
};

export function ConceptIcon({ concept, size = 17 }: { concept: Concept; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.9}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
      focusable="false"
    >
      {PATHS[concept].map((path) => (
        <path key={path} d={path} />
      ))}
    </svg>
  );
}
