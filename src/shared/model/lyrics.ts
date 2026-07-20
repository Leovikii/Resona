export type LyricsFormat = "lrc" | "srt" | "vtt";
export type LyricsStatus = "idle" | "missing" | "empty" | "ready" | "failed";

export interface LyricsFailure {
  code: "read" | "decode_text" | "parse" | string;
  message: string;
}

export interface LyricsLine {
  startMs: number;
  endMs: number | null;
  text: string;
}

export interface LyricsDocument {
  sourcePath: string;
  format: LyricsFormat;
  title: string | null;
  artist: string | null;
  album: string | null;
  warningCount: number;
  lines: LyricsLine[];
}

export interface LyricsSnapshot {
  revision: number;
  audioPath: string | null;
  status: LyricsStatus;
  document: LyricsDocument | null;
  activeLineIndex: number | null;
  error: LyricsFailure | null;
}

export const emptyLyricsSnapshot: LyricsSnapshot = {
  revision: 0,
  audioPath: null,
  status: "idle",
  document: null,
  activeLineIndex: null,
  error: null,
};

export function mergeLyricsSnapshot(
  current: LyricsSnapshot,
  incoming: LyricsSnapshot,
): LyricsSnapshot {
  const sameDocument = current.revision === incoming.revision
    && current.audioPath === incoming.audioPath;
  return {
    ...incoming,
    document: incoming.document ?? (sameDocument ? current.document : null),
  };
}

export function previewLyricsSnapshot(): LyricsSnapshot {
  return {
    revision: 1,
    audioPath: "C:\\Music\\Resona Demo\\Midnight Signal.flac",
    status: "ready",
    activeLineIndex: 3,
    error: null,
    document: {
      sourcePath: "C:\\Music\\Resona Demo\\Midnight Signal.lrc",
      format: "lrc",
      title: "Midnight Signal",
      artist: "Resona",
      album: null,
      warningCount: 0,
      lines: [
        { startMs: 60_000, endMs: 68_000, text: "Streetlights fade into the rain" },
        { startMs: 68_000, endMs: 75_000, text: "A quiet pulse beneath the city" },
        { startMs: 75_000, endMs: 81_000, text: "Every frequency aligns" },
        { startMs: 81_000, endMs: 89_000, text: "Hold the midnight signal" },
        { startMs: 89_000, endMs: 96_000, text: "Let the silence turn to sound" },
        { startMs: 96_000, endMs: null, text: "We are listening now" },
      ],
    },
  };
}
