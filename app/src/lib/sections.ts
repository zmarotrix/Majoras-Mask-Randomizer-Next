// Each settings section has its own colour, so you always know where you
// are: used for its sidebar icon, page heading, tabs and controls.

const COLORS: Record<string, string> = {
  quick: "#f5c451",
  logic: "#2fc4a2",
  items: "#f2a33a",
  dungeons: "#62c254",
  world: "#5aa7f2",
  hints: "#ee78b0",
  gameplay: "#ff7a4f",
  convenience: "#a58bff",
  cosmetics: "#e46fd6",
  "logic-editor": "#2fc4a2",
  tracker: "#5aa7f2",
  manual: "#f0a35a",
};

export function sectionColor(id: string): string {
  return COLORS[id] ?? "#9b7bff";
}
