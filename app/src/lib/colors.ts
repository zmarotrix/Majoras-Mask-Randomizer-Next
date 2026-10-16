// Colours in settings use MMR 2.0's "R, G, B" strings.

export function toHex(rgb: unknown): string {
  const parts = String(rgb ?? "")
    .split(",")
    .map((p) => Number(p.trim()));
  if (parts.length !== 3 || parts.some((n) => !Number.isInteger(n) || n < 0 || n > 255)) return "#000000";
  return "#" + parts.map((n) => n.toString(16).padStart(2, "0")).join("");
}

export function fromHex(hex: string): string {
  const n = parseInt(hex.replace("#", ""), 16);
  return `${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}`;
}

export function randomColor(): string {
  const c = () => Math.floor(Math.random() * 256);
  return `${c()}, ${c()}, ${c()}`;
}

/** "FierceDeity" -> "Fierce Deity". */
export function formName(form: string): string {
  return form.replace(/([a-z])([A-Z])/g, "$1 $2");
}

/** Deep copy of a setting value, for editing. */
export function copy<T>(v: T): T {
  return v == null ? v : (JSON.parse(JSON.stringify(v)) as T);
}
