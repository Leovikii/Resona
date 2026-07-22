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

export type PreviewLyricsFixture = "empty" | "long-latin" | "long-zh" | "short" | "two-lines";

export function previewLyricsSnapshot(fixture: PreviewLyricsFixture = "short"): LyricsSnapshot {
  const fixtureLines: Record<Exclude<PreviewLyricsFixture, "empty">, [string, string]> = {
    short: ["Hold the midnight signal", "Let the silence turn to sound"],
    "two-lines": [
      "让这一段歌词在常见桌面歌词宽度下自然换行并稳定占用两行",
      "下一行不应在当前歌词换行时出现",
    ],
    "long-zh": [
      "这是一条用于验证桌面歌词稳定性的超长中文歌词，它会超过两行并进入水平滚动模式，但渲染模式和窗口几何绝对不能来回切换或者发生任何抖动",
      "下一行预览必须保持隐藏",
    ],
    "long-latin": [
      "SUPERCALIFRAGILISTICEXPIALIDOCIOUS_RES0NA_DESKTOP_LYRICS_STABILITY_WITHOUT_ANY_BREAKING_SPACES_0123456789",
      "The next line stays hidden while the current line scrolls",
    ],
  };
  if (fixture === "empty") {
    return {
      ...emptyLyricsSnapshot,
      revision: 1,
      audioPath: "C:\\Music\\Resona Demo\\Midnight Signal.flac",
      status: "empty",
    };
  }
  const [current, next] = fixtureLines[fixture];
  return {
    revision: 1,
    audioPath: "C:\\Music\\Resona Demo\\Midnight Signal.flac",
    status: "ready",
    activeLineIndex: 0,
    error: null,
    document: {
      sourcePath: "C:\\Music\\Resona Demo\\Midnight Signal.lrc",
      format: "lrc",
      title: "Midnight Signal",
      artist: "Resona",
      album: null,
      warningCount: 0,
      lines: [
        { startMs: 0, endMs: 8_000, text: current },
        { startMs: 8_000, endMs: null, text: next },
      ],
    },
  };
}
