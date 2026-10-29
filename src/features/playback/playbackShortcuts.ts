import type { PlaybackSnapshot } from "../../shared/model/playback";

export type PlaybackControl = "toggle" | "seek_backward" | "seek_forward" | "volume_up" | "volume_down";

export const interactiveSelector = 'input,textarea,select,button,a[href],summary,[contenteditable]:not([contenteditable="false"]),[role="slider"],[role="spinbutton"],[role="button"],[role="option"],[role="listbox"],[role="combobox"],[role="tab"],[role="menuitem"],[role="treeitem"],[role="checkbox"],[role="switch"],[role="radio"]';

export function playbackControlForKey(event: Pick<KeyboardEvent, "key" | "altKey" | "ctrlKey" | "metaKey" | "shiftKey" | "isComposing" | "defaultPrevented" | "repeat">): PlaybackControl | null {
  if (event.defaultPrevented || event.isComposing || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return null;
  if (event.key === " ") return event.repeat ? null : "toggle";
  return ({ ArrowLeft: "seek_backward", ArrowRight: "seek_forward", ArrowUp: "volume_up", ArrowDown: "volume_down" } as const)[event.key as "ArrowLeft"] ?? null;
}

// At most one IPC in flight; repeated directions are coalesced and bounded.
export function createControlQueue<T>(run: (action: PlaybackControl, steps: number) => Promise<T>, onResult: (action: PlaybackControl, result: T) => void, onError: (error: unknown) => void) {
  type Entry = { action: PlaybackControl; steps: number; repeat: boolean };
  let pending: Entry[] = [];
  let running = false;
  let disposed = false;
  async function drain() {
    if (running || disposed) return;
    running = true;
    try {
      while (pending.length && !disposed) {
        const next = pending.shift()!;
        try {
          const result = await run(next.action, next.steps);
          if (!disposed) onResult(next.action, result);
        } catch (error) { if (!disposed) onError(error); }
      }
    } finally { running = false; }
  }
  return {
    push(action: PlaybackControl, repeat: boolean) {
      if (disposed) return;
      const last = pending.at(-1);
      if (repeat && last?.repeat && last.action === action) last.steps = Math.min(20, last.steps + 1);
      else if (pending.length < 8) pending.push({ action, repeat, steps: 1 });
      void drain();
    },
    release(action: PlaybackControl) { pending = pending.filter((entry) => !entry.repeat || entry.action !== action); },
    clear() { pending = []; },
    dispose() { disposed = true; pending = []; },
  };
}

// Development fixture only; production relative state belongs to the Rust actor.
export function previewPlaybackControl(snapshot: PlaybackSnapshot, action: PlaybackControl, steps = 1): PlaybackSnapshot {
  const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));
  steps = clamp(steps, 1, 20);
  if (action === "volume_up" || action === "volume_down") return { ...snapshot, volume: clamp(snapshot.volume + (action === "volume_up" ? .1 : -.1) * steps, 0, 1) };
  if (action === "toggle" && (snapshot.status === "playing" || snapshot.status === "paused")) return { ...snapshot, status: snapshot.status === "playing" ? "paused" : "playing" };
  if ((action === "seek_forward" || action === "seek_backward") && snapshot.seekable && snapshot.durationMs !== null && (snapshot.status === "playing" || snapshot.status === "paused")) return { ...snapshot, positionMs: clamp(snapshot.positionMs + (action === "seek_forward" ? 5000 : -5000) * steps, 0, snapshot.durationMs) };
  return snapshot;
}
