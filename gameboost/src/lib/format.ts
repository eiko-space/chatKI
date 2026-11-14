/** Zahl im deutschen Format, `null` bleibt `null` (→ „—“ in der Anzeige). */
export function num(v: number | null | undefined, digits = 0): string | null {
  if (v === null || v === undefined || !Number.isFinite(v)) return null;
  return v.toLocaleString("de-DE", { minimumFractionDigits: digits, maximumFractionDigits: digits });
}
