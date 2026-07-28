import type { LyricsLine } from "../../shared/model/lyrics";

export interface DesktopLyricsLineTiming {
  delayMs: number;
  durationMs: number;
}

export interface DesktopLyricsPlaybackObservation {
  observedAtMs: number;
  positionMs: number;
  status: string;
}

const SEEK_TOLERANCE_MS = 750;

export function desktopLyricsLineTiming(
  lines: LyricsLine[],
  activeLineIndex: number | null,
  positionMs: number,
  trackDurationMs: number | null,
): DesktopLyricsLineTiming | null {
  if (activeLineIndex === null || !lines[activeLineIndex]) return null;
  const line = lines[activeLineIndex];
  const endMs = line.endMs ?? lines[activeLineIndex + 1]?.startMs ?? trackDurationMs;
  if (endMs === null || endMs <= line.startMs) return null;
  const durationMs = endMs - line.startMs;
  return {
    delayMs: -Math.min(durationMs, Math.max(0, positionMs - line.startMs)),
    durationMs,
  };
}

export function desktopLyricsPlaybackNeedsResync(
  previous: DesktopLyricsPlaybackObservation,
  current: DesktopLyricsPlaybackObservation,
) {
  if (current.status === "paused") {
    return previous.status !== "paused"
      || current.positionMs !== previous.positionMs;
  }
  if (current.status !== "playing" || previous.status !== "playing") return false;
  const playbackDelta = current.positionMs - previous.positionMs;
  const clockDelta = Math.max(0, current.observedAtMs - previous.observedAtMs);
  return Math.abs(playbackDelta - clockDelta) > SEEK_TOLERANCE_MS;
}
