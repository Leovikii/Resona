import { useCallback, useEffect, useRef, useState } from "react";

import { invokeTauri, isTauriRuntime } from "../../shared/bridge/tauri";
import {
  checkingFfmpegDependency,
  type FfmpegDependencySnapshot,
} from "../../shared/model/compression";

export function useFfmpegDependency() {
  const preview = import.meta.env.DEV && !isTauriRuntime();
  const previewTimer = useRef<number | null>(null);
  const [snapshot, setSnapshot] = useState<FfmpegDependencySnapshot>(() => {
    if (!preview) return checkingFfmpegDependency;
    return new URLSearchParams(window.location.search).get("ffmpeg") === "missing"
      ? { ...checkingFfmpegDependency, status: "missing" }
      : {
          ...checkingFfmpegDependency,
          status: "ready",
          installedBytes: 148 * 1024 * 1024,
        };
  });
  const [commandError, setCommandError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!isTauriRuntime()) return;
    try {
      setSnapshot(await invokeTauri<FfmpegDependencySnapshot>("get_ffmpeg_dependency_state"));
      setCommandError(null);
    } catch (error) {
      setCommandError(errorMessage(error));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!["checking", "downloading", "installing", "cancelling"].includes(snapshot.status)) {
      return;
    }
    const timer = window.setInterval(() => void refresh(), 300);
    return () => window.clearInterval(timer);
  }, [refresh, snapshot.status]);

  useEffect(() => () => {
    if (previewTimer.current !== null) window.clearInterval(previewTimer.current);
  }, []);

  const install = useCallback(async () => {
    try {
      if (preview) {
        let downloadedBytes = 0;
        const totalBytes = 96 * 1024 * 1024;
        setSnapshot({
          ...checkingFfmpegDependency,
          status: "downloading",
          totalBytes,
        });
        previewTimer.current = window.setInterval(() => {
          downloadedBytes = Math.min(totalBytes, downloadedBytes + 12 * 1024 * 1024);
          if (downloadedBytes >= totalBytes) {
            if (previewTimer.current !== null) window.clearInterval(previewTimer.current);
            previewTimer.current = null;
            setSnapshot({
              ...checkingFfmpegDependency,
              status: "ready",
              downloadedBytes,
              totalBytes,
              installedBytes: 148 * 1024 * 1024,
            });
          } else {
            setSnapshot((current) => ({ ...current, downloadedBytes }));
          }
        }, 180);
      } else {
        setSnapshot(await invokeTauri<FfmpegDependencySnapshot>("install_ffmpeg_dependency"));
      }
      setCommandError(null);
    } catch (error) {
      setCommandError(errorMessage(error));
    }
  }, [preview]);

  const cancel = useCallback(async () => {
    try {
      if (preview) {
        if (previewTimer.current !== null) window.clearInterval(previewTimer.current);
        previewTimer.current = null;
        setSnapshot((current) => ({ ...current, status: "cancelled" }));
      } else {
        setSnapshot(await invokeTauri<FfmpegDependencySnapshot>(
          "cancel_ffmpeg_dependency_install",
        ));
      }
      setCommandError(null);
    } catch (error) {
      setCommandError(errorMessage(error));
    }
  }, [preview]);

  return {
    cancel,
    commandError,
    install,
    refresh,
    snapshot,
  };
}

function errorMessage(error: unknown) {
  if (typeof error === "object" && error !== null && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}
