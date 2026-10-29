import { getCurrentWebview } from "@tauri-apps/api/webview";
import type { Event } from "@tauri-apps/api/event";

type Position = { x: number; y: number };
type FileDragDrop =
  | { type: "enter" | "drop"; paths: string[]; position: Position }
  | { type: "over"; position: Position }
  | { type: "leave" };

// The native adapter preserves remote URIs and supplies logical widget coordinates.
export function onFileDragDropEvent(handler: (event: Event<FileDragDrop>) => void) {
  return getCurrentWebview().listen<FileDragDrop>("resona://file-drag-drop", handler);
}
