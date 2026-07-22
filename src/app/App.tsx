import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  ActionIcon,
  Badge,
  Button,
  ColorInput,
  Divider,
  Group,
  Loader,
  Menu,
  Modal,
  NumberInput,
  Paper,
  Popover,
  Portal,
  Progress,
  ScrollArea,
  SegmentedControl,
  Select,
  Slider,
  Stack,
  Switch,
  Text,
  TextInput,
  ThemeIcon,
  Title,
  Tooltip,
  UnstyledButton,
  useMantineColorScheme,
} from "@mantine/core";
import {
  Captions,
  ArrowRight,
  Check,
  Disc3,
  FileAudio,
  History,
  ListMusic,
  Maximize2,
  Minimize2,
  Music2,
  Pause,
  Play,
  Plus,
  RefreshCw,
  Repeat,
  Repeat1,
  Settings,
  SkipBack,
  SkipForward,
  Shuffle,
  Tags,
  Trash2,
  Volume2,
  Wrench,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { useDesktopLyricsWindow } from "../features/lyrics/useDesktopLyricsWindow";
import { useAudioCompression } from "../features/compression/useAudioCompression";
import { useLibrary } from "../features/library/useLibrary";
import { useTrackDetails } from "../features/metadata/useTrackDetails";
import { showAudioCompressionWindow } from "../shared/bridge/compressionWindow";
import { usePlaybackController } from "../features/playback/usePlaybackController";
import { useSeekTransaction } from "../features/playback/useSeekTransaction";
import { useMainWindowLayout } from "../features/window/useMainWindowLayout";
import { isTauriRuntime } from "../shared/bridge/tauri";
import type {
  DefaultPlaylistItem,
  DefaultPlaylistSnapshot,
  PlaylistItem,
  PlaylistSummary,
  RecentPlayRecord,
} from "../shared/model/library";
import type { PlaybackFailure, PlaybackSnapshot } from "../shared/model/playback";
import type { LyricsSnapshot } from "../shared/model/lyrics";
import type { TrackDetails } from "../shared/model/metadata";
import { fileNameFromPath, formatDuration } from "../shared/utils/format";
import { listInsertionPositionAtY } from "../shared/ui/usePointerReorder";
import { PlaylistTrackList } from "../shared/ui/PlaylistTrackList";
import { AddMediaMenu } from "../shared/ui/AddMediaMenu";
import { CompactTopNavigation, type CompactNavigationSelection } from "../shared/ui/CompactTopNavigation";
import { OverflowMarquee } from "../shared/ui/OverflowMarquee";
import { accentColors, type AccentColor, usePreferences } from "./preferences";

type Selection = CompactNavigationSelection;

type DropTarget =
  | { kind: "default" }
  | { kind: "default-track-gap"; position: number }
  | { kind: "playlist"; playlistId: number }
  | { kind: "playlist-gap"; position: number }
  | { kind: "track-gap"; playlistId: number; position: number };

export default function App() {
  const { t } = useTranslation();
  const playback = usePlaybackController();
  const library = useLibrary();
  const desktopLyrics = useDesktopLyricsWindow();
  const { desktopLyrics: lyricsPreferences, setDesktopLyrics } = usePreferences();
  const mainWindow = useMainWindowLayout();
  const [selection, setSelection] = useState<Selection>({ kind: "default" });
  const [createPlaylistOpen, setCreatePlaylistOpen] = useState(false);
  const [playlistName, setPlaylistName] = useState("");
  const [dropTarget, setDropTarget] = useState<DropTarget | null>(null);
  const [externalDragActive, setExternalDragActive] = useState(false);
  const [playerExpanded, setPlayerExpanded] = useState(false);
  const dropActionsRef = useRef({ library, playback, t });
  const autoLyricsPathRef = useRef<string | null>(null);
  dropActionsRef.current = { library, playback, t };

  const hasCurrentTrack = playback.snapshot.currentItemId !== null
    && playback.selectedPath !== null;
  const currentTitle = useMemo(
    () => fileNameFromPath(playback.selectedPath) || t("playback.noTrack"),
    [playback.selectedPath, t],
  );
  const trackDetails = useTrackDetails(playback.selectedPath);
  const runSeek = useCallback(
    (positionMs: number) => playback.run("seek_playback", { positionMs }),
    [playback.run],
  );
  const seek = useSeekTransaction(playback.snapshot.positionMs, runSeek);
  const compact = mainWindow.snapshot.layoutMode === "compact";

  useEffect(() => {
    if (!hasCurrentTrack) setPlayerExpanded(false);
  }, [hasCurrentTrack]);

  useEffect(() => {
    const path = playback.selectedPath;
    if (!isTauriRuntime() || !lyricsPreferences.enabled || !hasCurrentTrack || !path) {
      if (!path) autoLyricsPathRef.current = null;
      return;
    }
    if (autoLyricsPathRef.current === path || desktopLyrics.snapshot.visible) return;
    autoLyricsPathRef.current = path;
    void desktopLyrics.run("show_desktop_lyrics_window", { fontSize: lyricsPreferences.fontSize });
  }, [desktopLyrics, hasCurrentTrack, lyricsPreferences.enabled, lyricsPreferences.fontSize, playback.selectedPath]);

  useEffect(() => {
    if (playback.openSequence > 0) setSelection({ kind: "default" });
  }, [playback.openSequence]);

  useEffect(() => {
    if (!isTauriRuntime() || playback.defaultPlaylist.revision === 0) return;
    if (playback.snapshot.path) setSelection({ kind: "default" });
  }, [playback.defaultPlaylist.revision, playback.snapshot.path]);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen("resona://open-settings", () => {
      setPlayerExpanded(false);
      setSelection({ kind: "settings" });
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    const suppressBrowserMenu = (event: MouseEvent) => {
      const target = event.target;
      if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement || (target instanceof HTMLElement && target.isContentEditable)) return;
      event.preventDefault();
    };
    window.addEventListener("contextmenu", suppressBrowserMenu);
    return () => window.removeEventListener("contextmenu", suppressBrowserMenu);
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    let unlistenDrop: (() => void) | undefined;
    let unlistenScale: (() => void) | undefined;
    let scaleFactor = window.devicePixelRatio || 1;
    const currentWindow = getCurrentWindow();
    void currentWindow.scaleFactor().then((factor) => {
      scaleFactor = factor;
    }).catch((error) => {
      console.warn("Unable to read the window scale factor for file drop targeting", error);
    });
    void currentWindow.onScaleChanged(({ payload }) => {
      scaleFactor = payload.scaleFactor;
    }).then((stop) => {
      if (disposed) stop();
      else unlistenScale = stop;
    }).catch((error) => {
      console.error("Unable to listen for window scale changes", error);
    });
    void getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "leave") {
        setDropTarget(null);
        setExternalDragActive(false);
        return;
      }
      const target = dropTargetAtPosition(
        event.payload.position.x / scaleFactor,
        event.payload.position.y / scaleFactor,
      );
      if (event.payload.type === "enter" || event.payload.type === "over") {
        setExternalDragActive(true);
        setDropTarget((current) => dropTargetsEqual(current, target) ? current : target);
        return;
      }
      setDropTarget(null);
      setExternalDragActive(false);
      if (!target || event.payload.paths.length === 0) return;
      const { library: actions, playback: playbackActions, t: translate } = dropActionsRef.current;
      if (target.kind === "default" || target.kind === "default-track-gap") {
        const position = target.kind === "default-track-gap" ? target.position : null;
        void playbackActions.addDefaultItems(event.payload.paths, position).then((result) => {
          if (result) setSelection({ kind: "default" });
        });
      } else if (target.kind === "playlist-gap") {
        void actions.createPlaylist({
          ensureUniqueName: true,
          name: translate("library.untitled"),
          paths: event.payload.paths,
          position: target.position,
          requireItems: true,
        }).then((result) => {
          if (!result) return;
          setSelection({ kind: "user", playlistId: result.playlist.id });
        });
      } else {
        const position = target.kind === "track-gap" ? target.position : null;
        void actions.addItems(target.playlistId, event.payload.paths, position).then((result) => {
          if (!result) return;
          setSelection({ kind: "user", playlistId: target.playlistId });
          void playbackActions.refresh();
        });
      }
    }).then((stop) => {
      if (disposed) stop();
      else unlistenDrop = stop;
    }).catch((error) => {
      console.error("Unable to listen for native file drops", error);
    });
    return () => {
      disposed = true;
      unlistenDrop?.();
      unlistenScale?.();
    };
  }, []);

  const selectUserPlaylist = useCallback((playlistId: number) => {
    setSelection({ kind: "user", playlistId });
    void library.selectPlaylist(playlistId);
  }, [library.selectPlaylist]);
  const openCreatePlaylist = useCallback(() => {
    setPlaylistName("");
    setCreatePlaylistOpen(true);
  }, []);
  const selectNavigation = useCallback((next: Selection) => {
    setPlayerExpanded(false);
    if (next.kind === "user") selectUserPlaylist(next.playlistId);
    else setSelection(next);
  }, [selectUserPlaylist]);

  const closePlayer = useCallback(() => setPlayerExpanded(false), []);
  const togglePlayer = useCallback(() => setPlayerExpanded((value) => !value), []);
  const selectedUserId = selection.kind === "user" ? selection.playlistId : null;
  const addUserFiles = useCallback(() => {
    if (selectedUserId === null) return;
    void library.chooseAndAddItems(selectedUserId).then((result) => {
      if (result) void playback.refresh();
    });
  }, [library.chooseAndAddItems, playback.refresh, selectedUserId]);
  const addUserFolders = useCallback(() => {
    if (selectedUserId === null) return;
    void library.chooseAndAddFolders(selectedUserId).then((result) => {
      if (result) void playback.refresh();
    });
  }, [library.chooseAndAddFolders, playback.refresh, selectedUserId]);
  const deleteUserPlaylist = useCallback((playlistId: number) => {
    void library.deletePlaylist(playlistId).then(async (deleted) => {
      if (!deleted) return;
      await Promise.all([playback.refresh(), playback.reloadDefaultPlaylist()]);
      setSelection({ kind: "default" });
    });
  }, [library.deletePlaylist, playback.refresh, playback.reloadDefaultPlaylist]);
  const moveUserItem = useCallback((itemId: number, toPosition: number) => {
    if (selectedUserId === null) return;
    void library.moveItem(selectedUserId, itemId, toPosition).then((moved) => {
      if (moved) void playback.refresh();
    });
  }, [library.moveItem, playback.refresh, selectedUserId]);
  const playUserItem = useCallback((itemId: number) => {
    if (selectedUserId !== null) {
      void playback.playUserPlaylistItem(selectedUserId, itemId, library.selectedItems);
    }
  }, [library.selectedItems, playback.playUserPlaylistItem, selectedUserId]);
  const removeUserItems = useCallback((itemIds: number[]) => {
    if (selectedUserId === null) return;
    void library.removeItems(selectedUserId, itemIds).then((removed) => {
      if (removed) void playback.refresh();
    });
  }, [library.removeItems, playback.refresh, selectedUserId]);
  const clearUserItems = useCallback(() => {
    if (selectedUserId === null || !window.confirm(t("library.clearConfirm"))) return;
    void library.clearItems(selectedUserId).then((cleared) => {
      if (cleared) void playback.refresh();
    });
  }, [library.clearItems, playback.refresh, selectedUserId, t]);
  const clearDefaultItems = useCallback(() => {
    if (!window.confirm(t("library.clearConfirm"))) return;
    void playback.clearDefaultPlaylist();
  }, [playback.clearDefaultPlaylist, t]);
  const renameUserPlaylist = useCallback((name: string) => selectedUserId === null
    ? Promise.resolve(false)
    : library.renamePlaylist(selectedUserId, name), [library.renamePlaylist, selectedUserId]);
  const renamePlaylistById = useCallback((playlistId: number, name: string) => {
    void library.renamePlaylist(playlistId, name);
  }, [library.renamePlaylist]);
  const requestRenamePlaylist = useCallback((playlistId: number) => {
    const playlist = library.playlists.find((candidate) => candidate.id === playlistId);
    const name = window.prompt(t("library.name"), playlist?.name ?? "")?.trim();
    if (name) renamePlaylistById(playlistId, name);
  }, [library.playlists, renamePlaylistById, t]);
  const refreshOutputs = useCallback(() => {
    void playback.run("refresh_output_devices");
  }, [playback.run]);
  const selectOutput = useCallback((deviceId: string | null) => {
    void playback.run("select_output_device", { deviceId });
  }, [playback.run]);

  const selectedPlaylist = selection.kind === "user"
    ? library.playlists.find((playlist) => playlist.id === selection.playlistId) ?? null
    : null;
  const sidebar = (
    <MemoSidebar
      activePlaylist={playback.activePlaylist}
      dropTarget={dropTarget}
      externalDragActive={externalDragActive}
      onCreate={openCreatePlaylist}
      onClearDefault={clearDefaultItems}
      onDeletePlaylist={deleteUserPlaylist}
      onRenamePlaylist={renamePlaylistById}
      onSelect={selectNavigation}
      playlists={library.playlists}
      selection={selection}
    />
  );
  return (
    <main className="app-shell" data-layout={mainWindow.snapshot.layoutMode}>
      {compact ? (
        <>
          <CompactTopNavigation
            activePlaylist={playback.activePlaylist}
            onClearDefault={clearDefaultItems}
            onCreatePlaylist={openCreatePlaylist}
            onDeletePlaylist={deleteUserPlaylist}
            onRenamePlaylist={requestRenamePlaylist}
            onSelect={selectNavigation}
            playlists={library.playlists}
            selection={selection}
          />
        </>
      ) : sidebar}

      <section className="main-region" data-player-expanded={playerExpanded || undefined}>
        {playerExpanded && hasCurrentTrack ? (
          <MemoFullPlayerView
            details={trackDetails.details}
            error={trackDetails.error}
            loading={trackDetails.loading}
            lyrics={playback.lyrics}
            onSeek={seek.requestSeek}
            onClose={closePlayer}
            output={playback.snapshot.output}
            seekable={playback.snapshot.seekable && (playback.snapshot.status === "playing" || playback.snapshot.status === "paused")}
            title={currentTitle}
          />
        ) : (
          <section className="content-viewport">
          <div className="view-transition" key={`${selection.kind}-${selectedPlaylist?.id ?? ""}`}>
            {selection.kind === "recent" && (
              <MemoRecentView
                error={library.error}
                loading={library.loading}
                onPlay={playback.openPath}
                records={library.recent}
              />
            )}
            {selection.kind === "default" && (
              <MemoDefaultPlaylistView
                busy={playback.busy}
                currentPath={playback.selectedPath}
                dropTarget={dropTarget}
                items={playback.defaultPlaylist.items}
                onAddFiles={playback.chooseAndAddDefaultFiles}
                onAddFolders={playback.chooseAndAddDefaultFolders}
                onPlay={playback.playDefaultItem}
                onClear={clearDefaultItems}
                onMove={playback.moveDefaultItem}
                onRemove={playback.removeDefaultItems}
                sourceDirectory={playback.defaultPlaylist.sourceDirectory}
              />
            )}
            {selection.kind === "user" && (
              <MemoUserPlaylistView
                busy={playback.busy || library.loading}
                currentPath={playback.selectedPath}
                dropTarget={dropTarget}
                externalDragActive={externalDragActive}
                items={library.selectedItems}
                itemsLoading={library.itemsLoading}
                onAddFiles={addUserFiles}
                onAddFolders={addUserFolders}
                onMove={moveUserItem}
                onPlay={playUserItem}
                onClear={clearUserItems}
                onRemove={removeUserItems}
                onRename={renameUserPlaylist}
                playlist={selectedPlaylist}
              />
            )}
            {selection.kind === "tools" && <MemoToolsView />}
            {selection.kind === "settings" && (
              <MemoSettingsView
                busy={playback.busy}
                desktopLyrics={desktopLyrics}
                layoutBusy={mainWindow.busy}
                layoutMode={mainWindow.snapshot.layoutMode}
                onRefresh={refreshOutputs}
                onSetLayoutMode={mainWindow.setLayoutMode}
                onSelectOutput={selectOutput}
                output={playback.snapshot.output}
              />
            )}
          </div>
          </section>
        )}

        {(playback.snapshot.error ?? playback.refreshError) && (
          <ErrorBanner failure={(playback.snapshot.error ?? playback.refreshError)!} />
        )}
        {library.error && <div className="error-banner" role="alert">{library.error}</div>}
        {mainWindow.error && <div className="error-banner" role="alert">{mainWindow.error.message}</div>}
        {library.rejectedCount + playback.defaultRejectedCount > 0 && (
          <div className="import-notice">
            {t("import.rejected", {
              count: library.rejectedCount + playback.defaultRejectedCount,
            })}
          </div>
        )}
      </section>

      <PlayerBar
        busy={playback.busy}
        desktopLyrics={desktopLyrics}
        details={trackDetails.details}
        hasCurrentTrack={hasCurrentTrack}
        expanded={playerExpanded}
        onToggleExpanded={togglePlayer}
        onRun={playback.run}
        seek={seek}
        snapshot={playback.snapshot}
        title={currentTitle}
        compact={compact}
      />

      <Modal
        centered
        onClose={() => setCreatePlaylistOpen(false)}
        opened={createPlaylistOpen}
        title={t("library.create")}
      >
        <Stack>
          <TextInput
            autoFocus
            label={t("library.name")}
            onChange={(event) => setPlaylistName(event.currentTarget.value)}
            onKeyDown={(event) => {
              if (event.key !== "Enter") return;
              event.preventDefault();
              const name = playlistName.trim();
              if (!name || library.loading) return;
              void library.createPlaylist({ name }).then((result) => {
                if (!result) return;
                setSelection({ kind: "user", playlistId: result.playlist.id });
                setCreatePlaylistOpen(false);
              });
            }}
            value={playlistName}
          />
          <Group justify="flex-end">
            <Button onClick={() => setCreatePlaylistOpen(false)} variant="default">
              {t("common.cancel")}
            </Button>
            <Button
              disabled={!playlistName.trim() || library.loading}
              loading={library.loading}
              onClick={() => {
                const name = playlistName.trim();
                if (!name) return;
                void library.createPlaylist({ name }).then((result) => {
                  if (!result) return;
                  setSelection({ kind: "user", playlistId: result.playlist.id });
                  setCreatePlaylistOpen(false);
                });
              }}
            >
              {t("common.create")}
            </Button>
          </Group>
        </Stack>
      </Modal>
    </main>
  );
}

function Sidebar({
  activePlaylist,
  dropTarget,
  externalDragActive,
  onCreate,
  onClearDefault,
  onDeletePlaylist,
  onRenamePlaylist,
  onSelect,
  playlists,
  selection,
}: {
  activePlaylist: ReturnType<typeof usePlaybackController>["activePlaylist"];
  dropTarget: DropTarget | null;
  externalDragActive: boolean;
  onCreate: () => void;
  onClearDefault: () => void;
  onDeletePlaylist: (playlistId: number) => void;
  onRenamePlaylist: (playlistId: number, name: string) => void;
  onSelect: (selection: Selection) => void;
  playlists: PlaylistSummary[];
  selection: Selection;
}) {
  const { t } = useTranslation();
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; playlist: PlaylistSummary | null; isDefault: boolean } | null>(null);
  const contextMenuRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    const dismiss = (event: PointerEvent) => {
      if (event.target instanceof Node && contextMenuRef.current?.contains(event.target)) return;
      setContextMenu(null);
    };
    window.addEventListener("pointerdown", dismiss);
    return () => window.removeEventListener("pointerdown", dismiss);
  }, []);
  return (
    <aside className="sidebar">
      <BrandLockup />

      <nav className="sidebar-primary" aria-label={t("app.name")}>
        <NavButton
          active={selection.kind === "recent"}
          icon={<History />}
          label={t("nav.recent")}
          onClick={() => onSelect({ kind: "recent" })}
        />
        <Divider className="nav-divider" />
        <div className="playlist-nav-heading">
          <Text c="dimmed" fw={650} size="xs">{t("nav.playlists")}</Text>
          <Tooltip label={t("library.create")}>
            <ActionIcon aria-label={t("library.create")} onClick={onCreate} size="sm" variant="subtle">
              <Plus size={15} />
            </ActionIcon>
          </Tooltip>
        </div>
        <PlaylistNavItem
          active={selection.kind === "default"}
          icon={<Disc3 />}
          label={t("library.default")}
          playing={activePlaylist?.kind === "default"}
          onClick={() => onSelect({ kind: "default" })}
          onContextMenu={(event) => { event.preventDefault(); setContextMenu({ x: event.clientX, y: event.clientY, playlist: null, isDefault: true }); }}
        />
        <ScrollArea className="playlist-nav-scroll" type="auto">
          <div className="playlist-nav-list" data-external-drag={externalDragActive || undefined}>
            <PlaylistGap active={dropTarget?.kind === "playlist-gap" && dropTarget.position === 0} position={0} />
            {playlists.map((playlist, index) => (
              <div key={playlist.id}>
                <PlaylistNavItem
                  active={selection.kind === "user" && selection.playlistId === playlist.id}
                  dropActive={dropTarget?.kind === "playlist" && dropTarget.playlistId === playlist.id}
                  icon={<ListMusic />}
                  label={playlist.name}
                  onClick={() => onSelect({ kind: "user", playlistId: playlist.id })}
                  onDelete={() => onDeletePlaylist(playlist.id)}
                  onContextMenu={(event) => { event.preventDefault(); setContextMenu({ x: event.clientX, y: event.clientY, playlist, isDefault: false }); }}
                  playing={activePlaylist?.kind === "user" && activePlaylist.playlistId === playlist.id}
                  playlistId={playlist.id}
                />
                <PlaylistGap
                  active={dropTarget?.kind === "playlist-gap" && dropTarget.position === index + 1}
                  position={index + 1}
                />
              </div>
            ))}
          </div>
        </ScrollArea>
      </nav>
      {contextMenu && <Portal><Paper className="app-context-menu" ref={contextMenuRef} shadow="md" style={{ left: contextMenu.x, top: contextMenu.y }} withBorder>
        {contextMenu.isDefault ? (
          <button onClick={() => { onClearDefault(); setContextMenu(null); }} type="button">{t("common.clear")}</button>
        ) : <>
          <button onClick={() => {
            const name = window.prompt(t("library.name"), contextMenu.playlist?.name ?? "")?.trim();
            if (name && contextMenu.playlist) onRenamePlaylist(contextMenu.playlist.id, name);
            setContextMenu(null);
          }} type="button">{t("library.rename")}</button>
          <button onClick={() => { if (contextMenu.playlist) onDeletePlaylist(contextMenu.playlist.id); setContextMenu(null); }} type="button">{t("library.delete")}</button>
        </>}
      </Paper></Portal>}

      <nav className="sidebar-nav-bottom">
        <NavButton
          active={selection.kind === "tools"}
          icon={<Wrench />}
          label={t("nav.tools")}
          onClick={() => onSelect({ kind: "tools" })}
        />
        <NavButton
          active={selection.kind === "settings"}
          icon={<Settings />}
          label={t("nav.settings")}
          onClick={() => onSelect({ kind: "settings" })}
        />
      </nav>
    </aside>
  );
}

function BrandLockup() {
  const { t } = useTranslation();
  return (
    <div className="brand-lockup">
      <ThemeIcon radius="sm" size={34} variant="light">
        <Music2 size={20} strokeWidth={1.8} />
      </ThemeIcon>
      <div className="brand-copy">
        <Text fw={700}>{t("app.name")}</Text>
      </div>
    </div>
  );
}

function NavButton({ active, icon, label, onClick }: {
  active: boolean;
  icon: ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <UnstyledButton
      aria-current={active ? "page" : undefined}
      className="nav-button"
      data-active={active || undefined}
      onClick={onClick}
    >
      <span className="nav-icon">{icon}</span>
      <OverflowMarquee className="nav-label" text={label} />
    </UnstyledButton>
  );
}

function PlaylistNavItem({ active, dropActive, icon, label, onClick, onContextMenu, onDelete, playing, playlistId }: {
  active: boolean;
  dropActive?: boolean;
  icon: ReactNode;
  label: string;
  onClick: () => void;
  onContextMenu?: (event: React.MouseEvent) => void;
  onDelete?: () => void;
  playing?: boolean;
  playlistId?: number;
}) {
  const { t } = useTranslation();
  return (
    <div className="playlist-nav-item-wrap" onContextMenu={onContextMenu}>
      <UnstyledButton
        aria-current={active ? "page" : undefined}
        className="playlist-nav-item"
        data-active={active || undefined}
        data-drop-active={dropActive || undefined}
        data-drop-kind={playlistId === undefined ? undefined : "playlist"}
        data-playlist-id={playlistId}
        data-playing={playing || undefined}
        onClick={onClick}
      >
        <span className="nav-icon">{playing ? <span aria-label="Playing" className="playlist-playing-indicator"><i /><i /><i /></span> : icon}</span>
        <OverflowMarquee className="nav-label" text={label} />
      </UnstyledButton>
      {onDelete && <Tooltip label={t("library.delete")}><ActionIcon aria-label={t("library.delete")} className="playlist-nav-delete" color="red" onClick={(event) => { event.stopPropagation(); onDelete(); }} size="sm" variant="subtle"><Trash2 size={14} /></ActionIcon></Tooltip>}
    </div>
  );
}

function PlaylistGap({ active, position }: { active: boolean; position: number }) {
  return (
    <div
      className="playlist-nav-gap"
      data-active={active || undefined}
      data-drop-kind="playlist-gap"
      data-position={position}
    >
      <span />
    </div>
  );
}

function DefaultPlaylistView({
  busy,
  currentPath,
  dropTarget,
  items,
  onAddFiles,
  onAddFolders,
  onClear,
  onMove,
  onPlay,
  onRemove,
  sourceDirectory,
}: {
  busy: boolean;
  currentPath: string | null;
  dropTarget: DropTarget | null;
  items: DefaultPlaylistItem[];
  onAddFiles: () => Promise<void>;
  onAddFolders: () => Promise<void>;
  onClear: () => void;
  onMove: (itemId: number, toPosition: number) => Promise<unknown>;
  onPlay: (itemId: number) => Promise<PlaybackSnapshot | null>;
  onRemove: (itemIds: number[]) => Promise<DefaultPlaylistSnapshot | null>;
  sourceDirectory: string | null;
}) {
  const { t } = useTranslation();
  return (
    <div className="page-content list-page">
      <div className="playlist-detail-heading">
        <div className="path-heading">
          <Title order={2}>{t("library.default")}</Title>
          <OverflowMarquee
            className="path-heading-copy"
            observe={false}
            text={sourceDirectory ?? (items.length > 0
              ? t("library.multipleSources")
              : t("library.defaultEmptyHint"))}
          />
        </div>
        <AddMediaMenu
          buttonLabel={t("common.add")}
          disabled={busy}
          fileLabel={t("library.addFiles")}
          folderLabel={t("library.addFolder")}
          onAddFiles={() => void onAddFiles()}
          onAddFolders={() => void onAddFolders()}
        />
      </div>
      {items.length === 0 ? (
        <div
          className="default-playlist-drop"
          data-drop-active={dropTarget?.kind === "default" || undefined}
          data-drop-kind="default"
        >
          <EmptyView icon={<Disc3 />} label={t("library.defaultEmpty")} />
        </div>
      ) : (
        <ScrollArea
          className="track-scroll"
          data-drop-kind="default-track-list"
          type="auto"
        >
          <PlaylistTrackList
            busy={busy}
            currentPath={currentPath}
            externalInsertionPosition={dropTarget?.kind === "default-track-gap" ? dropTarget.position : null}
            items={items}
            onAddFiles={() => void onAddFiles()}
            onAddFolders={() => void onAddFolders()}
            onClear={onClear}
            onMove={(itemId, position) => void onMove(itemId, position)}
            onPlay={(itemId) => void onPlay(itemId)}
            onRemove={(itemIds) => void onRemove(itemIds)}
          />
        </ScrollArea>
      )}
    </div>
  );
}

function UserPlaylistView({
  busy,
  currentPath,
  dropTarget,
  externalDragActive,
  items,
  itemsLoading,
  onAddFiles,
  onAddFolders,
  onMove,
  onPlay,
  onClear,
  onRemove,
  onRename,
  playlist,
}: {
  busy: boolean;
  currentPath: string | null;
  dropTarget: DropTarget | null;
  externalDragActive: boolean;
  items: PlaylistItem[];
  itemsLoading: boolean;
  onAddFiles: () => void;
  onAddFolders: () => void;
  onMove: (itemId: number, toPosition: number) => void;
  onPlay: (itemId: number) => void;
  onClear: () => void;
  onRemove: (itemIds: number[]) => void;
  onRename: (name: string) => Promise<boolean>;
  playlist: PlaylistSummary | null;
}) {
  const { t } = useTranslation();
  const [name, setName] = useState(playlist?.name ?? "");
  useEffect(() => setName(playlist?.name ?? ""), [playlist?.id, playlist?.name]);

  if (!playlist) return <EmptyView icon={<ListMusic />} label={t("library.selectPrompt")} />;
  return (
    <div className="page-content list-page">
      <div className="playlist-detail-heading">
        <TextInput
          aria-label={t("library.name")}
          className="playlist-name-input"
          onBlur={() => {
            const next = name.trim();
            if (!next) setName(playlist.name);
            else if (next !== playlist.name) void onRename(next);
          }}
          onChange={(event) => setName(event.currentTarget.value)}
          value={name}
        />
        <AddMediaMenu
          buttonLabel={t("common.add")}
          disabled={busy}
          fileLabel={t("library.addFiles")}
          folderLabel={t("library.addFolder")}
          onAddFiles={onAddFiles}
          onAddFolders={onAddFolders}
        />
      </div>
      {itemsLoading ? (
        <div className="center-state"><Loader size="sm" /></div>
      ) : items.length === 0 ? (
        <div
          className="playlist-empty-drop"
          data-active={dropTarget?.kind === "track-gap" || undefined}
          data-drop-kind="track-gap"
          data-playlist-id={playlist.id}
          data-position={0}
        >
          <EmptyView icon={<ListMusic />} label={t("library.itemsEmptyDrop")} />
        </div>
      ) : (
        <ScrollArea
          className="track-scroll"
          data-drop-kind="track-list"
          data-playlist-id={playlist.id}
          type="auto"
        >
          <div>
            <PlaylistTrackList
              busy={busy}
              currentPath={currentPath}
              externalInsertionPosition={externalDragActive && dropTarget?.kind === "track-gap" && dropTarget.playlistId === playlist.id
                ? dropTarget.position
                : null}
              items={items}
              onAddFiles={onAddFiles}
              onAddFolders={onAddFolders}
              onClear={onClear}
              onMove={onMove}
              onPlay={onPlay}
              onRemove={onRemove}
            />
          </div>
        </ScrollArea>
      )}
    </div>
  );
}

function RecentView({ error, loading, onPlay, records }: {
  error: string | null;
  loading: boolean;
  onPlay: (path: string) => Promise<PlaybackSnapshot | null>;
  records: RecentPlayRecord[];
}) {
  const { t } = useTranslation();
  return (
    <div className="page-content list-page">
      <Group className="page-heading" justify="space-between">
        <div>
          <Title order={2}>{t("recent.title")}</Title>
          <Text c="dimmed" size="sm">{t("common.tracks", { count: records.length })}</Text>
        </div>
        {loading && <Loader size="xs" />}
      </Group>
      {error ? (
        <EmptyView icon={<History />} label={error} />
      ) : records.length === 0 ? (
        <EmptyView icon={<History />} label={t("recent.empty")} />
      ) : (
        <ScrollArea className="track-scroll" type="auto">
          <div className="recent-list">
            {records.map((record) => (
              <UnstyledButton className="recent-row" key={record.path} onClick={() => void onPlay(record.path)}>
                <ThemeIcon size={34} variant="light"><History size={17} /></ThemeIcon>
                <div className="track-copy">
                  <Text fw={600} lineClamp={1} size="sm">{record.displayName}</Text>
                  <Text c="dimmed" lineClamp={1} size="xs">{record.path}</Text>
                </div>
                <Text c="dimmed" size="xs">{t("library.playedAt", { count: record.playCount })}</Text>
              </UnstyledButton>
            ))}
          </div>
        </ScrollArea>
      )}
    </div>
  );
}

function ToolsView() {
  const { t } = useTranslation();
  const compression = useAudioCompression();
  const [openError, setOpenError] = useState<string | null>(null);
  const running = compression.snapshot.status === "running" || compression.snapshot.status === "cancelling";
  const progress = compression.snapshot.total > 0
    ? ((compression.snapshot.completed + compression.snapshot.currentProgress) / compression.snapshot.total) * 100
    : 0;
  return (
    <ScrollArea className="page-scroll" type="auto">
    <div className="page-content tools-page">
      <Title className="page-heading" order={2}>{t("tools.title")}</Title>
      <div className="tool-list">
        <Paper className="tool-row tool-entry" withBorder>
          <ThemeIcon size={38} variant="light"><Wrench size={19} /></ThemeIcon>
          <div className="tool-entry-copy">
            <Text fw={600}>{t("tools.compression")}</Text>
            <Text c="dimmed" size="xs">{t("tools.compressionScope")}</Text>
            {running && <Progress mt={7} size="xs" value={progress} />}
          </div>
          <Button
            onClick={() => void showAudioCompressionWindow()
              .then(() => setOpenError(null))
              .catch((error) => setOpenError(String(error)))}
            size="xs"
            variant="light"
          >{t("common.launch")}</Button>
        </Paper>
        {openError && <div className="error-banner" role="alert">{openError}</div>}
        <Paper className="tool-row" withBorder>
          <ThemeIcon color="gray" size={38} variant="light"><Tags size={19} /></ThemeIcon>
          <Text fw={600}>{t("tools.tagEditor")}</Text>
          <Badge color="gray" ml="auto" variant="light">{t("common.later")}</Badge>
        </Paper>
      </div>
    </div>
    </ScrollArea>
  );
}

function SettingsView({ busy, desktopLyrics, layoutBusy, layoutMode, onRefresh, onSetLayoutMode, onSelectOutput, output }: {
  busy: boolean;
  desktopLyrics: ReturnType<typeof useDesktopLyricsWindow>;
  layoutBusy: boolean;
  layoutMode: "wide" | "compact";
  onRefresh: () => void;
  onSetLayoutMode: (mode: "wide" | "compact") => Promise<boolean>;
  onSelectOutput: (id: string | null) => void;
  output: PlaybackSnapshot["output"];
}) {
  const { t } = useTranslation();
  const { colorScheme, setColorScheme } = useMantineColorScheme({ keepTransitions: true });
  const {
    accentColor,
    desktopLyrics: lyricsPreferences,
    locale,
    setAccentColor,
    setDesktopLyrics,
    setLocale,
  } = usePreferences();
  const [backgroundOpacity, setBackgroundOpacity] = useState(lyricsPreferences.backgroundOpacity);
  const [fontDraft, setFontDraft] = useState(String(lyricsPreferences.fontSize));
  useEffect(() => setBackgroundOpacity(lyricsPreferences.backgroundOpacity), [lyricsPreferences.backgroundOpacity]);
  useEffect(() => setFontDraft(String(lyricsPreferences.fontSize)), [lyricsPreferences.fontSize]);
  const commitFontSize = useCallback(async (raw: string) => {
    const parsed = Number(raw);
    if (!Number.isFinite(parsed)) {
      setFontDraft(String(lyricsPreferences.fontSize));
      return;
    }
    const value = Math.round(Math.min(64, Math.max(16, parsed)));
    setFontDraft(String(value));
    if (desktopLyrics.snapshot.visible) {
      const fitted = await desktopLyrics.run("fit_desktop_lyrics_window", { fontSize: value });
      if (!fitted) return;
    }
    setDesktopLyrics({ fontSize: value });
  }, [desktopLyrics, lyricsPreferences.fontSize, setDesktopLyrics]);
  return (
    <ScrollArea className="settings-scroll" type="auto">
    <div className="page-content settings-page">
      <Title className="page-heading" order={2}>{t("settings.title")}</Title>
      <section className="settings-section">
        <Text className="settings-section-title" fw={650}>{t("settings.appearance")}</Text>
        <SettingRow label={t("settings.colorScheme")}>
          <SegmentedControl
            data={[
              { label: t("settings.system"), value: "auto" },
              { label: t("settings.light"), value: "light" },
              { label: t("settings.dark"), value: "dark" },
            ]}
            onChange={(value) => setColorScheme(value as "auto" | "light" | "dark")}
            size="xs"
            value={colorScheme}
          />
        </SettingRow>
        <SettingRow label={t("settings.accent")}>
          <Group gap="xs">
            {accentColors.map((color) => (
              <button
                aria-label={color}
                className="color-swatch"
                data-active={accentColor === color || undefined}
                key={color}
                onClick={() => setAccentColor(color as AccentColor)}
                style={{ backgroundColor: `var(--mantine-color-${color}-6)` }}
                type="button"
              />
            ))}
          </Group>
        </SettingRow>
        <SettingRow label={t("settings.language")}>
          <SegmentedControl
            data={[
              { label: t("settings.languageSystem"), value: "system" },
              { label: t("settings.chinese"), value: "zh-CN" },
              { label: t("settings.english"), value: "en" },
            ]}
            onChange={(value) => setLocale(value as "system" | "zh-CN" | "en")}
            size="xs"
            value={locale}
          />
        </SettingRow>
        <SettingRow label={t("settings.windowLayout")}>
          <SegmentedControl
            data={[
              { label: t("settings.wideLayout"), value: "wide" },
              { label: t("settings.compactLayout"), value: "compact" },
            ]}
            disabled={layoutBusy}
            onChange={(value) => void onSetLayoutMode(value as "wide" | "compact")}
            size="xs"
            value={layoutMode}
          />
        </SettingRow>
      </section>

      <section className="settings-section">
        <Text className="settings-section-title" fw={650}>{t("desktopLyrics.title")}</Text>
        <SettingRow className="font-size-setting" label={t("desktopLyrics.fontSize")}>
          <NumberInput
            allowDecimal={false}
            allowNegative={false}
            aria-label={t("desktopLyrics.fontSize")}
            clampBehavior="blur"
            className="desktop-lyrics-font-input"
            hideControls
            inputMode="numeric"
            max={64}
            min={16}
            onBlur={() => void commitFontSize(fontDraft)}
            onChange={(value) => setFontDraft(String(value))}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                void commitFontSize(fontDraft);
              } else if (event.key === "Escape") {
                setFontDraft(String(lyricsPreferences.fontSize));
                event.currentTarget.blur();
              }
            }}
            onWheel={(event) => event.currentTarget.blur()}
            suffix=" px"
            value={fontDraft}
          />
        </SettingRow>
        <SettingRow className="color-setting" label={t("desktopLyrics.color")}>
          <ColorInput
            aria-label={t("desktopLyrics.color")}
            format="hex"
            onChange={(value) => setDesktopLyrics({ color: value })}
            value={lyricsPreferences.color}
            w={130}
          />
        </SettingRow>
        <SettingRow label={t("desktopLyrics.textOpacity")}>
          <Slider
            aria-label={t("desktopLyrics.textOpacity")}
            className="lyrics-opacity-setting"
            label={(value) => `${value}%`}
            max={100}
            min={10}
            onChangeEnd={(value) => setDesktopLyrics({ textOpacity: value })}
            value={lyricsPreferences.textOpacity}
          />
        </SettingRow>
        <SettingRow label={t("desktopLyrics.backgroundOpacity")}>
          <Slider
            aria-label={t("desktopLyrics.backgroundOpacity")}
            className="lyrics-opacity-setting"
            label={(value) => `${value}%`}
            max={80}
            min={0}
            onChange={setBackgroundOpacity}
            onChangeEnd={(value) => setDesktopLyrics({ backgroundOpacity: value })}
            value={backgroundOpacity}
          />
        </SettingRow>
        {desktopLyrics.error && (
          <Text c="red" size="xs">
            {t(`desktopLyrics.errors.${desktopLyrics.error.code}`, {
              defaultValue: t("desktopLyrics.unavailable"),
            })}
          </Text>
        )}
      </section>

      <section className="settings-section">
        <Text className="settings-section-title" fw={650}>{t("settings.output")}</Text>
        <SettingRow label={t("settings.device")}>
          <Group className="output-setting" gap="xs" wrap="nowrap">
            <Select
              allowDeselect={false}
              data={[
                { value: "__system_default__", label: t("settings.followDefault") },
                ...output.devices.map((device) => ({
                  value: device.id,
                  label: `${device.name}${device.isDefault ? ` (${t("settings.system")})` : ""}`,
                })),
              ]}
              disabled={busy}
              onChange={(value) => onSelectOutput(value === "__system_default__" ? null : value)}
              value={output.followSystemDefault ? "__system_default__" : output.selectedDeviceId}
            />
            <ActionIcon aria-label={t("common.refresh")} disabled={busy} onClick={onRefresh} variant="default">
              <RefreshCw size={16} />
            </ActionIcon>
          </Group>
        </SettingRow>
        <Text c={output.status === "unavailable" ? "red" : "dimmed"} size="xs">
          {output.status === "unavailable"
            ? localizeFailure(output.error, t)
            : output.activeDeviceName
              ? t("settings.activeOutput", { name: output.activeDeviceName })
              : t("settings.outputClosed")}
        </Text>
      </section>
    </div>
    </ScrollArea>
  );
}

function SettingRow({ children, className, label }: { children: ReactNode; className?: string; label: string }) {
  return (
    <div className={`setting-row${className ? ` ${className}` : ""}`}>
      <Text c="dimmed" size="sm">{label}</Text>
      <div className="setting-control">{children}</div>
    </div>
  );
}

function PlayerBar({ busy, compact, desktopLyrics, details, expanded, hasCurrentTrack, onRun, onToggleExpanded, seek, snapshot, title }: {
  busy: boolean;
  compact: boolean;
  desktopLyrics: ReturnType<typeof useDesktopLyricsWindow>;
  details: ReturnType<typeof useTrackDetails>["details"];
  expanded: boolean;
  hasCurrentTrack: boolean;
  onRun: ReturnType<typeof usePlaybackController>["run"];
  onToggleExpanded: () => void;
  seek: ReturnType<typeof useSeekTransaction>;
  snapshot: PlaybackSnapshot;
  title: string;
}) {
  const { t } = useTranslation();
  const [changingVolume, setChangingVolume] = useState(false);
  const [volume, setVolume] = useState(Math.round(snapshot.volume * 100));
  useEffect(() => {
    if (!changingVolume) setVolume(Math.round(snapshot.volume * 100));
  }, [changingVolume, snapshot.volume]);

  const canControl = snapshot.status === "playing" || snapshot.status === "paused";
  const currentId = snapshot.currentItemId ?? snapshot.queue[0]?.id;
  return (
    <footer className="player-bar" data-expanded={expanded || undefined} data-layout={compact ? "compact" : "wide"}>
      <Slider
        aria-label={t("playback.progress")}
        className="player-progress"
        disabled={!canControl || !snapshot.seekable || busy}
        label={formatDuration}
        max={Math.max(snapshot.durationMs ?? 0, 1)}
        min={0}
        onChange={seek.setDragPosition}
        onChangeEnd={(value) => void seek.requestSeek(value)}
        size="xs"
        value={Math.min(seek.displayPositionMs, Math.max(snapshot.durationMs ?? 0, 1))}
      />
      <div className="player-track">
        <UnstyledButton
          aria-label={expanded ? t("playback.collapse") : t("playback.expand")}
          className="mini-artwork"
          data-testid="player-expand"
          disabled={!hasCurrentTrack}
          onClick={onToggleExpanded}
        >
          <Disc3 className="mini-artwork-disc" size={23} />
          <span className="mini-artwork-action">
            {expanded ? <Minimize2 size={18} /> : <Maximize2 size={18} />}
          </span>
        </UnstyledButton>
        <div className="player-track-copy">
          <OverflowMarquee className="player-track-title" text={title} />
          <Text c="dimmed" size="xs">
            {formatDuration(seek.displayPositionMs)} / {formatDuration(snapshot.durationMs)}
          </Text>
        </div>
      </div>
      {compact ? (
        <CompactPlayerControls
          busy={busy}
          canControl={canControl}
          currentId={currentId}
          desktopLyrics={desktopLyrics}
          details={details}
          hasCurrentTrack={hasCurrentTrack}
          mode={snapshot.playbackMode}
          onRun={onRun}
          snapshot={snapshot}
          onVolumeChange={(value) => {
            setChangingVolume(true);
            setVolume(value);
          }}
          onVolumeChangeEnd={(value) => {
            setChangingVolume(false);
            void onRun("set_playback_volume", { volume: value / 100 });
          }}
          volume={volume}
        />
      ) : <>
        <PlaybackCoreControls
          busy={busy}
          canControl={canControl}
          currentId={currentId}
          onRun={onRun}
          snapshot={snapshot}
        />
        <div className="player-actions">
          <AudioQualityBadge quality={details?.quality ?? null} />
          <PlaybackModeButton busy={busy} mode={snapshot.playbackMode} onRun={onRun} />
          <DesktopLyricsButton busy={busy} controller={desktopLyrics} hasCurrentTrack={hasCurrentTrack} />
          <VolumeButton
            busy={busy}
            onChange={(value) => {
              setChangingVolume(true);
              setVolume(value);
            }}
            onChangeEnd={(value) => {
              setChangingVolume(false);
              void onRun("set_playback_volume", { volume: value / 100 });
            }}
            volume={volume}
          />
        </div>
      </>}
    </footer>
  );
}

function PlaybackCoreControls({ busy, canControl, currentId, onRun, snapshot }: {
  busy: boolean;
  canControl: boolean;
  currentId: number | undefined;
  onRun: ReturnType<typeof usePlaybackController>["run"];
  snapshot: PlaybackSnapshot;
}) {
  const { t } = useTranslation();
  const toggleLabel = snapshot.status === "playing"
    ? t("playback.pause")
    : snapshot.status === "paused"
      ? t("playback.resume")
      : t("playback.play");
  return (
    <Group className="player-controls" gap="xs" justify="center" wrap="nowrap">
      <Tooltip label={t("playback.previous")}>
        <ActionIcon aria-label={t("playback.previous")} disabled={!canControl || busy} onClick={() => void onRun("previous_playback")} size="xl" variant="default">
          <SkipBack fill="currentColor" size={18} />
        </ActionIcon>
      </Tooltip>
      <Tooltip label={toggleLabel}>
        <ActionIcon
          aria-label={toggleLabel}
          disabled={currentId === undefined || busy}
          onClick={() => {
            if (snapshot.status === "playing") void onRun("pause_playback");
            else if (snapshot.status === "paused") void onRun("resume_playback");
            else if (currentId !== undefined) void onRun("play_queue_item", { id: currentId });
          }}
          size="xl"
          variant="filled"
        >
          {snapshot.status === "playing" ? <Pause fill="currentColor" size={19} /> : <Play fill="currentColor" size={19} />}
        </ActionIcon>
      </Tooltip>
      <Tooltip label={t("playback.next")}>
        <ActionIcon aria-label={t("playback.next")} disabled={!canControl || busy} onClick={() => void onRun("next_playback")} size="xl" variant="default">
          <SkipForward fill="currentColor" size={18} />
        </ActionIcon>
      </Tooltip>
    </Group>
  );
}

function CompactPlayerControls({ busy, canControl, currentId, desktopLyrics, details, hasCurrentTrack, mode, onRun, onVolumeChange, onVolumeChangeEnd, snapshot, volume }: {
  busy: boolean;
  canControl: boolean;
  currentId: number | undefined;
  desktopLyrics: ReturnType<typeof useDesktopLyricsWindow>;
  details: TrackDetails | null;
  hasCurrentTrack: boolean;
  mode: PlaybackSnapshot["playbackMode"];
  onRun: ReturnType<typeof usePlaybackController>["run"];
  onVolumeChange: (value: number) => void;
  onVolumeChangeEnd: (value: number) => void;
  snapshot: PlaybackSnapshot;
  volume: number;
}) {
  return (
    <div className="compact-player-controls">
      <div className="compact-player-side compact-player-side-left">
        <AudioQualityBadge quality={details?.quality ?? null} />
      </div>
      <PlaybackCoreControls
        busy={busy}
        canControl={canControl}
        currentId={currentId}
        onRun={onRun}
        snapshot={snapshot}
      />
      <div className="compact-player-side compact-player-side-right">
        <PlaybackModeButton busy={busy} mode={mode} onRun={onRun} />
        <DesktopLyricsButton busy={busy} controller={desktopLyrics} hasCurrentTrack={hasCurrentTrack} />
        <VolumeButton
          busy={busy}
          onChange={onVolumeChange}
          onChangeEnd={onVolumeChangeEnd}
          volume={volume}
        />
      </div>
    </div>
  );
}

function VolumeButton({ busy, onChange, onChangeEnd, volume }: {
  busy: boolean;
  onChange: (value: number) => void;
  onChangeEnd: (value: number) => void;
  volume: number;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  return (
    <Popover onChange={setOpen} opened={open} position="top" shadow="md" width={190} withArrow>
      <Popover.Target>
        <Tooltip label={t("playback.volume")}>
          <ActionIcon aria-label={t("playback.volume")} disabled={busy} onClick={() => setOpen((value) => !value)} size="xl" variant="subtle">
            <Volume2 size={19} />
          </ActionIcon>
        </Tooltip>
      </Popover.Target>
      <Popover.Dropdown className="player-volume-popover">
        <Slider
          aria-label={t("playback.volume")}
          disabled={busy}
          label={(value) => `${Math.round(value)}%`}
          max={100}
          min={0}
          onChange={onChange}
          onChangeEnd={onChangeEnd}
          value={volume}
        />
      </Popover.Dropdown>
    </Popover>
  );
}

function DesktopLyricsButton({ busy, controller, hasCurrentTrack }: {
  busy: boolean;
  controller: ReturnType<typeof useDesktopLyricsWindow>;
  hasCurrentTrack: boolean;
}) {
  const { t } = useTranslation();
  const { desktopLyrics: preferences, setDesktopLyrics } = usePreferences();
  return (
    <Tooltip label={controller.snapshot.visible ? t("desktopLyrics.hide") : t("desktopLyrics.show")}>
      <ActionIcon
        aria-label={controller.snapshot.visible ? t("desktopLyrics.hide") : t("desktopLyrics.show")}
        color={controller.snapshot.visible ? undefined : "gray"}
        disabled={busy || controller.busy || !hasCurrentTrack || !controller.snapshot.supported}
        onClick={() => {
          if (controller.snapshot.visible) {
            setDesktopLyrics({ enabled: false });
            void controller.run("hide_desktop_lyrics_window");
            return;
          }
          void controller.run("show_desktop_lyrics_window", { fontSize: preferences.fontSize }).then((success) => {
            if (success) setDesktopLyrics({ enabled: true });
          });
        }}
        size="xl"
        variant={controller.snapshot.visible ? "light" : "subtle"}
      >
        <Captions size={19} />
      </ActionIcon>
    </Tooltip>
  );
}

function AudioQualityBadge({ quality }: { quality: TrackDetails["quality"] }) {
  if (!quality) return <span aria-hidden="true" className="audio-quality-slot" />;
  const label = quality === "hi_res" ? "Hi-Res" : quality === "sq" ? "SQ" : "HQ";
  return (
    <svg
      aria-label={label}
      className="audio-quality-badge"
      data-quality={quality}
      role="img"
      viewBox="0 0 46 26"
    >
      <rect height="24.5" rx="3" width="44.5" x="0.75" y="0.75" />
      <text x="23" y="13">{quality === "hi_res" ? "HI-RES" : label}</text>
    </svg>
  );
}

function PlaybackModeButton({ busy, mode, onRun }: {
  busy: boolean;
  mode: PlaybackSnapshot["playbackMode"];
  onRun: ReturnType<typeof usePlaybackController>["run"];
}) {
  const { t } = useTranslation();
  const modes: { icon: ReactNode; value: PlaybackSnapshot["playbackMode"] }[] = [
    { icon: <ArrowRight size={19} />, value: "sequential" },
    { icon: <Repeat size={19} />, value: "repeat_all" },
    { icon: <Repeat1 size={19} />, value: "repeat_one" },
    { icon: <Shuffle size={19} />, value: "shuffle" },
  ];
  const active = modes.find((item) => item.value === mode) ?? modes[0];
  return (
    <Menu position="top-end" shadow="md" width={156}>
      <Menu.Target>
        <ActionIcon
          aria-label={t(`playback.modes.${mode}`)}
          color={mode === "sequential" ? "gray" : undefined}
          disabled={busy}
          size="xl"
          title={t(`playback.modes.${mode}`)}
          variant={mode === "sequential" ? "subtle" : "light"}
        >
          {active.icon}
        </ActionIcon>
      </Menu.Target>
      <Menu.Dropdown>
        {modes.map((item) => (
          <Menu.Item
            key={item.value}
            leftSection={item.icon}
            onClick={() => void onRun("set_playback_mode", { mode: item.value })}
            rightSection={item.value === mode ? <Check size={14} /> : null}
          >
            {t(`playback.modes.${item.value}`)}
          </Menu.Item>
        ))}
      </Menu.Dropdown>
    </Menu>
  );
}

function FullPlayerView({ details, error, loading, lyrics, onClose, onSeek, output, seekable, title }: {
  details: ReturnType<typeof useTrackDetails>["details"];
  error: string | null;
  loading: boolean;
  lyrics: LyricsSnapshot;
  onClose: () => void;
  onSeek: (positionMs: number) => Promise<boolean>;
  output: PlaybackSnapshot["output"];
  seekable: boolean;
  title: string;
}) {
  const { t } = useTranslation();
  const [tab, setTab] = useState<"lyrics" | "details">("lyrics");
  const activeLyricRef = useRef<HTMLButtonElement | null>(null);
  const lines = lyrics.document?.lines ?? [];
  useEffect(() => {
    if (tab !== "lyrics" || lyrics.activeLineIndex === null) return;
    const frame = requestAnimationFrame(() => {
      activeLyricRef.current?.scrollIntoView({
        behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth",
        block: "center",
      });
    });
    return () => cancelAnimationFrame(frame);
  }, [lyrics.activeLineIndex, tab]);
  return (
    <section className="full-player" aria-label={t("playback.nowPlaying")}>
      <ActionIcon aria-label={t("common.close")} className="full-player-close" data-testid="player-close" onClick={onClose} variant="subtle">
        <X size={19} />
      </ActionIcon>
      <div className="full-player-summary">
        <div className="full-player-artwork">
          {details?.artworkDataUrl
            ? <img alt="" src={details.artworkDataUrl} />
            : <Disc3 aria-hidden="true" size={72} />}
        </div>
        <div className="full-player-copy">
          <Title order={2}>{details?.title || title}</Title>
          <Text c="dimmed">{details?.artist || t("metadata.unknownArtist")}</Text>
          {loading && <Loader size="xs" />}
          {(error || details?.metadataWarning) && <Text c="yellow" size="xs">{t("metadata.partial")}</Text>}
        </div>
      </div>
      <div className="full-player-compact-copy">
        <Title order={3} lineClamp={1}>{details?.title || title}</Title>
        <Text c="dimmed" lineClamp={1} size="sm">{details?.artist || t("metadata.unknownArtist")}</Text>
      </div>
      <div className="full-player-right">
        <div className="full-player-view-switch">
          <SegmentedControl
            aria-label={t("playback.nowPlayingView")}
            data={[
              { label: t("lyrics.region"), value: "lyrics" },
              { label: t("metadata.title"), value: "details" },
            ]}
            fullWidth
            onChange={(value) => setTab(value as "lyrics" | "details")}
            size="xs"
            value={tab}
          />
        </div>
        {tab === "lyrics" ? (
          <ScrollArea className="full-player-panel full-player-lyrics" type="never" aria-label={t("lyrics.region")}>
            <div className="full-player-lyrics-content">
            {lines.length === 0 ? (
              <EmptyView icon={<Captions />} label={t("playback.noLyrics")} />
            ) : (
              lines.map((line, index) => (
                <UnstyledButton
                  aria-label={`${line.text} · ${formatDuration(line.startMs)}`}
                  className="full-player-lyric-line"
                  data-active={index === lyrics.activeLineIndex || undefined}
                  disabled={!seekable}
                  key={`${line.startMs}-${index}`}
                  onClick={() => void onSeek(line.startMs)}
                  ref={index === lyrics.activeLineIndex ? activeLyricRef : undefined}
                >{line.text}</UnstyledButton>
              ))
            )}
            </div>
          </ScrollArea>
        ) : (
          <ScrollArea className="full-player-panel full-player-details" type="auto">
            <div className="full-player-details-content">
            <MetadataRow label={t("metadata.artist")} value={details?.artist} />
            <MetadataRow label={t("metadata.album")} value={details?.album} />
            <MetadataRow compact label={t("metadata.inputInfo")} value={formatInputAudioInfo(details, t)} />
            <MetadataRow compact label={t("metadata.outputInfo")} value={formatOutputAudioInfo(output, t)} />
            <MetadataRow label={t("metadata.path")} value={details?.path} />
            </div>
          </ScrollArea>
        )}
      </div>
    </section>
  );
}

function MetadataRow({ compact, label, value }: { compact?: boolean; label: string; value: string | null | undefined }) {
  const { t } = useTranslation();
  return (
    <div className="metadata-row" data-compact={compact || undefined}>
      <Text c="dimmed" size="sm">{label}</Text>
      <Text lineClamp={compact ? 1 : 2} size="sm" title={value ?? undefined}>{value || t("metadata.unknown")}</Text>
    </div>
  );
}

function formatInputAudioInfo(
  details: ReturnType<typeof useTrackDetails>["details"],
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (!details) return null;
  return [
    details.codec.toUpperCase(),
    details.sampleRate ? formatSampleRate(details.sampleRate) : null,
    details.bitDepth ? `${details.bitDepth}-bit` : null,
    details.channels ? t("metadata.channels", { count: details.channels }) : null,
    details.audioBitrate ? `${details.audioBitrate.toLocaleString()} kbps` : null,
  ].filter(Boolean).join(" · ");
}

function formatOutputAudioInfo(
  output: PlaybackSnapshot["output"],
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (output.status !== "ready") return null;
  return [
    output.activeSampleRate ? formatSampleRate(output.activeSampleRate) : null,
    output.activeSampleFormat?.toUpperCase(),
    output.activeChannelCount
      ? t("metadata.channels", { count: output.activeChannelCount })
      : null,
    output.activeDeviceName,
  ].filter(Boolean).join(" · ");
}

function formatSampleRate(sampleRate: number) {
  const kilohertz = sampleRate / 1_000;
  return `${Number.isInteger(kilohertz) ? kilohertz : kilohertz.toFixed(1)} kHz`;
}

function EmptyView({ icon, label }: { icon: ReactNode; label: string }) {
  return (
    <div className="empty-state">
      <ThemeIcon color="gray" size={48} variant="light">{icon}</ThemeIcon>
      <Text c="dimmed" size="sm">{label}</Text>
    </div>
  );
}

function ErrorBanner({ failure }: { failure: PlaybackFailure }) {
  const { t } = useTranslation();
  return <div className="error-banner" role="alert">{localizeFailure(failure, t)}</div>;
}

function localizeFailure(
  failure: PlaybackFailure | null,
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (!failure) return "";
  const localized = t(`errors.${failure.code}`, { defaultValue: failure.message });
  return failure.code === "task_failed" && failure.message && failure.message !== localized
    ? `${localized}: ${failure.message}`
    : localized;
}

function dropTargetAtPosition(clientX: number, clientY: number): DropTarget | null {
  const hit = document.elementFromPoint(clientX, clientY);
  const element = hit?.closest<HTMLElement>("[data-drop-kind]");
  const trackList = hit?.closest<HTMLElement>(".track-scroll[data-drop-kind]");
  if (trackList) {
    const position = listInsertionPositionAtY(trackList, clientY);
    if (trackList.dataset.dropKind === "default-track-list") {
      return { kind: "default-track-gap", position };
    }
    const playlistId = Number(trackList.dataset.playlistId);
    if (Number.isFinite(playlistId)) return { kind: "track-gap", playlistId, position };
  }
  if (!element) return null;
  const kind = element.dataset.dropKind;
  const playlistId = Number(element.dataset.playlistId);
  const position = Number(element.dataset.position);
  if (kind === "default") return { kind };
  if (kind === "playlist" && Number.isFinite(playlistId)) return { kind, playlistId };
  if (kind === "playlist-gap" && Number.isFinite(position)) return { kind, position };
  if (kind === "track-gap" && Number.isFinite(playlistId) && Number.isFinite(position)) {
    return { kind, playlistId, position };
  }
  return null;
}

function dropTargetsEqual(left: DropTarget | null, right: DropTarget | null) {
  if (left?.kind !== right?.kind) return false;
  if (!left || !right) return left === right;
  if (left.kind === "default" && right.kind === "default") return true;
  if (left.kind === "default-track-gap" && right.kind === "default-track-gap") {
    return left.position === right.position;
  }
  if (left.kind === "playlist" && right.kind === "playlist") {
    return left.playlistId === right.playlistId;
  }
  if (left.kind === "playlist-gap" && right.kind === "playlist-gap") {
    return left.position === right.position;
  }
  return left.kind === "track-gap" && right.kind === "track-gap"
    && left.playlistId === right.playlistId
    && left.position === right.position;
}

const MemoSidebar = memo(Sidebar);
const MemoDefaultPlaylistView = memo(DefaultPlaylistView);
const MemoUserPlaylistView = memo(UserPlaylistView);
const MemoRecentView = memo(RecentView);
const MemoToolsView = memo(ToolsView);
const MemoSettingsView = memo(SettingsView);
const MemoFullPlayerView = memo(FullPlayerView);
