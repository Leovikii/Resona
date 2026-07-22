import { useCallback, useEffect, useRef, useState } from "react";

import { invokeTauri, isTauriRuntime } from "../../shared/bridge/tauri";
import {
  emptyLyricsSnapshot,
  mergeLyricsSnapshot,
  previewLyricsSnapshot,
  type PreviewLyricsFixture,
  type LyricsSnapshot,
} from "../../shared/model/lyrics";
import {
  emptySnapshot,
  previewSnapshot,
  type PlaybackFailure,
  type PlaybackSnapshot,
} from "../../shared/model/playback";

type DesktopPlaybackCommand =
  | "next_playback"
  | "pause_playback"
  | "play_queue_item"
  | "previous_playback"
  | "resume_playback";

interface NowPlayingSnapshot {
  playback: PlaybackSnapshot;
  lyrics: LyricsSnapshot;
}

export function useDesktopLyricsNowPlaying() {
  const preview = !isTauriRuntime();
  const previewEmpty = preview
    && new URLSearchParams(window.location.search).get("preview") === "empty";
  const previewFixture = preview ? readPreviewFixture() : "short";
  const [playback, setPlayback] = useState(() => preview && !previewEmpty
    ? previewSnapshot()
    : emptySnapshot);
  const [lyrics, setLyrics] = useState(() => preview && !previewEmpty
    ? previewLyricsSnapshot(previewFixture)
    : emptyLyricsSnapshot);
  const [error, setError] = useState<PlaybackFailure | null>(null);
  const [initialized, setInitialized] = useState(preview);
  const revisionRef = useRef(lyrics.revision);

  const acceptPlayback = useCallback((next: PlaybackSnapshot) => {
    setPlayback(next);
    setLyrics((current) => current.audioPath === next.path ? current : {
      ...emptyLyricsSnapshot,
      revision: current.revision,
      audioPath: next.path,
    });
  }, []);

  useEffect(() => {
    revisionRef.current = lyrics.revision;
  }, [lyrics.revision]);

  const refresh = useCallback(async () => {
    if (preview) return;
    try {
      const next = await invokeTauri<NowPlayingSnapshot>("get_now_playing_state", {
        knownLyricsRevision: revisionRef.current,
      });
      acceptPlayback(next.playback);
      setLyrics((current) => mergeLyricsSnapshot(current, next.lyrics));
      setError(null);
    } catch (nextError) {
      setError(toPlaybackFailure(nextError));
    } finally {
      setInitialized(true);
    }
  }, [acceptPlayback, preview]);

  const runPlayback = useCallback(async (
    command: DesktopPlaybackCommand,
    args?: Record<string, unknown>,
  ) => {
    try {
      if (preview) {
        acceptPlayback(applyPreviewPlayback(playback, command, args));
        return;
      }
      acceptPlayback(await invokeTauri<PlaybackSnapshot>(command, args));
      setError(null);
    } catch (nextError) {
      setError(toPlaybackFailure(nextError));
    }
  }, [acceptPlayback, playback, preview]);

  useEffect(() => {
    void refresh();
    if (preview) return;
    const timer = window.setInterval(() => void refresh(), 750);
    return () => window.clearInterval(timer);
  }, [preview, refresh]);

  return { error, initialized, lyrics, playback, runPlayback };
}

function readPreviewFixture(): PreviewLyricsFixture {
  const value = new URLSearchParams(window.location.search).get("lyricsFixture");
  return value === "empty" || value === "long-latin" || value === "long-zh" || value === "two-lines"
    ? value
    : "short";
}

function applyPreviewPlayback(
  snapshot: PlaybackSnapshot,
  command: DesktopPlaybackCommand,
  args?: Record<string, unknown>,
): PlaybackSnapshot {
  if (command === "pause_playback") return { ...snapshot, status: "paused" };
  if (command === "resume_playback") return { ...snapshot, status: "playing" };
  if (command === "play_queue_item") {
    const id = Number(args?.id);
    const item = snapshot.queue.find((candidate) => candidate.id === id);
    return item ? {
      ...snapshot,
      currentItemId: item.id,
      path: item.path,
      positionMs: 0,
      durationMs: item.durationMs,
      status: "playing",
    } : snapshot;
  }
  const currentIndex = snapshot.queue.findIndex((item) => item.id === snapshot.currentItemId);
  const offset = command === "next_playback" ? 1 : -1;
  const item = snapshot.queue[currentIndex + offset];
  return item ? applyPreviewPlayback(snapshot, "play_queue_item", { id: item.id }) : snapshot;
}

function toPlaybackFailure(error: unknown): PlaybackFailure {
  if (typeof error === "object" && error !== null) {
    const candidate = error as { code?: unknown; message?: unknown };
    if (typeof candidate.code === "string" && typeof candidate.message === "string") {
      return { code: candidate.code, message: candidate.message };
    }
  }
  return { code: "task_failed", message: String(error) };
}
