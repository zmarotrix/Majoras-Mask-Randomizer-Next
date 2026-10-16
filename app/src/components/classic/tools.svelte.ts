// Which classic-layout tool window is open (Customize menu and buttons).

export type ClassicTool = "hud" | "dpad" | "hints" | "random-starting";

class Tools {
  open = $state<ClassicTool | null>(null);
}

export const tools = new Tools();

export function openTool(tool: ClassicTool) {
  tools.open = tool;
}
