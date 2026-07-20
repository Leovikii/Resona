import { useCallback, useEffect, useState } from "react";

import { invokeTauri, isTauriRuntime } from "../../shared/bridge/tauri";
import {
  emptyCompressionSnapshot,
  emptyCompressionScanSnapshot,
  type CompressionPreset,
  type CompressionScanSnapshot,
  type CompressionSnapshot,
} from "../../shared/model/compression";

export function useAudioCompression() {
  const preview = import.meta.env.DEV && !isTauriRuntime();
  const [snapshot, setSnapshot] = useState(emptyCompressionSnapshot);
  const [scan, setScan] = useState<CompressionScanSnapshot>(() =>
    preview && new URLSearchParams(window.location.search).get("preview") === "ready"
      ? previewScan(["C:\\Music"])
      : emptyCompressionScanSnapshot);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!isTauriRuntime()) return;
    try {
      setSnapshot(await invokeTauri<CompressionSnapshot>("get_audio_compression_state"));
    } catch (nextError) {
      setError(errorMessage(nextError));
    }
  }, []);

  const refreshScan = useCallback(async () => {
    if (!isTauriRuntime()) return;
    try {
      setScan(await invokeTauri<CompressionScanSnapshot>("get_audio_compression_scan_state"));
    } catch (nextError) {
      setError(errorMessage(nextError));
    }
  }, []);

  useEffect(() => {
    void refresh();
    void refreshScan();
  }, [refresh, refreshScan]);

  useEffect(() => {
    if (snapshot.status !== "running" && snapshot.status !== "cancelling") return;
    const timer = window.setInterval(() => void refresh(), 300);
    return () => window.clearInterval(timer);
  }, [refresh, snapshot.status]);

  useEffect(() => {
    if (scan.status !== "scanning" && scan.status !== "cancelling") return;
    const timer = window.setInterval(() => void refreshScan(), 250);
    return () => window.clearInterval(timer);
  }, [refreshScan, scan.status]);

  const addInputs = useCallback(async (paths: string[]) => {
    if (paths.length === 0) return false;
    try {
      const next = isTauriRuntime()
        ? await invokeTauri<CompressionScanSnapshot>("scan_audio_compression_inputs", { paths })
        : previewScan(paths);
      setScan(next);
      setError(null);
      return true;
    } catch (nextError) {
      setError(errorMessage(nextError));
      return false;
    }
  }, []);

  const removeInputs = useCallback(async (paths: string[]) => {
    try {
      const next = isTauriRuntime()
        ? await invokeTauri<CompressionScanSnapshot>("remove_audio_compression_inputs", { paths })
        : removePreviewNodes(scan, new Set(paths));
      setScan(next);
      setError(null);
    } catch (nextError) {
      setError(errorMessage(nextError));
    }
  }, [scan]);

  const clearInputs = useCallback(async () => {
    try {
      const next = isTauriRuntime()
        ? await invokeTauri<CompressionScanSnapshot>("clear_audio_compression_inputs")
        : emptyCompressionScanSnapshot;
      setScan(next);
      setError(null);
    } catch (nextError) {
      setError(errorMessage(nextError));
    }
  }, []);

  const cancelScan = useCallback(async () => {
    if (!isTauriRuntime()) {
      setScan((current) => ({ ...current, status: "cancelled" }));
      return;
    }
    try {
      setScan(await invokeTauri<CompressionScanSnapshot>("cancel_audio_compression_scan"));
      setError(null);
    } catch (nextError) {
      setError(errorMessage(nextError));
    }
  }, []);

  const start = useCallback(async (
    paths: string[],
    preset: CompressionPreset,
    deleteSource: boolean,
    deletionConfirmed: boolean,
  ) => {
    try {
      const next = isTauriRuntime()
        ? await invokeTauri<CompressionSnapshot>("start_audio_compression", {
            paths,
            preset,
            deleteSource,
            deletionConfirmed,
          })
        : {
            ...emptyCompressionSnapshot,
            taskId: 1,
            status: "completed" as const,
            completed: paths.length,
            total: paths.length,
            items: paths.map((source) => ({
              source,
              output: source.replace(/\.wav$/i, ".flac"),
              status: "completed" as const,
              message: null,
              sourceDeleted: deleteSource,
            })),
          };
      setSnapshot(next);
      setError(null);
      return true;
    } catch (nextError) {
      setError(errorMessage(nextError));
      return false;
    }
  }, []);

  const cancel = useCallback(async () => {
    if (!isTauriRuntime()) return;
    try {
      setSnapshot(await invokeTauri<CompressionSnapshot>("cancel_audio_compression"));
      setError(null);
    } catch (nextError) {
      setError(errorMessage(nextError));
    }
  }, []);

  return {
    addInputs,
    cancel,
    cancelScan,
    clearInputs,
    error,
    refresh,
    refreshScan,
    removeInputs,
    scan,
    snapshot,
    start,
  };
}

function previewScan(paths: string[]): CompressionScanSnapshot {
  const roots = paths.map((path, index) => ({
    path,
    name: path.split(/[\\/]/).filter(Boolean).pop() || path,
    kind: "root" as const,
    ready: false,
    issueCode: null,
    children: [
      {
        path: `${path}\\Album ${index + 1}`,
        name: `Album ${index + 1}`,
        kind: "directory" as const,
        ready: false,
        issueCode: null,
        children: [
          {
            path: `${path}\\Album ${index + 1}\\Track 01.wav`,
            name: "Track 01.wav",
            kind: "file" as const,
            ready: true,
            issueCode: null,
            children: [],
          },
          {
            path: `${path}\\Album ${index + 1}\\Track 02.wav`,
            name: "Track 02.wav",
            kind: "file" as const,
            ready: false,
            issueCode: "output_exists",
            children: [],
          },
        ],
      },
    ],
  }));
  return {
    ...emptyCompressionScanSnapshot,
    scanId: 1,
    status: "ready",
    inputRoots: paths,
    scannedEntries: paths.length * 4,
    candidateFiles: paths.length * 2,
    validatedFiles: paths.length * 2,
    readyFiles: paths.length,
    roots,
  };
}

function removePreviewNodes(snapshot: CompressionScanSnapshot, removed: Set<string>) {
  const filter = (nodes: CompressionScanSnapshot["roots"]): CompressionScanSnapshot["roots"] => nodes
    .filter((node) => !removed.has(node.path))
    .map((node) => ({ ...node, children: filter(node.children) }));
  const roots = filter(snapshot.roots);
  const readyFiles = countReady(roots);
  const candidateFiles = countFiles(roots);
  return {
    ...snapshot,
    candidateFiles,
    inputRoots: roots.map((root) => root.path),
    roots,
    readyFiles,
    validatedFiles: candidateFiles,
  };
}

function countReady(nodes: CompressionScanSnapshot["roots"]): number {
  return nodes.reduce((total, node) => total + (node.ready ? 1 : 0) + countReady(node.children), 0);
}

function countFiles(nodes: CompressionScanSnapshot["roots"]): number {
  return nodes.reduce(
    (total, node) => total + (node.kind === "file" ? 1 : 0) + countFiles(node.children),
    0,
  );
}

function errorMessage(error: unknown) {
  if (typeof error === "object" && error !== null && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}
