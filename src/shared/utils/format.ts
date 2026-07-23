export function formatDuration(milliseconds: number | null | undefined) {
  const totalSeconds = Math.max(0, Math.floor((milliseconds ?? 0) / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`
    : `${minutes}:${String(seconds).padStart(2, "0")}`;
}

export function fileNameFromPath(path: string | null | undefined) {
  return path?.split(/[\\/]/).pop() ?? "";
}

export function directoryOf(path: string) {
  const separator = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  if (separator < 0) return null;
  if (separator === 2 && path[1] === ":") return path.slice(0, 3);
  return separator > 0 ? path.slice(0, separator) : path.slice(0, 1);
}

export function formatBytes(bytes: number | null | undefined) {
  const value = Math.max(0, bytes ?? 0);
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KiB`;
  if (value < 1024 * 1024 * 1024) return `${(value / (1024 * 1024)).toFixed(1)} MiB`;
  return `${(value / (1024 * 1024 * 1024)).toFixed(2)} GiB`;
}
