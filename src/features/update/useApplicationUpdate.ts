import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import { invokeTauri, isTauriRuntime } from "../../shared/bridge/tauri";
import {
  defaultApplicationUpdateSnapshot,
  type ApplicationUpdateCheckResult,
  type ApplicationUpdateFailure,
  type ApplicationUpdateProgress,
  type ApplicationUpdateRelease,
  type ApplicationUpdateSnapshot,
} from "../../shared/model/applicationUpdate";

type ApplicationUpdateStatus = "idle" | "checking" | "installing" | "cancelling";

const previewUpdateAvailable =
  import.meta.env.DEV
  && !isTauriRuntime()
  && new URLSearchParams(window.location.search).get("update") === "available";

const previewUpdateRelease: ApplicationUpdateRelease = {
  version: "0.1.0",
  title: "Resona 0.1.0",
  notes: "Release readiness fixes and final design polish.",
  publishedAt: "2026-07-23T12:00:00Z",
  releaseUrl: "https://github.com/Leovikii/Resona/releases/tag/v0.1.0",
  installerSize: 6 * 1024 * 1024,
  prerelease: false,
};

export function useApplicationUpdate() {
  const [snapshot, setSnapshot] = useState<ApplicationUpdateSnapshot>(() =>
    previewUpdateAvailable
      ? { ...defaultApplicationUpdateSnapshot, updaterConfigured: true }
      : defaultApplicationUpdateSnapshot);
  const [available, setAvailable] = useState<ApplicationUpdateRelease | null>(
    previewUpdateAvailable ? previewUpdateRelease : null,
  );
  const [checked, setChecked] = useState(previewUpdateAvailable);
  const [status, setStatus] = useState<ApplicationUpdateStatus>("idle");
  const [progress, setProgress] = useState<ApplicationUpdateProgress>({
    downloadedBytes: 0,
    totalBytes: null,
  });
  const [error, setError] = useState<ApplicationUpdateFailure | null>(null);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    void invokeTauri<ApplicationUpdateSnapshot>("get_application_update_state")
      .then((next) => {
        setSnapshot(next);
        setError(null);
      })
      .catch((nextError) => setError(normalizeFailure(nextError)));
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<ApplicationUpdateProgress>(
      "resona://application-update-progress",
      ({ payload }) => setProgress(payload),
    ).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const setReceivePrereleaseUpdates = useCallback(async (enabled: boolean) => {
    try {
      const next = isTauriRuntime()
        ? await invokeTauri<ApplicationUpdateSnapshot>(
            "set_receive_prerelease_updates",
            { enabled },
          )
        : { ...snapshot, receivePrereleaseUpdates: enabled };
      setSnapshot(next);
      setAvailable(null);
      setChecked(false);
      setError(null);
      return true;
    } catch (nextError) {
      setError(normalizeFailure(nextError));
      return false;
    }
  }, [snapshot]);

  const check = useCallback(async () => {
    if (status !== "idle") return null;
    setStatus("checking");
    setChecked(false);
    setError(null);
    try {
      const result = isTauriRuntime()
        ? await invokeTauri<ApplicationUpdateCheckResult>("check_application_update")
        : {
            currentVersion: snapshot.currentVersion,
            update: previewUpdateAvailable ? previewUpdateRelease : null,
          };
      setAvailable(result.update);
      setChecked(true);
      return result.update;
    } catch (nextError) {
      setAvailable(null);
      setError(normalizeFailure(nextError));
      return null;
    } finally {
      setStatus("idle");
    }
  }, [snapshot.currentVersion, status]);

  const install = useCallback(async () => {
    if (!available || status !== "idle") return false;
    setStatus("installing");
    setProgress({ downloadedBytes: 0, totalBytes: available.installerSize });
    setError(null);
    try {
      if (isTauriRuntime()) {
        await invokeTauri<void>("install_application_update", { version: available.version });
      }
      return true;
    } catch (nextError) {
      const failure = normalizeFailure(nextError);
      if (failure.code !== "update_cancelled") setError(failure);
      return false;
    } finally {
      setStatus("idle");
    }
  }, [available, status]);

  const cancel = useCallback(async () => {
    if (status !== "installing") return;
    setStatus("cancelling");
    try {
      if (isTauriRuntime()) await invokeTauri<void>("cancel_application_update");
    } catch (nextError) {
      setError(normalizeFailure(nextError));
      setStatus("installing");
    }
  }, [status]);

  return {
    available,
    cancel,
    check,
    checked,
    error,
    install,
    progress,
    setReceivePrereleaseUpdates,
    snapshot,
    status,
  };
}

function normalizeFailure(error: unknown): ApplicationUpdateFailure {
  if (typeof error === "object" && error !== null) {
    const value = error as { code?: unknown; message?: unknown };
    if (typeof value.code === "string" && typeof value.message === "string") {
      return { code: value.code, message: value.message };
    }
  }
  return { code: "update_failed", message: String(error) };
}
