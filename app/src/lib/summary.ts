// One-line summaries of a set of settings, for preset cards.

import type { SettingsSchema, Values } from "./types";

export function summarize(values: Values, schema: SettingsSchema): string[] {
  const opt = (key: string) => {
    const v = String(values[key]);
    return schema.layout.optionLabels?.[key]?.[v] ?? schema.settings.find((s) => s.key === key)?.options?.find((o) => o.value === v)?.label ?? v;
  };
  const count = (key: string) => (Array.isArray(values[key]) ? (values[key] as unknown[]).length : 0);
  const parts = [
    `${opt("LogicMode")} logic`,
    `${count("CustomItemList")} shuffled locations`,
  ];
  const tricks = count("EnabledTricks");
  if (tricks) parts.push(`${tricks} trick${tricks === 1 ? "" : "s"}`);
  const start = count("CustomStartingItemList");
  if (start) parts.push(`${start} starting items`);
  for (const key of ["SmallKeyMode", "BossKeyMode", "EntranceMode"]) {
    const v = values[key];
    const labels = schema.layout.optionLabels?.[key] ?? {};
    if (Array.isArray(v) && v.length) parts.push(`${schema.layout.labels[key] ?? key}: ${v.map((x) => labels[x] ?? x).join(", ")}`);
  }
  return parts;
}
