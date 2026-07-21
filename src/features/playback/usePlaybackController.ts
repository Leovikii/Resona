import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import { selectAudioFiles, selectAudioFolders } from "../../shared/bridge/audioDialog";
import { invokeTauri, isTauriRuntime } from "../../shared/bridge/tauri";
import type {
  ActivePlaylistSnapshot,
  DefaultPlaylistSnapshot,
  DefaultPlaylistMutationResult,
  OpenMediaResult,
  PlaylistItem,
  PlaylistPlaybackResult,
} from "../../shared/model/library";
import {
  emptyLyricsSnapshot,
  mergeLyricsSnapshot,
  previewLyricsSnapshot,
  type LyricsSnapshot,
} from "../../shared/model/lyrics";
import {
  emptySnapshot,
  previewSnapshot,
  type PlaybackFailure,
  type PlaybackSnapshot,
} from "../../shared/model/playback";

type PlaybackCommand =
  | "next_playback"
  | "pause_playback"
  | "play_queue_item"
  | "previous_playback"
  | "refresh_output_devices"
  | "resume_playback"
  | "seek_playback"
  | "select_output_device"
  | "set_playback_mode"
  | "set_playback_volume"
  | "stop_playback";

const emptyDefaultPlaylist: DefaultPlaylistSnapshot = {
  revision: 0,
  sourceDirectory: null,
  selectedIndex: null,
  items: [],
};

export function usePlaybackController() {
  const preview = import.meta.env.DEV && !isTauriRuntime();
  const previewPlayback = useMemo(() => previewSnapshot(), []);
  const [snapshot, setSnapshot] = useState<PlaybackSnapshot>(() =>
    preview && new URLSearchParams(window.location.search).get("preview") !== "empty"
      ? previewPlayback
      : emptySnapshot,
  );
  const [defaultPlaylist, setDefaultPlaylist] = useState<DefaultPlaylistSnapshot>(() =>
    preview && new URLSearchParams(window.location.search).get("preview") !== "empty"
      ? defaultFromPlayback(previewPlayback)
      : emptyDefaultPlaylist,
  );
  const [defaultRejectedCount, setDefaultRejectedCount] = useState(0);
  const [activePlaylist, setActivePlaylist] = useState<ActivePlaylistSnapshot | null>(
    preview && new URLSearchParams(window.location.search).get("preview") !== "empty"
      ? { kind: "default", playlistId: null }
      : null,
  );
  const [pending, setPending] = useState(false);
  const [lyrics, setLyrics] = useState<LyricsSnapshot>(() =>
    preview ? previewLyricsSnapshot() : emptyLyricsSnapshot,
  );
  const [dialogOpen, setDialogOpen] = useState(false);
  const [openSequence, setOpenSequence] = useState(0);
  const [refreshError, setRefreshError] = useState<PlaybackFailure | null>(null);
  const snapshotRef = useRef(snapshot);
  const defaultPlaylistRef = useRef(defaultPlaylist);
  const lyricsRevisionRef = useRef(lyrics.revision);

  useEffect(() => {
    lyricsRevisionRef.current = lyrics.revision;
  }, [lyrics.revision]);

  const acceptPlaybackSnapshot = useCallback((next: PlaybackSnapshot) => {
    const merged = mergePlaybackSnapshot(snapshotRef.current, next);
    snapshotRef.current = merged;
    setSnapshot(merged);
    setLyrics((current) => current.audioPath === next.path ? current : {
      ...emptyLyricsSnapshot,
      revision: current.revision,
      audioPath: next.path,
    });
  }, []);

  const acceptOpenResult = useCallback((result: OpenMediaResult) => {
    acceptPlaybackSnapshot(result.playback);
    setActivePlaylist(result.activePlaylist);
    defaultPlaylistRef.current = result.defaultPlaylist;
    setDefaultPlaylist(result.defaultPlaylist);
    setDefaultRejectedCount(0);
    setOpenSequence((current) => current + 1);
    setRefreshError(null);
  }, [acceptPlaybackSnapshot]);

  const acceptDefaultPlaylist = useCallback((next: DefaultPlaylistSnapshot) => {
    defaultPlaylistRef.current = next;
    setDefaultPlaylist(next);
  }, []);

  const acceptPlaylistPlayback = useCallback((result: PlaylistPlaybackResult) => {
    acceptPlaybackSnapshot(result.playback);
    setActivePlaylist(result.activePlaylist);
    setRefreshError(null);
  }, [acceptPlaybackSnapshot]);

  const currentItem = useMemo(
    () => snapshot.queue.find((item) => item.id === snapshot.currentItemId) ?? null,
    [snapshot.currentItemId, snapshot.queue],
  );
  const selectedPath = snapshot.path ?? currentItem?.path ?? snapshot.queue[0]?.path ?? null;

  const refresh = useCallback(async () => {
    if (!isTauriRuntime()) return;
    try {
      const next = await invokeTauri<NowPlayingSnapshot>("get_now_playing_state", {
        knownLyricsRevision: lyricsRevisionRef.current,
      });
      acceptPlaybackSnapshot(next.playback);
      setActivePlaylist((current) => sameActivePlaylist(current, next.activePlaylist)
        ? current
        : next.activePlaylist);
      setLyrics((current) => mergeLyricsSnapshot(current, next.lyrics));
      setRefreshError(null);
    } catch (error) {
      setRefreshError(toPlaybackFailure(error));
    }
  }, [acceptPlaybackSnapshot]);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    void Promise.all([
      refresh(),
      invokeTauri<DefaultPlaylistSnapshot>("get_default_playlist").then(acceptDefaultPlaylist),
    ]).catch((error) => setRefreshError(toPlaybackFailure(error)));
    const timer = window.setInterval(() => void refresh(), 750);
    return () => window.clearInterval(timer);
  }, [acceptDefaultPlaylist, refresh]);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    let stopOpened: (() => void) | undefined;
    let stopFailed: (() => void) | undefined;
    void listen<OpenMediaResult>("resona://media-opened", (event) => {
      if (!disposed) acceptOpenResult(event.payload);
    }).then((stop) => {
      if (disposed) stop();
      else stopOpened = stop;
    });
    void listen<PlaybackFailure>("resona://media-open-failed", (event) => {
      if (!disposed) setRefreshError(event.payload);
    }).then((stop) => {
      if (disposed) stop();
      else stopFailed = stop;
    });
    return () => {
      disposed = true;
      stopOpened?.();
      stopFailed?.();
    };
  }, [acceptOpenResult]);

  const run = useCallback(async (
    command: PlaybackCommand,
    args?: Record<string, unknown>,
  ) => {
    try {
      const next = preview
        ? applyPreviewCommand(snapshotRef.current, command, args)
        : await invokeTauri<PlaybackSnapshot>(command, args);
      acceptPlaybackSnapshot(next);
      return next;
    } catch (error) {
      const failure = toPlaybackFailure(error);
      setSnapshot((current) => ({ ...current, error: failure }));
      return null;
    }
  }, [acceptPlaybackSnapshot, preview]);

  const openPath = useCallback(async (path: string) => {
    setPending(true);
    try {
      if (preview) {
        const playback = playPreviewPath(snapshotRef.current, path);
        acceptOpenResult({
          playback,
          defaultPlaylist: defaultFromPlayback(playback),
          activePlaylist: { kind: "default", playlistId: null },
        });
        return playback;
      }
      const result = await invokeTauri<OpenMediaResult>("open_media_context", { path });
      acceptOpenResult(result);
      return result.playback;
    } catch (error) {
      const failure = toPlaybackFailure(error);
      setSnapshot((current) => ({ ...current, error: failure }));
      return null;
    } finally {
      setPending(false);
    }
  }, [acceptOpenResult, preview]);

  const addDefaultItems = useCallback(async (paths: string[]) => {
    if (paths.length === 0) return null;
    setPending(true);
    try {
      const result = preview
        ? appendPreviewDefault(defaultPlaylistRef.current, paths)
        : await invokeTauri<DefaultPlaylistMutationResult>("add_default_playlist_items", { paths });
      acceptDefaultPlaylist(result.defaultPlaylist);
      await refresh();
      setDefaultRejectedCount(result.rejected.length);
      setRefreshError(null);
      return result;
    } catch (error) {
      const failure = toPlaybackFailure(error);
      setSnapshot((current) => ({ ...current, error: failure }));
      return null;
    } finally {
      setPending(false);
    }
  }, [acceptDefaultPlaylist, preview, refresh]);

  const chooseAndAddDefault = useCallback(async (kind: "files" | "folders") => {
    setDialogOpen(true);
    try {
      if (preview) {
        const paths = kind === "files"
          ? ["C:\\Music\\Imported track.flac"]
          : ["C:\\Music\\Imported\\Folder track.flac"];
        await addDefaultItems(paths);
        return;
      }
      const paths = kind === "files" ? await selectAudioFiles(true) : await selectAudioFolders();
      await addDefaultItems(paths);
    } catch (error) {
      setSnapshot((current) => ({
        ...current,
        error: { ...toPlaybackFailure(error), code: "dialog_open_failed" },
      }));
    } finally {
      setDialogOpen(false);
    }
  }, [addDefaultItems, preview]);

  const playDefaultItem = useCallback(async (itemId: number) => {
    setPending(true);
    try {
      if (preview) {
        const current = defaultPlaylistRef.current;
        const selectedIndex = current.items.findIndex((item) => item.id === itemId);
        const playback = replacePreviewAndPlay(
          snapshotRef.current,
          current.items.map((item) => item.path),
          Math.max(0, selectedIndex),
        );
        acceptOpenResult({
          playback,
          defaultPlaylist: { ...current, selectedIndex: Math.max(0, selectedIndex) },
          activePlaylist: { kind: "default", playlistId: null },
        });
        return playback;
      }
      const result = await invokeTauri<OpenMediaResult>("play_default_playlist_item", {
        itemId,
      });
      acceptOpenResult(result);
      return result.playback;
    } catch (error) {
      const failure = toPlaybackFailure(error);
      setSnapshot((current) => ({ ...current, error: failure }));
      return null;
    } finally {
      setPending(false);
    }
  }, [acceptOpenResult, preview]);

  const removeDefaultItems = useCallback(async (itemIds: number[]) => {
    if (itemIds.length === 0) return null;
    setPending(true);
    try {
      if (preview) {
        const selected = new Set(itemIds);
        const next = {
          ...defaultPlaylistRef.current,
          revision: defaultPlaylistRef.current.revision + 1,
          items: defaultPlaylistRef.current.items.filter((item) => !selected.has(item.id)),
        };
        acceptDefaultPlaylist(next);
        return next;
      }
      const next = await invokeTauri<DefaultPlaylistSnapshot>("remove_default_playlist_items", { itemIds });
      acceptDefaultPlaylist(next);
      await refresh();
      return next;
    } catch (error) {
      setSnapshot((current) => ({ ...current, error: toPlaybackFailure(error) }));
      return null;
    } finally {
      setPending(false);
    }
  }, [acceptDefaultPlaylist, preview, refresh]);

  const clearDefaultPlaylist = useCallback(async () => {
    setPending(true);
    try {
      if (preview) {
        const next = { ...defaultPlaylistRef.current, revision: defaultPlaylistRef.current.revision + 1, items: [] };
        acceptDefaultPlaylist(next);
        return next;
      }
      const next = await invokeTauri<DefaultPlaylistSnapshot>("clear_default_playlist");
      acceptDefaultPlaylist(next);
      await refresh();
      return next;
    } catch (error) {
      setSnapshot((current) => ({ ...current, error: toPlaybackFailure(error) }));
      return null;
    } finally {
      setPending(false);
    }
  }, [acceptDefaultPlaylist, preview, refresh]);

  const moveDefaultItem = useCallback(async (itemId: number, toPosition: number) => {
    setPending(true);
    try {
      if (preview) {
        const items = [...defaultPlaylistRef.current.items];
        const from = items.findIndex((item) => item.id === itemId);
        if (from < 0) return null;
        const [moved] = items.splice(from, 1);
        items.splice(Math.max(0, Math.min(toPosition, items.length)), 0, moved);
        const next = { ...defaultPlaylistRef.current, revision: defaultPlaylistRef.current.revision + 1, items };
        acceptDefaultPlaylist(next);
        return next;
      }
      const next = await invokeTauri<DefaultPlaylistSnapshot>("move_default_playlist_item", { itemId, toPosition });
      acceptDefaultPlaylist(next);
      await refresh();
      return next;
    } catch (error) {
      setSnapshot((current) => ({ ...current, error: toPlaybackFailure(error) }));
      return null;
    } finally {
      setPending(false);
    }
  }, [acceptDefaultPlaylist, preview, refresh]);

  const playUserPlaylistItem = useCallback(async (
    playlistId: number,
    itemId: number,
    previewItems: PlaylistItem[] = [],
  ) => {
    setPending(true);
    try {
      if (preview) {
        const selectedIndex = previewItems.findIndex((item) => item.id === itemId);
        const playback = selectedIndex >= 0
          ? replacePreviewAndPlay(
            snapshotRef.current,
            previewItems.map((item) => item.path),
            selectedIndex,
          )
          : snapshotRef.current;
        const result: PlaylistPlaybackResult = {
          playback,
          activePlaylist: { kind: "user", playlistId },
        };
        acceptPlaylistPlayback(result);
        return playback;
      }
      const result = await invokeTauri<PlaylistPlaybackResult>("play_user_playlist_item", {
        playlistId,
        itemId,
      });
      acceptPlaylistPlayback(result);
      return result.playback;
    } catch (error) {
      const failure = toPlaybackFailure(error);
      setSnapshot((current) => ({ ...current, error: failure }));
      return null;
    } finally {
      setPending(false);
    }
  }, [acceptPlaylistPlayback, preview]);

  const reloadDefaultPlaylist = useCallback(async () => {
    if (preview) return;
    try {
      acceptDefaultPlaylist(await invokeTauri<DefaultPlaylistSnapshot>("get_default_playlist"));
    } catch (error) {
      setRefreshError(toPlaybackFailure(error));
    }
  }, [acceptDefaultPlaylist, preview]);

  const chooseAndAddDefaultFiles = useCallback(
    () => chooseAndAddDefault("files"),
    [chooseAndAddDefault],
  );
  const chooseAndAddDefaultFolders = useCallback(
    () => chooseAndAddDefault("folders"),
    [chooseAndAddDefault],
  );

  return {
    busy: pending || dialogOpen,
    activePlaylist,
    addDefaultItems,
    chooseAndAddDefaultFiles,
    chooseAndAddDefaultFolders,
    currentItem,
    defaultPlaylist,
    defaultRejectedCount,
    dialogOpen,
    lyrics,
    openPath,
    openSequence,
    refresh,
    refreshError,
    playDefaultItem,
    playUserPlaylistItem,
    removeDefaultItems,
    clearDefaultPlaylist,
    moveDefaultItem,
    reloadDefaultPlaylist,
    run,
    selectedPath,
    snapshot,
  };
}

interface NowPlayingSnapshot {
  playback: PlaybackSnapshot;
  lyrics: LyricsSnapshot;
  activePlaylist: ActivePlaylistSnapshot | null;
}

export function toPlaybackFailure(error: unknown): PlaybackFailure {
  if (typeof error === "object" && error !== null) {
    const candidate = error as { code?: unknown; message?: unknown };
    if (typeof candidate.code === "string" && typeof candidate.message === "string") {
      return { code: candidate.code, message: candidate.message };
    }
  }
  return { code: "task_failed", message: String(error) };
}

function applyPreviewCommand(
  snapshot: PlaybackSnapshot,
  command: PlaybackCommand,
  args?: Record<string, unknown>,
): PlaybackSnapshot {
  if (command === "pause_playback") return { ...snapshot, status: "paused", error: null };
  if (command === "resume_playback") return { ...snapshot, status: "playing", error: null };
  if (command === "stop_playback") return { ...snapshot, status: "stopped", positionMs: 0 };
  if (command === "seek_playback") {
    return { ...snapshot, positionMs: Number(args?.positionMs ?? 0) };
  }
  if (command === "set_playback_volume") {
    return { ...snapshot, volume: Number(args?.volume ?? 1) };
  }
  if (command === "set_playback_mode") {
    const mode = args?.mode;
    if (mode === "sequential" || mode === "repeat_one" || mode === "repeat_all" || mode === "shuffle") {
      return { ...snapshot, playbackMode: mode };
    }
    return snapshot;
  }
  if (command === "select_output_device") {
    const id = (args?.deviceId as string | null) ?? null;
    const device = snapshot.output.devices.find((candidate) => candidate.id === id);
    return {
      ...snapshot,
      output: {
        ...snapshot.output,
        followSystemDefault: id === null,
        selectedDeviceId: id,
        activeDeviceId: device?.id ?? snapshot.output.devices[0]?.id ?? null,
        activeDeviceName: device?.name ?? snapshot.output.devices[0]?.name ?? null,
      },
    };
  }
  if (command === "play_queue_item") {
    const id = Number(args?.id);
    const selected = snapshot.queue.find((candidate) => candidate.id === id);
    if (!selected) return snapshot;
    return {
      ...snapshot,
      currentItemId: id,
      path: selected.path,
      durationMs: selected.durationMs,
      positionMs: 0,
      status: "playing",
      queue: snapshot.queue.map((candidate) => ({
        ...candidate,
        status: candidate.id === id
          ? "playing"
          : candidate.status === "playing" ? "pending" : candidate.status,
      })),
    };
  }
  return snapshot;
}

function replacePreviewAndPlay(
  snapshot: PlaybackSnapshot,
  paths: string[],
  selectedIndex: number,
) {
  const replaced = replacePreviewPaths(snapshot, paths);
  if (replaced.queue.length === 0) return replaced;
  const index = clamp(selectedIndex, 0, replaced.queue.length - 1);
  const selected = replaced.queue[index];
  return {
    ...replaced,
    currentItemId: selected.id,
    path: selected.path,
    durationMs: selected.durationMs,
    status: "playing" as const,
    queue: replaced.queue.map((candidate, candidateIndex) => ({
      ...candidate,
      status: candidateIndex === index ? "playing" as const : "pending" as const,
    })),
  };
}

function playPreviewPath(snapshot: PlaybackSnapshot, path: string) {
  const item = snapshot.queue.find((candidate) => candidate.path === path) ?? snapshot.queue[0];
  if (!item) return snapshot;
  return applyPreviewCommand(snapshot, "play_queue_item", { id: item.id });
}

function defaultFromPlayback(snapshot: PlaybackSnapshot): DefaultPlaylistSnapshot {
  const selectedIndex = snapshot.currentItemId === null
    ? null
    : snapshot.queue.findIndex((candidate) => candidate.id === snapshot.currentItemId);
  return {
    revision: snapshot.queue.length > 0 ? 1 : 0,
    sourceDirectory: snapshot.queue[0]?.path.replace(/[\\/][^\\/]+$/, "") ?? null,
    selectedIndex: selectedIndex !== -1 ? selectedIndex : null,
    items: snapshot.queue.map((candidate, index) => ({
      id: index + 1,
      path: candidate.path,
      displayName: candidate.displayName,
    })),
  };
}

function appendPreviewDefault(
  current: DefaultPlaylistSnapshot,
  paths: string[],
): DefaultPlaylistMutationResult {
  const known = new Set(current.items.map((item) => item.path));
  const accepted: string[] = [];
  const rejected: DefaultPlaylistMutationResult["rejected"] = [];
  for (const path of paths) {
    if (known.has(path)) {
      rejected.push({ path, reason: "duplicate" });
    } else {
      known.add(path);
      accepted.push(path);
    }
  }
  const items = [
    ...current.items,
    ...accepted.map((path, offset) => ({
      id: Math.max(0, ...current.items.map((item) => item.id)) + offset + 1,
      path,
      displayName: path.split(/[\\/]/).pop() ?? path,
    })),
  ];
  const directories = new Set(items.map((item) => item.path.replace(/[\\/][^\\/]+$/, "")));
  return {
    defaultPlaylist: {
      ...current,
      revision: accepted.length > 0 ? Math.max(1, current.revision + 1) : current.revision,
      sourceDirectory: directories.size === 1 ? [...directories][0] : null,
      items,
    },
    rejected,
  };
}

function insertPreviewPaths(snapshot: PlaybackSnapshot, paths: string[], index: number) {
  const queue = [...snapshot.queue];
  const nextId = Math.max(0, ...queue.map((candidate) => candidate.id)) + 1;
  const items = paths.map((path, offset) => ({
    id: nextId + offset,
    path,
    displayName: path.split(/[\\/]/).pop() ?? path,
    durationMs: null,
    status: "pending" as const,
    error: null,
  }));
  queue.splice(clamp(index, 0, queue.length), 0, ...items);
  return { ...snapshot, queue };
}

function replacePreviewPaths(snapshot: PlaybackSnapshot, paths: string[]): PlaybackSnapshot {
  const next = insertPreviewPaths(
    { ...emptySnapshot, output: snapshot.output, volume: snapshot.volume },
    paths,
    0,
  );
  return { ...next, status: paths.length > 0 ? "stopped" : "idle" };
}

function clamp(value: number, minimum: number, maximum: number) {
  return Math.max(minimum, Math.min(value, maximum));
}

function mergePlaybackSnapshot(
  previous: PlaybackSnapshot,
  next: PlaybackSnapshot,
): PlaybackSnapshot {
  const queue = sameQueue(previous.queue, next.queue) ? previous.queue : next.queue;
  const output = sameOutput(previous.output, next.output) ? previous.output : next.output;
  const error = sameFailure(previous.error, next.error) ? previous.error : next.error;
  if (
    previous.status === next.status
    && previous.path === next.path
    && previous.positionMs === next.positionMs
    && previous.durationMs === next.durationMs
    && previous.volume === next.volume
    && previous.seekable === next.seekable
    && previous.currentItemId === next.currentItemId
    && previous.playbackMode === next.playbackMode
    && previous.queue === queue
    && previous.output === output
    && previous.error === error
  ) {
    return previous;
  }
  return { ...next, queue, output, error };
}

function sameQueue(left: PlaybackSnapshot["queue"], right: PlaybackSnapshot["queue"]) {
  return left.length === right.length && left.every((item, index) => {
    const candidate = right[index];
    return item.id === candidate.id
      && item.path === candidate.path
      && item.displayName === candidate.displayName
      && item.durationMs === candidate.durationMs
      && item.status === candidate.status
      && sameFailure(item.error, candidate.error);
  });
}

function sameFailure(left: PlaybackFailure | null, right: PlaybackFailure | null) {
  return left === right || (left?.code === right?.code && left?.message === right?.message);
}

function sameActivePlaylist(
  left: ActivePlaylistSnapshot | null,
  right: ActivePlaylistSnapshot | null,
) {
  return left?.kind === right?.kind && left?.playlistId === right?.playlistId;
}

function sameOutput(left: PlaybackSnapshot["output"], right: PlaybackSnapshot["output"]) {
  return left.status === right.status
    && left.followSystemDefault === right.followSystemDefault
    && left.selectedDeviceId === right.selectedDeviceId
    && left.activeDeviceId === right.activeDeviceId
    && left.activeDeviceName === right.activeDeviceName
    && sameFailure(left.error, right.error)
    && left.devices.length === right.devices.length
    && left.devices.every((device, index) => {
      const candidate = right.devices[index];
      return device.id === candidate.id
        && device.name === candidate.name
        && device.isDefault === candidate.isDefault
        && device.interfaceType === candidate.interfaceType;
    });
}
