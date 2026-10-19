import type { CompressionSnapshot } from "../../shared/model/compression";

/** Display projection only; batch status and per-file results remain Rust-owned. */
export function compressionProgress(snapshot: CompressionSnapshot) {
  let processed = 0;
  let running = 0;
  let succeeded = 0;
  let failed = 0;
  let cancelled = 0;
  let deleted = 0;
  let warnings = 0;
  let sourceBytes = 0;
  let outputBytes = 0;
  for (const item of snapshot.items) {
    const progress = Number.isFinite(item.progress) ? Math.min(1, Math.max(0, item.progress)) : 0;
    processed += item.status === "completed" || item.status === "failed" ? 1 : item.status === "pending" ? 0 : progress;
    if (item.status === "running") running++;
    if (item.status === "failed") failed++;
    if (item.status === "cancelled") cancelled++;
    if (item.status === "completed") {
      succeeded++;
      if (item.sourceDeleted) deleted++;
      if (item.message) warnings++;
      sourceBytes += item.sourceBytes;
      outputBytes += item.outputBytes;
    }
  }
  const busy = snapshot.status === "running" || snapshot.status === "cancelling";
  const percent = snapshot.total > 0 ? Math.min(100, Math.round(processed / snapshot.total * 100)) : 0;
  return {
    percent: busy ? Math.min(99, percent) : percent,
    running, succeeded, failed, cancelled, deleted, warnings, sourceBytes, outputBytes,
  };
}
