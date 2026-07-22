import { useCallback, useEffect, useMemo, useState } from "react";

import { invokeTauri, isTauriRuntime } from "../../shared/bridge/tauri";
import {
  initialDesktopLyricsWindowSnapshot,
  type DesktopLyricsWindowFailure,
  type DesktopLyricsWindowSnapshot,
} from "../../shared/model/desktopLyrics";

type DesktopLyricsCommand =
  | "fit_desktop_lyrics_window"
  | "hide_desktop_lyrics_window"
  | "lock_desktop_lyrics_window"
  | "show_desktop_lyrics_window"
  | "unlock_desktop_lyrics_window";

export function useDesktopLyricsWindow() {
  const preview = !isTauriRuntime();
  const [snapshot, setSnapshot] = useState<DesktopLyricsWindowSnapshot>(
    initialDesktopLyricsWindowSnapshot,
  );
  const [error, setError] = useState<DesktopLyricsWindowFailure | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    if (preview) return;
    try {
      const next = await invokeTauri<DesktopLyricsWindowSnapshot>(
        "get_desktop_lyrics_window_state",
      );
      setSnapshot((current) => sameSnapshot(current, next) ? current : next);
      setError(null);
    } catch (nextError) {
      setError(toDesktopLyricsFailure(nextError));
    }
  }, [preview]);

  useEffect(() => {
    void refresh();
    if (preview) return;
    const timer = window.setInterval(() => void refresh(), 1_500);
    return () => window.clearInterval(timer);
  }, [preview, refresh]);

  const run = useCallback(async (
    command: DesktopLyricsCommand,
    args?: Record<string, unknown>,
  ): Promise<boolean> => {
    setBusy(true);
    try {
      if (preview) {
        setSnapshot((current) => previewCommand(current, command));
        setError(null);
        return true;
      }
      const next = await invokeTauri<DesktopLyricsWindowSnapshot>(command, args);
      setSnapshot(next);
      setError(null);
      return true;
    } catch (nextError) {
      setError(toDesktopLyricsFailure(nextError));
      await refresh();
      return false;
    } finally {
      setBusy(false);
    }
  }, [preview, refresh]);

  return useMemo(
    () => ({ busy, error, refresh, run, snapshot }),
    [busy, error, refresh, run, snapshot],
  );
}

function sameSnapshot(
  left: DesktopLyricsWindowSnapshot,
  right: DesktopLyricsWindowSnapshot,
) {
  return left.supported === right.supported
    && left.visible === right.visible
    && left.locked === right.locked;
}

function previewCommand(
  snapshot: DesktopLyricsWindowSnapshot,
  command: DesktopLyricsCommand,
  _args?: Record<string, unknown>,
): DesktopLyricsWindowSnapshot {
  if (command === "show_desktop_lyrics_window") {
    return { ...snapshot, visible: true, locked: false };
  }
  if (command === "hide_desktop_lyrics_window") {
    return { ...snapshot, visible: false, locked: false };
  }
  if (command === "lock_desktop_lyrics_window") {
    return snapshot.visible ? { ...snapshot, locked: true } : snapshot;
  }
  if (command === "unlock_desktop_lyrics_window") {
    return { ...snapshot, locked: false };
  }
  return { ...snapshot, locked: false };
}

function toDesktopLyricsFailure(error: unknown): DesktopLyricsWindowFailure {
  if (typeof error === "object" && error !== null) {
    const candidate = error as { code?: unknown; message?: unknown };
    if (typeof candidate.code === "string" && typeof candidate.message === "string") {
      return { code: candidate.code, message: candidate.message };
    }
  }
  return { code: "desktop_lyrics_failed", message: String(error) };
}
