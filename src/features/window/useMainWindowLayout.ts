import { useCallback, useEffect, useState } from "react";

import { invokeTauri, isTauriRuntime } from "../../shared/bridge/tauri";
import { initializeCurrentWindowMaterial } from "../../shared/bridge/windowAppearance";
import type {
  MainWindowFailure,
  MainWindowLayoutMode,
  MainWindowSnapshot,
} from "../../shared/model/mainWindow";

function initialLayoutMode(): MainWindowLayoutMode {
  const requested = new URLSearchParams(window.location.search).get("layout");
  if (requested === "wide" || requested === "compact") return requested;
  return window.matchMedia("(max-width: 600px)").matches ? "compact" : "wide";
}

export function useMainWindowLayout() {
  const preview = import.meta.env.DEV && !isTauriRuntime();
  const [snapshot, setSnapshot] = useState<MainWindowSnapshot>(() => ({
    layoutMode: initialLayoutMode(),
  }));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<MainWindowFailure | null>(null);
  const [initialized, setInitialized] = useState(preview);

  useEffect(() => {
    if (preview) return;
    void Promise.all([
      invokeTauri<MainWindowSnapshot>("get_main_window_state"),
      initializeCurrentWindowMaterial(),
    ])
      .then(([next]) => {
        setSnapshot(next);
        setError(null);
      })
      .catch((cause) => setError(toFailure(cause)))
      .finally(() => setInitialized(true));
  }, [preview]);

  useEffect(() => {
    if (preview || !initialized) return;
    void invokeTauri<MainWindowSnapshot>("main_window_ready")
      .then((next) => setSnapshot(next))
      .catch((cause) => setError(toFailure(cause)));
  }, [initialized, preview]);

  const setLayoutMode = useCallback(async (mode: MainWindowLayoutMode) => {
    if (mode === snapshot.layoutMode) return true;
    setBusy(true);
    try {
      const next = preview
        ? { layoutMode: mode }
        : await invokeTauri<MainWindowSnapshot>("set_main_window_layout_mode", { mode });
      setSnapshot(next);
      setError(null);
      return true;
    } catch (cause) {
      setError(toFailure(cause));
      return false;
    } finally {
      setBusy(false);
    }
  }, [preview, snapshot.layoutMode]);

  return { busy, error, setLayoutMode, snapshot };
}

function toFailure(cause: unknown): MainWindowFailure {
  if (cause && typeof cause === "object") {
    const candidate = cause as Partial<MainWindowFailure>;
    if (typeof candidate.code === "string" && typeof candidate.message === "string") {
      return { code: candidate.code, message: candidate.message };
    }
  }
  return { code: "main_window_failed", message: String(cause) };
}
