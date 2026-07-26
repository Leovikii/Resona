export function parseNumberPreference(value: string | null, fallback: number): number {
  if (value === null || value.trim() === "") return fallback;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

export function normalizeSteppedPreference(
  value: number,
  min: number,
  max: number,
  step: number,
  fallback: number,
): number {
  if (!Number.isFinite(value) || step <= 0) return fallback;
  const clamped = Math.min(max, Math.max(min, value));
  return Math.min(max, Math.max(min, min + Math.round((clamped - min) / step) * step));
}
