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
