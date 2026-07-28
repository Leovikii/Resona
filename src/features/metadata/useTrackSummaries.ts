import { useEffect, useMemo, useRef, useState } from "react";

import { invokeTauri, isTauriRuntime } from "../../shared/bridge/tauri";
import type { TrackSummary } from "../../shared/model/metadata";
import { fileNameFromPath } from "../../shared/utils/format";

const BATCH_SIZE = 64;
const MAX_CACHE_ENTRIES = 4_096;

export function useTrackSummaries(paths: string[]) {
  const cacheRef = useRef(new Map<string, TrackSummary>());
  const requestedRef = useRef(new Set<string>());
  const [revision, setRevision] = useState(0);
  const pathKey = paths.join("\0");

  useEffect(() => {
    const uniquePaths = [...new Set(paths)];
    for (const path of uniquePaths) {
      const cached = cacheRef.current.get(path);
      if (!cached) continue;
      cacheRef.current.delete(path);
      cacheRef.current.set(path, cached);
    }
    const pending = uniquePaths.filter((path) => !requestedRef.current.has(path));
    if (pending.length === 0) return;
    let disposed = false;
    void (async () => {
      for (let index = 0; index < pending.length && !disposed; index += BATCH_SIZE) {
        const batch = pending.slice(index, index + BATCH_SIZE);
        for (const path of batch) requestedRef.current.add(path);
        try {
          const summaries = isTauriRuntime()
            ? await invokeTauri<TrackSummary[]>("get_track_summaries", { paths: batch })
            : batch.map(previewSummary);
          for (const summary of summaries) cacheSummary(summary, cacheRef.current, requestedRef.current);
        } catch (error) {
          console.warn("track metadata summary failed", error);
          for (const path of batch) {
            cacheSummary({
              path,
              title: null,
              trackNumber: null,
              durationMs: null,
              metadataWarning: String(error),
            }, cacheRef.current, requestedRef.current);
          }
        }
        setRevision((current) => current + 1);
      }
    })();
    return () => { disposed = true; };
  }, [pathKey, paths]);

  return useMemo(() => new Map(cacheRef.current), [revision]);
}

function cacheSummary(
  summary: TrackSummary,
  cache: Map<string, TrackSummary>,
  requested: Set<string>,
) {
  cache.delete(summary.path);
  cache.set(summary.path, summary);
  while (cache.size > MAX_CACHE_ENTRIES) {
    const oldest = cache.keys().next().value;
    if (oldest === undefined) break;
    cache.delete(oldest);
    requested.delete(oldest);
  }
}

function previewSummary(path: string): TrackSummary {
  const fileName = fileNameFromPath(path);
  const title = fileName.replace(/\.[^.]+$/, "");
  return {
    path,
    title,
    trackNumber: fileName === "Midnight Signal.flac" ? 2 : null,
    durationMs: 247_000,
    metadataWarning: null,
  };
}
