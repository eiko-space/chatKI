/** Kleiner Helfer zum Zusammensetzen von CSS-Klassen. */
export function cn(...parts: Array<string | false | null | undefined>): string {
  return parts.filter(Boolean).join(" ");
}
