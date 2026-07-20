import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { DragEvent, ReactNode } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { listen } from "@tauri-apps/api/event";
import {
  ActionIcon,
  Badge,
  Button,
  ColorInput,
  Divider,
  Drawer,
  Group,
  Loader,
  Modal,
  NumberInput,
  Paper,
  Popover,
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
  Disc3,
  FileAudio,
  History,
  ListMusic,
  Lock,
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
  Square,
  Tags,
  Trash2,
  Unlock,
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
import { isTauriRuntime } from "../shared/bridge/tauri";
import type {
  DefaultPlaylistItem,
  PlaylistItem,
  PlaylistSummary,
  RecentPlayRecord,
} from "../shared/model/library";
import type { PlaybackFailure, PlaybackSnapshot } from "../shared/model/playback";
import type { LyricsSnapshot } from "../shared/model/lyrics";
import { fileNameFromPath, formatDuration } from "../shared/utils/format";
import { accentColors, type AccentColor, usePreferences } from "./preferences";

type Selection =
  | { kind: "recent" }
  | { kind: "default" }
  | { kind: "user"; playlistId: number }
  | { kind: "tools" }
  | { kind: "settings" };

type DropTarget =
  | { kind: "playlist"; playlistId: number }
  | { kind: "playlist-gap"; position: number }
  | { kind: "track-gap"; playlistId: number; position: number };

export default function App() {
  const { t } = useTranslation();
  const playback = usePlaybackController();
  const library = useLibrary();
  const desktopLyrics = useDesktopLyricsWindow();
  const [selection, setSelection] = useState<Selection>({ kind: "recent" });
  const [createPlaylistOpen, setCreatePlaylistOpen] = useState(false);
  const [playlistName, setPlaylistName] = useState("");
  const [dropTarget, setDropTarget] = useState<DropTarget | null>(null);
  const [externalDragActive, setExternalDragActive] = useState(false);
  const [playerExpanded, setPlayerExpanded] = useState(false);
  const internalDragSuppressionRef = useRef(0);
  const dropActionsRef = useRef({ library, t });
  dropActionsRef.current = { library, t };

  const hasCurrentTrack = playback.snapshot.currentItemId !== null
    && playback.selectedPath !== null;
  const currentTitle = useMemo(
    () => fileNameFromPath(playback.selectedPath) || t("playback.noTrack"),
    [playback.selectedPath, t],
  );
  const trackDetails = useTrackDetails(playback.selectedPath);

  useEffect(() => {
    if (!hasCurrentTrack) setPlayerExpanded(false);
  }, [hasCurrentTrack]);

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
    if (!isTauriRuntime()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void getCurrentWebview().onDragDropEvent((event) => {
      if (Date.now() < internalDragSuppressionRef.current) {
        setDropTarget(null);
        setExternalDragActive(false);
        return;
      }
      if (event.payload.type === "leave") {
        setDropTarget(null);
        setExternalDragActive(false);
        return;
      }
      const target = dropTargetAtPosition(event.payload.position.x, event.payload.position.y);
      if (event.payload.type === "enter" || event.payload.type === "over") {
        setExternalDragActive(true);
        setDropTarget(target);
        return;
      }
      setDropTarget(null);
      setExternalDragActive(false);
      if (!target || event.payload.paths.length === 0) return;
      const { library: actions, t: translate } = dropActionsRef.current;
      if (target.kind === "playlist-gap") {
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
        });
      }
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      unlisten?.();
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

  const selectedPlaylist = selection.kind === "user"
    ? library.playlists.find((playlist) => playlist.id === selection.playlistId) ?? null
    : null;
  return (
    <main
      className="app-shell"
      onDragEndCapture={() => {
        internalDragSuppressionRef.current = Date.now() + 300;
      }}
      onDragStartCapture={(event) => {
        if (!(event.target as Element).closest("[data-resona-internal-drag]")) return;
        internalDragSuppressionRef.current = Number.POSITIVE_INFINITY;
        setDropTarget(null);
        setExternalDragActive(false);
      }}
    >
      <MemoSidebar
        dropTarget={dropTarget}
        externalDragActive={externalDragActive}
        onCreate={openCreatePlaylist}
        onSelect={selectNavigation}
        playlists={library.playlists}
        selection={selection}
      />

      <section className="main-region" data-player-expanded={playerExpanded || undefined}>
        {playerExpanded && hasCurrentTrack ? (
          <FullPlayerView
            details={trackDetails.details}
            error={trackDetails.error}
            loading={trackDetails.loading}
            lyrics={playback.lyrics}
            onClose={() => setPlayerExpanded(false)}
            output={playback.snapshot.output}
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
                items={playback.defaultPlaylist.items}
                onOpen={playback.openFile}
                onPlay={playback.openPath}
                sourceDirectory={playback.defaultPlaylist.sourceDirectory}
              />
            )}
            {selection.kind === "user" && (
              <UserPlaylistView
                busy={playback.busy || library.loading}
                currentPath={playback.selectedPath}
                dropTarget={dropTarget}
                externalDragActive={externalDragActive}
                items={library.selectedItems}
                itemsLoading={library.itemsLoading}
                onAdd={() => void library.chooseAndAddItems(selection.playlistId)}
                onDelete={() => void library.deletePlaylist(selection.playlistId).then((deleted) => {
                  if (deleted) setSelection({ kind: "default" });
                })}
                onMove={(itemId, toPosition) => void library.moveItem(
                  selection.playlistId,
                  itemId,
                  toPosition,
                )}
                onPlay={(selectedIndex) => void playback.run("replace_queue_and_play", {
                  paths: library.selectedItems.map((item) => item.path),
                  selectedIndex,
                })}
                onRemove={(itemId) => void library.removeItem(selection.playlistId, itemId)}
                onRename={(name) => library.renamePlaylist(selection.playlistId, name)}
                playlist={selectedPlaylist}
              />
            )}
            {selection.kind === "tools" && <MemoToolsView />}
            {selection.kind === "settings" && (
              <SettingsView
                busy={playback.busy}
                desktopLyrics={desktopLyrics}
                onRefresh={() => void playback.run("refresh_output_devices")}
                onSelectOutput={(deviceId) => void playback.run("select_output_device", { deviceId })}
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
        {library.rejectedCount > 0 && (
          <div className="import-notice">
            {t("import.rejected", { count: library.rejectedCount })}
          </div>
        )}
      </section>

      <PlayerBar
        busy={playback.busy}
        desktopLyrics={desktopLyrics}
        hasCurrentTrack={hasCurrentTrack}
        expanded={playerExpanded}
        onToggleExpanded={() => setPlayerExpanded((value) => !value)}
        onRun={playback.run}
        snapshot={playback.snapshot}
        title={currentTitle}
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
  dropTarget,
  externalDragActive,
  onCreate,
  onSelect,
  playlists,
  selection,
}: {
  dropTarget: DropTarget | null;
  externalDragActive: boolean;
  onCreate: () => void;
  onSelect: (selection: Selection) => void;
  playlists: PlaylistSummary[];
  selection: Selection;
}) {
  const { t } = useTranslation();
  return (
    <aside className="sidebar">
      <div className="brand-lockup">
        <ThemeIcon radius="sm" size={34} variant="light">
          <Music2 size={20} strokeWidth={1.8} />
        </ThemeIcon>
        <div className="brand-copy">
          <Text fw={700}>{t("app.name")}</Text>
          <Text c="dimmed" size="xs">{t("app.version")}</Text>
        </div>
      </div>

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
          onClick={() => onSelect({ kind: "default" })}
        />
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
                playlistId={playlist.id}
              />
              <PlaylistGap
                active={dropTarget?.kind === "playlist-gap" && dropTarget.position === index + 1}
                position={index + 1}
              />
            </div>
          ))}
        </div>
      </nav>

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
      <span className="nav-label">{label}</span>
    </UnstyledButton>
  );
}

function PlaylistNavItem({ active, dropActive, icon, label, onClick, playlistId }: {
  active: boolean;
  dropActive?: boolean;
  icon: ReactNode;
  label: string;
  onClick: () => void;
  playlistId?: number;
}) {
  return (
    <UnstyledButton
      aria-current={active ? "page" : undefined}
      className="playlist-nav-item"
      data-active={active || undefined}
      data-drop-active={dropActive || undefined}
      data-drop-kind={playlistId === undefined ? undefined : "playlist"}
      data-playlist-id={playlistId}
      onClick={onClick}
    >
      <span className="nav-icon">{icon}</span>
      <span className="nav-label">{label}</span>
    </UnstyledButton>
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
  items,
  onOpen,
  onPlay,
  sourceDirectory,
}: {
  busy: boolean;
  currentPath: string | null;
  items: DefaultPlaylistItem[];
  onOpen: () => Promise<void>;
  onPlay: (path: string) => Promise<PlaybackSnapshot | null>;
  sourceDirectory: string | null;
}) {
  const { t } = useTranslation();
  return (
    <div className="page-content list-page">
      <Group className="page-heading" justify="space-between" wrap="nowrap">
        <div className="path-heading">
          <Title order={2}>{t("library.default")}</Title>
          <Text c="dimmed" lineClamp={1} size="sm">
            {sourceDirectory ?? t("library.defaultEmptyHint")}
          </Text>
        </div>
        <Button disabled={busy} leftSection={<FileAudio size={16} />} onClick={() => void onOpen()} size="xs">
          {t("common.open")}
        </Button>
      </Group>
      {items.length === 0 ? (
        <EmptyView icon={<Disc3 />} label={t("library.defaultEmpty")} />
      ) : (
        <ScrollArea className="track-scroll" type="auto">
          <div className="simple-track-list">
            {items.map((item, index) => (
              <UnstyledButton
                className="simple-track-row"
                data-current={currentPath === item.path || undefined}
                key={item.path}
                onClick={() => void onPlay(item.path)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") void onPlay(item.path);
                }}
              >
                <span className="track-index">{index + 1}</span>
                <ThemeIcon size={32} variant="light"><FileAudio size={16} /></ThemeIcon>
                <div className="track-copy">
                  <Text fw={600} lineClamp={1} size="sm">{item.displayName}</Text>
                  <Text c="dimmed" lineClamp={1} size="xs">{item.path}</Text>
                </div>
              </UnstyledButton>
            ))}
          </div>
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
  onAdd,
  onDelete,
  onMove,
  onPlay,
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
  onAdd: () => void;
  onDelete: () => void;
  onMove: (itemId: number, toPosition: number) => void;
  onPlay: (selectedIndex: number) => void;
  onRemove: (itemId: number) => void;
  onRename: (name: string) => Promise<boolean>;
  playlist: PlaylistSummary | null;
}) {
  const { t } = useTranslation();
  const [name, setName] = useState(playlist?.name ?? "");
  const [draggedItemId, setDraggedItemId] = useState<number | null>(null);
  const [internalDropPosition, setInternalDropPosition] = useState<number | null>(null);
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
        <Group gap="xs" wrap="nowrap">
          <Button disabled={busy} leftSection={<Plus size={15} />} onClick={onAdd} size="xs" variant="default">
            {t("common.add")}
          </Button>
          <Tooltip label={t("library.delete")}>
            <ActionIcon aria-label={t("library.delete")} color="red" disabled={busy} onClick={onDelete} variant="subtle">
              <Trash2 size={16} />
            </ActionIcon>
          </Tooltip>
        </Group>
      </div>
      <Text c="dimmed" mb="sm" size="sm">{t("common.tracks", { count: items.length })}</Text>
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
        <ScrollArea className="track-scroll" type="auto">
          <div
            className="saved-track-list"
            data-external-drag={externalDragActive || undefined}
            data-internal-drag={draggedItemId !== null || undefined}
          >
            {items.map((item, index) => (
              <div className="saved-track-slot" key={item.id}>
                <TrackGap
                  active={internalDropPosition === index || (dropTarget?.kind === "track-gap"
                    && dropTarget.playlistId === playlist.id
                    && dropTarget.position === index)}
                  onDragOver={(event) => {
                    if (draggedItemId === null) return;
                    event.preventDefault();
                    setInternalDropPosition(index);
                  }}
                  onDrop={(event) => {
                    event.preventDefault();
                    if (draggedItemId === null) return;
                    const from = items.findIndex((candidate) => candidate.id === draggedItemId);
                    const target = Math.max(0, Math.min(items.length - 1, from < index ? index - 1 : index));
                    setDraggedItemId(null);
                    setInternalDropPosition(null);
                    if (from !== target) onMove(draggedItemId, target);
                  }}
                  playlistId={playlist.id}
                  position={index}
                />
                <Paper
                  className="saved-track-row"
                  data-current={currentPath === item.path || undefined}
                  data-dragging={draggedItemId === item.id || undefined}
                  data-resona-internal-drag
                  draggable
                  onDragEnd={() => {
                    setDraggedItemId(null);
                    setInternalDropPosition(null);
                  }}
                  onDragStart={(event) => {
                    setDraggedItemId(item.id);
                    event.dataTransfer.effectAllowed = "move";
                    event.dataTransfer.setData("application/x-resona-playlist-item", String(item.id));
                  }}
                  onClick={() => onPlay(index)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      onPlay(index);
                    }
                  }}
                  role="button"
                  tabIndex={0}
                  withBorder
                >
                  <span className="track-index">{index + 1}</span>
                  <div className="track-copy">
                    <Text fw={600} lineClamp={1} size="sm">{item.displayName}</Text>
                    <Text c="dimmed" lineClamp={1} size="xs">{item.path}</Text>
                  </div>
                  <ActionIcon aria-label={t("common.remove")} onClick={(event) => { event.stopPropagation(); onRemove(item.id); }} size="sm" variant="subtle">
                    <Trash2 size={14} />
                  </ActionIcon>
                </Paper>
              </div>
            ))}
            <TrackGap
              active={internalDropPosition === items.length || (dropTarget?.kind === "track-gap"
                && dropTarget.playlistId === playlist.id
                && dropTarget.position === items.length)}
              onDragOver={(event) => {
                if (draggedItemId === null) return;
                event.preventDefault();
                setInternalDropPosition(items.length);
              }}
              onDrop={(event) => {
                event.preventDefault();
                if (draggedItemId === null) return;
                setDraggedItemId(null);
                setInternalDropPosition(null);
                onMove(draggedItemId, items.length - 1);
              }}
              playlistId={playlist.id}
              position={items.length}
            />
          </div>
        </ScrollArea>
      )}
    </div>
  );
}

function TrackGap({ active, onDragOver, onDrop, playlistId, position }: {
  active: boolean;
  onDragOver?: (event: DragEvent<HTMLDivElement>) => void;
  onDrop?: (event: DragEvent<HTMLDivElement>) => void;
  playlistId: number;
  position: number;
}) {
  return (
    <div
      className="saved-track-gap"
      data-active={active || undefined}
      data-drop-kind="track-gap"
      data-playlist-id={playlistId}
      data-position={position}
      onDragOver={onDragOver}
      onDrop={onDrop}
    >
      <span />
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
    <div className="page-content">
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
          <div>
            <Text fw={600}>{t("tools.tagEditor")}</Text>
            <Text c="dimmed" size="xs">{t("tools.afterFirstRelease")}</Text>
          </div>
          <Badge color="gray" ml="auto" variant="light">{t("common.later")}</Badge>
        </Paper>
      </div>
    </div>
  );
}

function SettingsView({ busy, desktopLyrics, onRefresh, onSelectOutput, output }: {
  busy: boolean;
  desktopLyrics: ReturnType<typeof useDesktopLyricsWindow>;
  onRefresh: () => void;
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
  useEffect(() => setBackgroundOpacity(lyricsPreferences.backgroundOpacity), [lyricsPreferences.backgroundOpacity]);
  return (
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
      </section>

      <section className="settings-section">
        <Text className="settings-section-title" fw={650}>{t("desktopLyrics.title")}</Text>
        <SettingRow label={t("desktopLyrics.fontSize")}>
          <NumberInput
            aria-label={t("desktopLyrics.fontSize")}
            clampBehavior="strict"
            max={64}
            min={16}
            onChange={(value) => {
              if (typeof value === "number") setDesktopLyrics({ fontSize: value });
            }}
            suffix=" px"
            value={lyricsPreferences.fontSize}
            w={130}
          />
        </SettingRow>
        <SettingRow label={t("desktopLyrics.color")}>
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
  );
}

function SettingRow({ children, label }: { children: ReactNode; label: string }) {
  return (
    <div className="setting-row">
      <Text c="dimmed" size="sm">{label}</Text>
      <div className="setting-control">{children}</div>
    </div>
  );
}

function PlayerBar({ busy, desktopLyrics, expanded, hasCurrentTrack, onRun, onToggleExpanded, snapshot, title }: {
  busy: boolean;
  desktopLyrics: ReturnType<typeof useDesktopLyricsWindow>;
  expanded: boolean;
  hasCurrentTrack: boolean;
  onRun: ReturnType<typeof usePlaybackController>["run"];
  onToggleExpanded: () => void;
  snapshot: PlaybackSnapshot;
  title: string;
}) {
  const { t } = useTranslation();
  const [seeking, setSeeking] = useState(false);
  const [seekValue, setSeekValue] = useState(snapshot.positionMs);
  const [changingVolume, setChangingVolume] = useState(false);
  const [volume, setVolume] = useState(Math.round(snapshot.volume * 100));
  const [volumeOpen, setVolumeOpen] = useState(false);
  const [queueOpen, setQueueOpen] = useState(false);
  const [draggedQueueId, setDraggedQueueId] = useState<number | null>(null);
  const [queueDropIndex, setQueueDropIndex] = useState<number | null>(null);
  useEffect(() => {
    if (!seeking) setSeekValue(snapshot.positionMs);
    if (!changingVolume) setVolume(Math.round(snapshot.volume * 100));
  }, [changingVolume, seeking, snapshot.positionMs, snapshot.volume]);

  const canControl = snapshot.status === "playing" || snapshot.status === "paused";
  const currentId = snapshot.currentItemId ?? snapshot.queue[0]?.id;
  return (
    <footer className="player-bar" data-expanded={expanded || undefined}>
      <Slider
        aria-label={t("playback.progress")}
        className="player-progress"
        disabled={!canControl || !snapshot.seekable || busy}
        label={formatDuration}
        max={Math.max(snapshot.durationMs ?? 0, 1)}
        min={0}
        onChange={(value) => {
          setSeeking(true);
          setSeekValue(value);
        }}
        onChangeEnd={(value) => {
          setSeeking(false);
          void onRun("seek_playback", { positionMs: Math.round(value) });
        }}
        value={Math.min(seekValue, Math.max(snapshot.durationMs ?? 0, 1))}
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
          <Text fw={650} lineClamp={1} size="sm">{title}</Text>
          <Text c="dimmed" size="xs">
            {formatDuration(seeking ? seekValue : snapshot.positionMs)} / {formatDuration(snapshot.durationMs)}
          </Text>
        </div>
      </div>
      <Group className="player-controls" gap="xs" justify="center" wrap="nowrap">
        <Tooltip label={t("playback.previous")}>
          <ActionIcon aria-label={t("playback.previous")} disabled={!canControl || busy} onClick={() => void onRun("previous_playback")} size="lg" variant="default">
            <SkipBack fill="currentColor" size={16} />
          </ActionIcon>
        </Tooltip>
        <Tooltip label={snapshot.status === "playing" ? t("playback.pause") : snapshot.status === "paused" ? t("playback.resume") : t("playback.play")}>
          <ActionIcon
            aria-label={snapshot.status === "playing" ? t("playback.pause") : snapshot.status === "paused" ? t("playback.resume") : t("playback.play")}
            disabled={currentId === undefined || busy}
            onClick={() => {
              if (snapshot.status === "playing") void onRun("pause_playback");
              else if (snapshot.status === "paused") void onRun("resume_playback");
              else if (currentId !== undefined) void onRun("play_queue_item", { id: currentId });
            }}
            size="lg"
            variant="filled"
          >
            {snapshot.status === "playing" ? <Pause fill="currentColor" size={17} /> : <Play fill="currentColor" size={17} />}
          </ActionIcon>
        </Tooltip>
        <Tooltip label={t("playback.next")}>
          <ActionIcon aria-label={t("playback.next")} disabled={!canControl || busy} onClick={() => void onRun("next_playback")} size="lg" variant="default">
            <SkipForward fill="currentColor" size={16} />
          </ActionIcon>
        </Tooltip>
        <Tooltip label={t("playback.stop")}>
          <ActionIcon aria-label={t("playback.stop")} disabled={!canControl || busy} onClick={() => void onRun("stop_playback")} size="lg" variant="subtle">
            <Square fill="currentColor" size={14} />
          </ActionIcon>
        </Tooltip>
      </Group>
      <div className="player-actions">
        <PlaybackModeButton busy={busy} mode={snapshot.playbackMode} onRun={onRun} />
        <div className="player-lyrics-actions">
          <Tooltip label={desktopLyrics.snapshot.visible ? t("desktopLyrics.hide") : t("desktopLyrics.show")}>
            <ActionIcon
              aria-label={desktopLyrics.snapshot.visible ? t("desktopLyrics.hide") : t("desktopLyrics.show")}
              color={desktopLyrics.snapshot.visible ? undefined : "gray"}
              disabled={busy || desktopLyrics.busy || !hasCurrentTrack || !desktopLyrics.snapshot.supported}
              onClick={() => void desktopLyrics.run(
                desktopLyrics.snapshot.visible
                  ? "hide_desktop_lyrics_window"
                  : "show_desktop_lyrics_window",
              )}
              variant={desktopLyrics.snapshot.visible ? "light" : "subtle"}
            >
              <Captions size={17} />
            </ActionIcon>
          </Tooltip>
          <span className="player-lyrics-lock">
            {desktopLyrics.snapshot.visible && (
              <Tooltip label={desktopLyrics.snapshot.locked ? t("desktopLyrics.unlock") : t("desktopLyrics.lock")}>
                <ActionIcon
                  aria-label={desktopLyrics.snapshot.locked ? t("desktopLyrics.unlock") : t("desktopLyrics.lock")}
                  disabled={busy || desktopLyrics.busy || !desktopLyrics.snapshot.supported}
                  onClick={() => void desktopLyrics.run(
                    desktopLyrics.snapshot.locked
                      ? "unlock_desktop_lyrics_window"
                      : "lock_desktop_lyrics_window",
                  )}
                  variant="subtle"
                >
                  {desktopLyrics.snapshot.locked ? <Unlock size={16} /> : <Lock size={16} />}
                </ActionIcon>
              </Tooltip>
            )}
          </span>
        </div>
        <Popover onChange={setVolumeOpen} opened={volumeOpen} position="top" shadow="md" width={190} withArrow>
          <Popover.Target>
            <Tooltip label={t("playback.volume")}>
              <ActionIcon aria-label={t("playback.volume")} disabled={busy} onClick={() => setVolumeOpen((open) => !open)} variant="subtle">
                <Volume2 size={17} />
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
              onChange={(value) => {
                setChangingVolume(true);
                setVolume(value);
              }}
              onChangeEnd={(value) => {
                setChangingVolume(false);
                void onRun("set_playback_volume", { volume: value / 100 });
              }}
              value={volume}
            />
          </Popover.Dropdown>
        </Popover>
        <Tooltip label={t("queue.title")}>
          <ActionIcon aria-label={t("queue.title")} onClick={() => setQueueOpen(true)} variant="subtle">
            <ListMusic size={17} />
          </ActionIcon>
        </Tooltip>
      </div>
      <Drawer
        classNames={{ body: "queue-drawer-body", header: "queue-drawer-header" }}
        onClose={() => setQueueOpen(false)}
        opened={queueOpen}
        position="right"
        size="sm"
        title={t("queue.title")}
      >
        {snapshot.queue.length === 0 ? (
          <EmptyView icon={<ListMusic />} label={t("queue.empty")} />
        ) : (
          <div className="queue-list">
            {snapshot.queue.map((item, index) => (
              <div
                className="queue-row"
                data-current={item.id === snapshot.currentItemId || undefined}
                data-dragging={draggedQueueId === item.id || undefined}
                data-drop-target={queueDropIndex === index || undefined}
                data-resona-internal-drag
                draggable
                key={item.id}
                onDragEnd={() => { setDraggedQueueId(null); setQueueDropIndex(null); }}
                onDragOver={(event) => {
                  if (draggedQueueId === null) return;
                  event.preventDefault();
                  setQueueDropIndex(index);
                }}
                onDragStart={(event) => {
                  setDraggedQueueId(item.id);
                  event.dataTransfer.effectAllowed = "move";
                  event.dataTransfer.setData("application/x-resona-queue-item", String(item.id));
                }}
                onDrop={(event) => {
                  event.preventDefault();
                  if (draggedQueueId === null) return;
                  setDraggedQueueId(null);
                  setQueueDropIndex(null);
                  void onRun("move_queue_item", { id: draggedQueueId, toIndex: index });
                }}
              >
                <UnstyledButton
                  className="queue-row-main"
                  onClick={() => void onRun("play_queue_item", { id: item.id })}
                >
                  <span className="track-index">{index + 1}</span>
                  <div className="track-copy">
                    <Text fw={600} lineClamp={1} size="sm">{item.displayName}</Text>
                    <Text c="dimmed" lineClamp={1} size="xs">{item.path}</Text>
                  </div>
                </UnstyledButton>
                <ActionIcon
                  aria-label={t("common.remove")}
                  disabled={busy}
                  onClick={() => void onRun("remove_queue_item", { id: item.id })}
                  size="sm"
                  variant="subtle"
                >
                  <Trash2 size={14} />
                </ActionIcon>
              </div>
            ))}
          </div>
        )}
      </Drawer>
    </footer>
  );
}

function PlaybackModeButton({ busy, mode, onRun }: {
  busy: boolean;
  mode: PlaybackSnapshot["playbackMode"];
  onRun: ReturnType<typeof usePlaybackController>["run"];
}) {
  const { t } = useTranslation();
  const modes: PlaybackSnapshot["playbackMode"][] = ["sequential", "repeat_all", "repeat_one", "shuffle"];
  const next = modes[(modes.indexOf(mode) + 1) % modes.length];
  const icon = mode === "repeat_all" ? <Repeat size={17} />
    : mode === "repeat_one" ? <Repeat1 size={17} />
      : mode === "shuffle" ? <Shuffle size={17} /> : <ArrowRight size={17} />;
  return (
    <Tooltip label={t(`playback.modes.${mode}`)}>
      <ActionIcon
        aria-label={t(`playback.modes.${mode}`)}
        color={mode === "sequential" ? "gray" : undefined}
        disabled={busy}
        onClick={() => void onRun("set_playback_mode", { mode: next })}
        variant={mode === "sequential" ? "subtle" : "light"}
      >
        {icon}
      </ActionIcon>
    </Tooltip>
  );
}

function FullPlayerView({ details, error, loading, lyrics, onClose, output, title }: {
  details: ReturnType<typeof useTrackDetails>["details"];
  error: string | null;
  loading: boolean;
  lyrics: LyricsSnapshot;
  onClose: () => void;
  output: PlaybackSnapshot["output"];
  title: string;
}) {
  const { t } = useTranslation();
  const [tab, setTab] = useState<"lyrics" | "details">("lyrics");
  const activeLyricRef = useRef<HTMLParagraphElement | null>(null);
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
          <div className="full-player-panel full-player-lyrics" aria-label={t("lyrics.region")}>
            {lines.length === 0 ? (
              <EmptyView icon={<Captions />} label={t("playback.noLyrics")} />
            ) : (
              lines.map((line, index) => (
                <Text
                  className="full-player-lyric-line"
                  data-active={index === lyrics.activeLineIndex || undefined}
                  key={`${line.startMs}-${index}`}
                  ref={index === lyrics.activeLineIndex ? activeLyricRef : undefined}
                >{line.text}</Text>
              ))
            )}
          </div>
        ) : (
          <div className="full-player-panel full-player-details">
            <MetadataRow label={t("metadata.artist")} value={details?.artist} />
            <MetadataRow label={t("metadata.album")} value={details?.album} />
            <MetadataRow compact label={t("metadata.inputInfo")} value={formatInputAudioInfo(details, t)} />
            <MetadataRow compact label={t("metadata.outputInfo")} value={formatOutputAudioInfo(output, t)} />
            <MetadataRow label={t("metadata.path")} value={details?.path} />
          </div>
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

function dropTargetAtPosition(physicalX: number, physicalY: number): DropTarget | null {
  const scale = window.devicePixelRatio || 1;
  const element = document
    .elementFromPoint(physicalX / scale, physicalY / scale)
    ?.closest<HTMLElement>("[data-drop-kind]");
  if (!element) return null;
  const kind = element.dataset.dropKind;
  const playlistId = Number(element.dataset.playlistId);
  const position = Number(element.dataset.position);
  if (kind === "playlist" && Number.isFinite(playlistId)) return { kind, playlistId };
  if (kind === "playlist-gap" && Number.isFinite(position)) return { kind, position };
  if (kind === "track-gap" && Number.isFinite(playlistId) && Number.isFinite(position)) {
    return { kind, playlistId, position };
  }
  return null;
}

const MemoSidebar = memo(Sidebar);
const MemoDefaultPlaylistView = memo(DefaultPlaylistView);
const MemoRecentView = memo(RecentView);
const MemoToolsView = memo(ToolsView);
