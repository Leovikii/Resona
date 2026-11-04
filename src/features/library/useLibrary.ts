import type { FileImportActivity } from "../../shared/bridge/useFileImportActivity";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { selectAudioFiles, selectAudioFolders } from "../../shared/bridge/audioDialog";
import { invokeTauri, isTauriRuntime } from "../../shared/bridge/tauri";
import type {
  PlaylistItem,
  PlaylistMutationResult,
  PlaylistSummary,
  RejectedPath,
} from "../../shared/model/library";
import { movePlaylistSummary } from "./playlistOrdering";

const previewPlaylists: PlaylistSummary[] = [
  { id: 1, name: "Resona demo", position: 0, itemCount: 3, createdAt: 0, updatedAt: 0 },
  { id: 2, name: "Late night", position: 1, itemCount: 1, createdAt: 0, updatedAt: 0 },
  { id: 3, name: "Focus", position: 2, itemCount: 0, createdAt: 0, updatedAt: 0 },
  { id: 4, name: "Archive", position: 3, itemCount: 0, createdAt: 0, updatedAt: 0 },
  { id: 5, name: "A very long playlist name", position: 4, itemCount: 0, createdAt: 0, updatedAt: 0 },
  { id: 6, name: "Weekend", position: 5, itemCount: 0, createdAt: 0, updatedAt: 0 },
];

const previewItems: Record<number, PlaylistItem[]> = {
  1: [
    item(1, 1, "C:\\Music\\Local Sessions\\First Light.wav", 0, "C:\\Music\\Local Sessions"),
    item(2, 1, "C:\\Music\\Local Sessions\\Disc 1\\Midnight Signal.flac", 1, "C:\\Music\\Local Sessions"),
    item(3, 1, "C:\\Music\\Local Sessions\\Disc 2\\Blue Transit.mp3", 2, "C:\\Music\\Local Sessions"),
  ],
  2: [item(4, 2, "C:\\Music\\Afterimage.flac", 0)],
};

export function useLibrary(imports: FileImportActivity) {
  const preview = import.meta.env.DEV && !isTauriRuntime();
  const [playlists, setPlaylists] = useState<PlaylistSummary[]>(preview ? previewPlaylists : []);
  const [previewItemsByPlaylist, setPreviewItemsByPlaylist] = useState<Record<number, PlaylistItem[]>>(
    () => preview ? clonePreviewItems() : {},
  );
  const [selectedPlaylistId, setSelectedPlaylistId] = useState<number | null>(null);
  const [selectedItems, setSelectedItems] = useState<PlaylistItem[]>([]);
  const selectionRef = useRef<number | null>(null);
  const itemRequestRef = useRef(0);
  const refreshRequestRef = useRef(0);
  const [loading, setLoading] = useState(false);
  const [itemsLoading, setItemsLoading] = useState(false);
  const [rejectedPaths, setRejectedPaths] = useState<RejectedPath[]>([]);
  const errorSequenceRef = useRef(0);
  const [error, setError] = useState<{ id: number; message: string; code?: string } | null>(null);
  const changeSelection = useCallback((id: number | null, items: PlaylistItem[] = []) => {
    selectionRef.current = id;
    itemRequestRef.current += 1;
    setSelectedPlaylistId(id);
    setSelectedItems(items);
    setItemsLoading(false);
  }, []);
  const acceptItems = useCallback((playlistId: number, items: PlaylistItem[]) => {
    if (selectionRef.current !== playlistId) return;
    itemRequestRef.current += 1;
    setSelectedItems(items);
  }, []);
  const reportError = useCallback((cause: unknown) => {
    errorSequenceRef.current += 1;
    const code = typeof cause === "object" && cause !== null && "code" in cause && typeof cause.code === "string" ? cause.code : undefined;
    setError({ id: errorSequenceRef.current, message: messageFrom(cause), code });
  }, []);

  const selectedPlaylist = useMemo(
    () => playlists.find((playlist) => playlist.id === selectedPlaylistId) ?? null,
    [playlists, selectedPlaylistId],
  );

  const refresh = useCallback(async () => {
    if (preview) return;
    const request = ++refreshRequestRef.current;
    setLoading(true);
    try {
      const nextPlaylists = await invokeTauri<PlaylistSummary[]>("list_playlists");
      if (request !== refreshRequestRef.current) return;
      setPlaylists(nextPlaylists);
      if (selectionRef.current !== null && !nextPlaylists.some((playlist) => playlist.id === selectionRef.current)) {
        changeSelection(null);
      }
      setError(null);
    } catch (cause) {
      if (request === refreshRequestRef.current) reportError(cause);
    } finally {
      if (request === refreshRequestRef.current) setLoading(false);
    }
  }, [changeSelection, preview, reportError]);

  useEffect(() => {
    void refresh();
    return () => { refreshRequestRef.current += 1; itemRequestRef.current += 1; };
  }, [refresh]);

  const selectPlaylist = useCallback(async (playlistId: number | null) => {
    changeSelection(playlistId);
    const request = itemRequestRef.current;
    if (playlistId === null) {
      setSelectedItems([]);
      return;
    }
    if (preview) {
      setSelectedItems(previewItemsByPlaylist[playlistId] ?? []);
      return;
    }
    setItemsLoading(true);
    try {
      const items = await invokeTauri<PlaylistItem[]>("list_playlist_items", { playlistId });
      if (request !== itemRequestRef.current) return;
      setSelectedItems(items);
      setError(null);
    } catch (cause) {
      if (request === itemRequestRef.current) {
        setSelectedItems([]);
        reportError(cause);
      }
    } finally {
      if (request === itemRequestRef.current) setItemsLoading(false);
    }
  }, [changeSelection, preview, previewItemsByPlaylist, reportError]);

  const reportMutationError = useCallback(async (cause: unknown) => {
    if (typeof cause === "object" && cause !== null && "code" in cause && cause.code === "playback_sequence_sync_failed") {
      await refresh();
      if (selectionRef.current !== null) await selectPlaylist(selectionRef.current);
    }
    reportError(cause);
  }, [refresh, reportError, selectPlaylist]);

  const createPlaylist = useCallback(async ({
    ensureUniqueName = false,
    name,
    paths = [],
    position = null,
    requireItems = false,
  }: {
    ensureUniqueName?: boolean;
    name: string;
    paths?: string[];
    position?: number | null;
    requireItems?: boolean;
  }) => imports.run(async () => {
    const selection = selectionRef.current;
    if (paths.length > 0) setRejectedPaths([]);
    setLoading(true);
    try {
      let result: PlaylistMutationResult;
      if (preview) {
        const id = Math.max(0, ...playlists.map((playlist) => playlist.id)) + 1;
        const insertion = position === null ? playlists.length : clamp(position, 0, playlists.length);
        const resolvedName = uniquePreviewName(name, playlists, ensureUniqueName);
        const playlist: PlaylistSummary = {
          id,
          name: resolvedName,
          position: insertion,
          itemCount: paths.length,
          createdAt: 0,
          updatedAt: 0,
        };
        const nextItemId = nextPreviewItemId(previewItemsByPlaylist);
        const items = paths.map((path, itemPosition) =>
          item(nextItemId + itemPosition, id, path, itemPosition));
        setPlaylists((current) => {
          const next = [...current];
          next.splice(insertion, 0, playlist);
          return next.map((candidate, listPosition) => ({ ...candidate, position: listPosition }));
        });
        setPreviewItemsByPlaylist((current) => ({ ...current, [id]: items }));
        result = { playlist, items, rejected: [] };
      } else {
        result = await invokeTauri<PlaylistMutationResult>("create_playlist", {
          ensureUniqueName,
          name,
          paths,
          position,
          requireItems,
        });
        await refresh();
      }
      if (selectionRef.current === selection) changeSelection(result.playlist.id, result.items);
      setRejectedPaths(result.rejected);
      setError(null);
      return result;
    } catch (cause) {
      reportError(cause);
      return null;
    } finally {
      setLoading(false);
    }
  }), [changeSelection, imports.run, playlists, preview, previewItemsByPlaylist, refresh, reportError]);

  const renamePlaylist = useCallback(async (id: number, name: string) => {
    setLoading(true);
    try {
      if (preview) {
        setPlaylists((current) => current.map((playlist) =>
          playlist.id === id ? { ...playlist, name } : playlist));
      } else {
        await invokeTauri("rename_playlist", { id, name });
        await refresh();
      }
      setError(null);
      return true;
    } catch (cause) {
      reportError(cause);
      return false;
    } finally {
      setLoading(false);
    }
  }, [preview, refresh, reportError]);

  const deletePlaylist = useCallback(async (id: number) => {
    setLoading(true);
    try {
      if (!preview) await invokeTauri("delete_playlist", { id });
      else {
        setPreviewItemsByPlaylist((current) => {
          const { [id]: _deleted, ...remaining } = current;
          return remaining;
        });
      }
      setPlaylists((current) => current
        .filter((playlist) => playlist.id !== id)
        .map((playlist, position) => ({ ...playlist, position })));
      if (selectionRef.current === id) changeSelection(null);
      if (!preview) await refresh();
      setError(null);
      return true;
    } catch (cause) {
      await reportMutationError(cause);
      return false;
    } finally {
      setLoading(false);
    }
  }, [changeSelection, preview, refresh, reportMutationError]);

  const deleteOtherPlaylists = useCallback(async (keepId: number | null) => {
    setLoading(true);
    try {
      const nextPlaylists = preview
        ? playlists
          .filter((playlist) => playlist.id === keepId)
          .map((playlist, position) => ({ ...playlist, position }))
        : await invokeTauri<PlaylistSummary[]>("delete_other_playlists", { keepId });
      if (preview) {
        const remainingIds = new Set(nextPlaylists.map((playlist) => playlist.id));
        setPreviewItemsByPlaylist((current) => Object.fromEntries(
          Object.entries(current).filter(([id]) => remainingIds.has(Number(id))),
        ));
      }
      setPlaylists(nextPlaylists);
      if (selectionRef.current !== keepId) changeSelection(null);
      setError(null);
      return true;
    } catch (cause) {
      await reportMutationError(cause);
      return false;
    } finally {
      setLoading(false);
    }
  }, [changeSelection, playlists, preview, reportMutationError]);

  const movePlaylist = useCallback(async (id: number, toPosition: number) => {
    setLoading(true);
    try {
      if (preview) {
        setPlaylists((current) => movePlaylistSummary(current, id, toPosition));
      } else {
        setPlaylists(await invokeTauri<PlaylistSummary[]>("move_playlist", { id, toPosition }));
      }
      setError(null);
      return true;
    } catch (cause) {
      await reportMutationError(cause);
      return false;
    } finally {
      setLoading(false);
    }
  }, [preview, reportMutationError]);

  const addItems = useCallback(async (
    playlistId: number,
    paths: string[],
    position: number | null = null,
  ) => imports.run(async () => {
    if (paths.length === 0) return null;
    setRejectedPaths([]);
    setItemsLoading(true);
    try {
      let result: PlaylistMutationResult;
      if (preview) {
        const currentItems = previewItemsByPlaylist[playlistId] ?? [];
        const insertion = position === null
          ? currentItems.length
          : clamp(position, 0, currentItems.length);
        const next = [...currentItems];
        const nextId = nextPreviewItemId(previewItemsByPlaylist);
        next.splice(insertion, 0, ...paths.map((path, offset) =>
          item(nextId + offset, playlistId, path, insertion + offset)));
        const items = next.map((candidate, itemPosition) => ({ ...candidate, position: itemPosition }));
        const playlist = playlists.find((candidate) => candidate.id === playlistId);
        if (!playlist) throw new Error("Playlist not found");
        result = { playlist: { ...playlist, itemCount: items.length }, items, rejected: [] };
        setPlaylists((current) => current.map((candidate) =>
          candidate.id === playlistId ? result.playlist : candidate));
        setPreviewItemsByPlaylist((current) => ({ ...current, [playlistId]: items }));
      } else {
        result = await invokeTauri<PlaylistMutationResult>("add_playlist_items", {
          playlistId,
          paths,
          position,
        });
        await refresh();
      }
      acceptItems(playlistId, result.items);
      setRejectedPaths(result.rejected);
      setError(null);
      return result;
    } catch (cause) {
      await reportMutationError(cause);
      return null;
    } finally {
      if (selectionRef.current === playlistId) setItemsLoading(false);
    }
  }), [acceptItems, imports.run, playlists, preview, previewItemsByPlaylist, refresh, reportMutationError]);

  const chooseAndAddItems = useCallback(async (playlistId: number, position: number | null = null) => {
    try {
      const paths = preview
        ? ["C:\\Music\\New track.flac"]
        : await selectAudioFiles(true);
      return addItems(playlistId, paths, position);
    } catch (cause) { await reportMutationError(cause); return null; }
  }, [addItems, preview, reportMutationError]);

  const chooseAndAddFolders = useCallback(async (playlistId: number) => {
    try {
      const paths = preview
        ? ["C:\\Music\\Imported\\Folder track.flac"]
        : await selectAudioFolders();
      return addItems(playlistId, paths);
    } catch (cause) { await reportMutationError(cause); return null; }
  }, [addItems, preview, reportMutationError]);

  const removeItem = useCallback(async (playlistId: number, itemId: number) => {
    setItemsLoading(true);
    try {
      if (preview) {
        const items = (previewItemsByPlaylist[playlistId] ?? [])
          .filter((candidate) => candidate.id !== itemId)
          .map((candidate, position) => ({ ...candidate, position }));
        setPreviewItemsByPlaylist((current) => ({ ...current, [playlistId]: items }));
        acceptItems(playlistId, items);
        setPlaylists((current) => current.map((playlist) =>
          playlist.id === playlistId ? { ...playlist, itemCount: items.length } : playlist));
      } else {
        acceptItems(playlistId, await invokeTauri<PlaylistItem[]>("remove_playlist_item", {
          playlistId,
          itemId,
        }));
        await refresh();
      }
      setError(null);
      return true;
    } catch (cause) {
      await reportMutationError(cause);
      return false;
    } finally {
      if (selectionRef.current === playlistId) setItemsLoading(false);
    }
  }, [acceptItems, preview, previewItemsByPlaylist, refresh, reportMutationError]);

  const removeItems = useCallback(async (playlistId: number, itemIds: number[]) => {
    if (itemIds.length === 0) return false;
    setItemsLoading(true);
    try {
      let items: PlaylistItem[];
      if (preview) {
        const selected = new Set(itemIds);
        items = (previewItemsByPlaylist[playlistId] ?? [])
          .filter((candidate) => !selected.has(candidate.id))
          .map((candidate, position) => ({ ...candidate, position }));
        setPreviewItemsByPlaylist((current) => ({ ...current, [playlistId]: items }));
      } else {
        items = await invokeTauri<PlaylistItem[]>("remove_playlist_items", { playlistId, itemIds });
        await refresh();
      }
      acceptItems(playlistId, items);
      setPlaylists((current) => current.map((playlist) => playlist.id === playlistId
        ? { ...playlist, itemCount: items.length }
        : playlist));
      setError(null);
      return true;
    } catch (cause) {
      await reportMutationError(cause);
      return false;
    } finally {
      if (selectionRef.current === playlistId) setItemsLoading(false);
    }
  }, [acceptItems, preview, previewItemsByPlaylist, refresh, reportMutationError]);

  const clearItems = useCallback(async (playlistId: number) => {
    setItemsLoading(true);
    try {
      const items = preview
        ? []
        : await invokeTauri<PlaylistItem[]>("clear_playlist_items", { playlistId });
      if (preview) setPreviewItemsByPlaylist((current) => ({ ...current, [playlistId]: [] }));
      else await refresh();
      acceptItems(playlistId, items);
      setPlaylists((current) => current.map((playlist) => playlist.id === playlistId
        ? { ...playlist, itemCount: 0 }
        : playlist));
      setError(null);
      return true;
    } catch (cause) {
      await reportMutationError(cause);
      return false;
    } finally {
      if (selectionRef.current === playlistId) setItemsLoading(false);
    }
  }, [acceptItems, preview, refresh, reportMutationError]);

  const moveItem = useCallback(async (
    playlistId: number,
    itemId: number,
    toPosition: number,
  ) => {
    setItemsLoading(true);
    try {
      if (preview) {
        const next = [...(previewItemsByPlaylist[playlistId] ?? [])];
        const from = next.findIndex((candidate) => candidate.id === itemId);
        if (from < 0) return false;
        const [moved] = next.splice(from, 1);
        next.splice(clamp(toPosition, 0, next.length), 0, moved);
        const items = next.map((candidate, position) => ({ ...candidate, position }));
        setPreviewItemsByPlaylist((current) => ({ ...current, [playlistId]: items }));
        acceptItems(playlistId, items);
      } else {
        acceptItems(playlistId, await invokeTauri<PlaylistItem[]>("move_playlist_item", {
          playlistId,
          itemId,
          toPosition,
        }));
      }
      setError(null);
      return true;
    } catch (cause) {
      await reportMutationError(cause);
      return false;
    } finally {
      if (selectionRef.current === playlistId) setItemsLoading(false);
    }
  }, [acceptItems, preview, previewItemsByPlaylist, reportMutationError]);

  const dismissRejected = useCallback(() => setRejectedPaths([]), []);

  return {
    addItems,
    chooseAndAddFolders,
    chooseAndAddItems,
    clearItems,
    createPlaylist,
    deleteOtherPlaylists,
    deletePlaylist,
    dismissRejected,
    error,
    itemsLoading,
    loading,
    moveItem,
    movePlaylist,
    playlists,
    refresh,
    rejectedCount: rejectedPaths.length,
    rejectedPaths,
    removeItem,
    removeItems,
    renamePlaylist,
    selectPlaylist,
    selectedItems,
    selectedPlaylist,
    selectedPlaylistId,
  };
}

function item(
  id: number,
  playlistId: number,
  path: string,
  position: number,
  folderRoot: string | null = null,
): PlaylistItem {
  return {
    id,
    playlistId,
    path,
    displayName: path.split(/[\\/]/).pop() ?? path,
    position,
    folderRoot,
  };
}

function clonePreviewItems() {
  return Object.fromEntries(Object.entries(previewItems).map(([playlistId, items]) => [
    Number(playlistId),
    items.map((candidate) => ({ ...candidate })),
  ]));
}

function nextPreviewItemId(itemsByPlaylist: Record<number, PlaylistItem[]>) {
  return Math.max(
    0,
    ...Object.values(itemsByPlaylist).flatMap((items) => items.map((candidate) => candidate.id)),
  ) + 1;
}

function uniquePreviewName(name: string, playlists: PlaylistSummary[], ensureUnique: boolean) {
  if (!ensureUnique || !playlists.some((playlist) => playlist.name === name)) return name;
  let suffix = 2;
  while (playlists.some((playlist) => playlist.name === `${name} ${suffix}`)) suffix += 1;
  return `${name} ${suffix}`;
}

function clamp(value: number, minimum: number, maximum: number) {
  return Math.max(minimum, Math.min(value, maximum));
}

function messageFrom(cause: unknown) {
  if (typeof cause === "object" && cause !== null && "message" in cause) {
    return String((cause as { message: unknown }).message);
  }
  return String(cause);
}
