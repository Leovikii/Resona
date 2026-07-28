import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { ActionIcon, Paper, Text, UnstyledButton } from "@mantine/core";
import {
  ChevronDown,
  ChevronRight,
  Eraser,
  FileAudio,
  Info,
  Folder,
  FolderOpen,
  FolderPlus,
  ListChecks,
  Play,
  Trash2,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { usePointerReorder } from "./usePointerReorder";
import { OverflowMarquee } from "./OverflowMarquee";
import { AppContextMenu, type AppContextMenuItem } from "./AppContextMenu";
import {
  buildPlaylistTree,
  playlistFolderPaths,
  playlistDurationSummary,
  playlistRootPaths,
  playlistTrackFolderPaths,
  playlistTrackDuration,
  playlistTrackTitle,
  resolvePlaylistFolderState,
  type PlaylistFolderState,
  type PlaylistTreeNode,
} from "./playlistTree";
import { formatDuration } from "../utils/format";

export interface PlaylistTrackListItem {
  id: number;
  displayName: string;
  path: string;
  folderRoot: string | null;
  cue?: import("../model/library").CueTrackSource | null;
}

export interface PlaylistTrackLocateRequest {
  id: number;
  path: string;
  cue?: import("../model/library").CueTrackSource | null;
}

interface PlaylistTrackListProps {
  busy: boolean;
  currentPath: string | null;
  currentCue?: import("../model/library").CueTrackSource | null;
  externalInsertionPosition: number | null;
  folderState: PlaylistFolderState | undefined;
  items: PlaylistTrackListItem[];
  locateRequest: PlaylistTrackLocateRequest | null;
  onAddFiles: () => void;
  onAddFolders: () => void;
  onClear: () => void;
  onFolderStateChange: (state: PlaylistFolderState) => void;
  onLocateHandled: (requestId: number) => void;
  onMove: (itemId: number, toPosition: number) => void;
  onPlay: (itemId: number) => void;
  onRemove: (itemIds: number[]) => void;
  onShowInfo: (path: string) => void;
  scrollViewportRef: { current: HTMLDivElement | null };
  summaries: Map<string, import("../model/metadata").TrackSummary>;
  treeKey: string;
}

export function PlaylistTrackList({
  busy,
  currentPath,
  currentCue,
  externalInsertionPosition,
  folderState,
  items,
  locateRequest,
  onAddFiles,
  onAddFolders,
  onClear,
  onFolderStateChange,
  onLocateHandled,
  onMove,
  onPlay,
  onRemove,
  onShowInfo,
  scrollViewportRef,
  summaries,
  treeKey,
}: PlaylistTrackListProps) {
  const { t } = useTranslation();
  const [selected, setSelected] = useState<Set<number>>(() => new Set());
  const [anchor, setAnchor] = useState<number | null>(null);
  const [contextTrackId, setContextTrackId] = useState<number | null>(null);
  const handledLocateRequestRef = useRef<number | null>(null);
  const tree = useMemo(() => buildPlaylistTree(items), [items]);
  const rootPaths = useMemo(() => playlistRootPaths(items), [items]);
  const folderPaths = useMemo(() => playlistFolderPaths(tree), [tree]);
  const rootPathKey = rootPaths.join("\0");
  const folderPathKey = folderPaths.join("\0");
  const resolvedFolderState = useMemo(
    () => resolvePlaylistFolderState(folderState, rootPaths, folderPaths),
    [folderPathKey, folderPaths, folderState, rootPathKey, rootPaths],
  );
  const expandedFolders = useMemo(
    () => new Set(resolvedFolderState.expandedPaths),
    [resolvedFolderState.expandedPaths],
  );
  const selectDraggedTrack = useCallback((itemId: number) => {
    setSelected(new Set([itemId]));
    setAnchor(itemId);
  }, []);
  const reorder = usePointerReorder({
    disabled: busy,
    items,
    onDragStart: selectDraggedTrack,
    onMove,
    scrollViewportRef,
  });

  useEffect(() => {
    setSelected((current) => {
      const available = new Set(items.map((item) => item.id));
      const next = new Set([...current].filter((id) => available.has(id)));
      return next.size === current.size ? current : next;
    });
    setAnchor((current) => current !== null && items.some((item) => item.id === current) ? current : null);
  }, [items]);

  useEffect(() => {
    if (!folderStatesEqual(folderState, resolvedFolderState)) {
      onFolderStateChange(resolvedFolderState);
    }
  }, [folderState, onFolderStateChange, resolvedFolderState, treeKey]);

  useEffect(() => {
    if (!locateRequest) return;
    const item = items.find((candidate) => sameLogicalTrack(candidate, locateRequest));
    if (!item) return;
    const expanded = new Set(resolvedFolderState.expandedPaths);
    const ancestors = playlistTrackFolderPaths(item);
    if (ancestors.every((path) => expanded.has(path))) return;
    for (const path of ancestors) expanded.add(path);
    onFolderStateChange({
      expandedPaths: folderPaths.filter((path) => expanded.has(path)),
      seenRootPaths: resolvedFolderState.seenRootPaths,
    });
  }, [folderPaths, items, locateRequest, onFolderStateChange, resolvedFolderState]);

  useLayoutEffect(() => {
    if (!locateRequest || handledLocateRequestRef.current === locateRequest.id) return;
    const item = items.find((candidate) => sameLogicalTrack(candidate, locateRequest));
    const viewport = scrollViewportRef.current;
    const row = item
      ? reorder.listRef.current?.querySelector<HTMLElement>(`[data-track-id="${item.id}"]`)
      : null;
    if (!viewport || !row) return;
    const viewportBounds = viewport.getBoundingClientRect();
    const rowBounds = row.getBoundingClientRect();
    const top = viewport.scrollTop
      + rowBounds.top
      - viewportBounds.top
      - (viewportBounds.height - rowBounds.height) / 2;
    viewport.scrollTo({ behavior: "smooth", top: Math.max(0, top) });
    row.focus({ preventScroll: true });
    handledLocateRequestRef.current = locateRequest.id;
    onLocateHandled(locateRequest.id);
  }, [expandedFolders, items, locateRequest, onLocateHandled, reorder.listRef, scrollViewportRef]);

  const selectedIds = useMemo(() => [...selected], [selected]);
  const clearSelection = useCallback(() => {
    setSelected(new Set());
    setAnchor(null);
  }, []);
  const selectTrack = useCallback((event: React.MouseEvent, itemId: number) => {
    const itemIndex = items.findIndex((item) => item.id === itemId);
    if (itemIndex < 0) return;
    if (event.shiftKey && anchor !== null) {
      const anchorIndex = items.findIndex((item) => item.id === anchor);
      if (anchorIndex >= 0) {
        const [start, end] = [anchorIndex, itemIndex].sort((left, right) => left - right);
        setSelected(new Set(items.slice(start, end + 1).map((item) => item.id)));
        return;
      }
    }
    if (event.ctrlKey || event.metaKey) {
      setSelected((current) => {
        const next = new Set(current);
        if (next.has(itemId)) next.delete(itemId);
        else next.add(itemId);
        return next;
      });
      setAnchor(itemId);
      return;
    }
    setSelected(new Set([itemId]));
    setAnchor(itemId);
  }, [anchor, items]);

  const openTrackMenu = useCallback((event: React.MouseEvent, itemId: number) => {
    if (!selected.has(itemId)) {
      setSelected(new Set([itemId]));
      setAnchor(itemId);
    }
    setContextTrackId(itemId);
  }, [selected]);

  const openBlankMenu = useCallback((event: React.MouseEvent) => {
    const target = event.target as Element;
    if (target.closest("[data-track-id]")) return;
    setContextTrackId(null);
  }, []);

  const removeSelected = useCallback(() => {
    if (selectedIds.length === 0) return;
    onRemove(selectedIds);
    clearSelection();
  }, [clearSelection, onRemove, selectedIds]);
  const toggleFolder = useCallback((path: string) => {
    const expanded = new Set(resolvedFolderState.expandedPaths);
    if (expanded.has(path)) expanded.delete(path);
    else expanded.add(path);
    onFolderStateChange({
      expandedPaths: folderPaths.filter((folderPath) => expanded.has(folderPath)),
      seenRootPaths: resolvedFolderState.seenRootPaths,
    });
  }, [folderPaths, onFolderStateChange, resolvedFolderState]);
  const contextMenuItems: AppContextMenuItem[] = [
    ...(contextTrackId !== null ? [{
      icon: Play,
      id: "play",
      label: t("playback.play"),
      onSelect: () => onPlay(contextTrackId),
    }, {
      icon: Info,
      id: "file-info",
      label: t("metadata.fileInfo"),
      onSelect: () => {
        const item = items.find((candidate) => candidate.id === contextTrackId);
        if (item) onShowInfo(item.path);
      },
    }] : []),
    {
      destructive: true,
      disabled: selectedIds.length === 0,
      dividerBefore: contextTrackId !== null,
      icon: Trash2,
      id: "remove",
      label: t("common.remove"),
      onSelect: removeSelected,
    },
    {
      icon: ListChecks,
      id: "select-all",
      label: t("library.selectAll"),
      onSelect: () => {
        setSelected(new Set(items.map((item) => item.id)));
        setAnchor(items[0]?.id ?? null);
      },
    },
    {
      disabled: selectedIds.length === 0,
      icon: X,
      id: "clear-selection",
      label: t("library.clearSelection"),
      onSelect: clearSelection,
    },
    ...(contextTrackId === null ? [
      {
        dividerBefore: true,
        icon: FileAudio,
        id: "add-files",
        label: t("library.addFiles"),
        onSelect: onAddFiles,
      },
      {
        icon: FolderPlus,
        id: "add-folder",
        label: t("library.addFolder"),
        onSelect: onAddFolders,
      },
      {
        destructive: true,
        dividerBefore: true,
        icon: Eraser,
        id: "clear",
        label: t("common.clear"),
        onSelect: onClear,
      },
    ] : []),
  ];

  return (
    <AppContextMenu items={contextMenuItems}>
      <div
        className="playlist-track-area"
        onClick={(event) => {
          if ((event.target as Element).closest("[data-track-id], button, input, [role='button']")) return;
          clearSelection();
        }}
        onContextMenu={openBlankMenu}
      >
      <div className="playlist-selection-actions">
        <Text c="dimmed" size="sm">
          {t("common.tracks", { count: items.length })} · {formatAggregateDuration(playlistDurationSummary(items, summaries))}
        </Text>
        <div className="playlist-selection-buttons">
          <ActionIcon aria-label={t("library.deleteSelected")} color="red" disabled={busy || selectedIds.length === 0} onClick={removeSelected} variant="subtle"><Trash2 size={16} /></ActionIcon>
        </div>
      </div>
      <div
        className="playlist-track-list saved-track-list"
        data-external-drag={externalInsertionPosition !== null || undefined}
        data-internal-drag={reorder.draggedId !== null || undefined}
        ref={reorder.listRef}
      >
        {renderTreeNodes(tree, 0, {
          currentPath,
          currentCue,
          expandedFolders,
          externalInsertionPosition,
          items,
          onPlay,
          openTrackMenu,
          reorder,
          selectTrack,
          selected,
          summaries,
          t,
          toggleFolder,
        })}
        <div
          className="saved-track-gap"
          data-active={reorder.insertionPosition === items.length || externalInsertionPosition === items.length || undefined}
        ><span /></div>
      </div>
      </div>
    </AppContextMenu>
  );
}

function folderStatesEqual(left: PlaylistFolderState | undefined, right: PlaylistFolderState) {
  return Boolean(left)
    && arraysEqual(left!.expandedPaths, right.expandedPaths)
    && arraysEqual(left!.seenRootPaths, right.seenRootPaths);
}

function arraysEqual(left: string[], right: string[]) {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

interface TreeRenderContext {
  currentPath: string | null;
  currentCue?: import("../model/library").CueTrackSource | null;
  expandedFolders: Set<string>;
  externalInsertionPosition: number | null;
  items: PlaylistTrackListItem[];
  onPlay: (itemId: number) => void;
  openTrackMenu: (event: React.MouseEvent, itemId: number) => void;
  reorder: ReturnType<typeof usePointerReorder<PlaylistTrackListItem>>;
  selectTrack: (event: React.MouseEvent, itemId: number) => void;
  selected: Set<number>;
  summaries: Map<string, import("../model/metadata").TrackSummary>;
  t: ReturnType<typeof useTranslation>["t"];
  toggleFolder: (path: string) => void;
}

function renderTreeNodes(
  nodes: PlaylistTreeNode<PlaylistTrackListItem>[],
  level: number,
  context: TreeRenderContext,
): React.ReactNode {
  return nodes.map((node) => {
    if (node.kind === "folder") {
      const open = context.expandedFolders.has(node.path);
      const insertionPosition = context.reorder.insertionPosition ?? context.externalInsertionPosition;
      const insertionEdge = insertionPosition === node.startPosition
        ? "before"
        : insertionPosition === node.endPosition ? "after" : undefined;
      const duration = playlistDurationSummary(
        context.items,
        context.summaries,
        node.startPosition,
        node.endPosition,
      );
      return (
        <div className="playlist-folder-branch" key={node.id}>
          <UnstyledButton
            aria-expanded={open}
            className="playlist-folder-row"
            data-insertion-edge={insertionEdge}
            data-reorder-end-position={node.endPosition}
            data-reorder-position={node.startPosition}
            data-track-end-position={node.endPosition}
            data-track-position={node.startPosition}
            onClick={() => context.toggleFolder(node.path)}
            style={{ "--playlist-tree-level": level } as React.CSSProperties}
            title={node.path}
          >
            <span className="playlist-folder-toggle">
              {open ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
            </span>
            {open ? <FolderOpen size={17} /> : <Folder size={17} />}
            <Text className="playlist-folder-name" lineClamp={1} size="sm">{node.name}</Text>
            <Text c="dimmed" className="playlist-folder-summary" size="xs">
              {node.itemCount} · {formatAggregateDuration(duration)}
            </Text>
          </UnstyledButton>
          {open && (
            <div className="playlist-folder-children">
              {renderTreeNodes(node.children, level + 1, context)}
            </div>
          )}
        </div>
      );
    }

    const { item, position } = node;
    const summary = context.summaries.get(item.path);
    const duration = playlistTrackDuration(item, summary);
    const current = sameLogicalTrack(item, { path: context.currentPath ?? "", cue: context.currentCue });
    return <div className="saved-track-slot" key={item.id}>
          <div
            className="saved-track-gap"
            data-active={context.reorder.insertionPosition === position || context.externalInsertionPosition === position || undefined}
          ><span /></div>
          <Paper
            className="saved-track-row"
            data-current={current || undefined}
            data-dragging={context.reorder.draggedId === item.id || undefined}
            data-reorder-position={position}
            data-selected={context.selected.has(item.id) || undefined}
            data-track-id={item.id}
            data-track-position={position}
            aria-selected={context.selected.has(item.id)}
            onClick={(event) => {
              if (!context.reorder.consumeClick()) context.selectTrack(event, item.id);
            }}
            onContextMenu={(event) => context.openTrackMenu(event, item.id)}
            onDoubleClick={() => context.onPlay(item.id)}
            onKeyDown={(event) => {
              if (event.key === "Enter") context.onPlay(item.id);
              if (event.key === " " && !event.repeat) {
                event.preventDefault();
                context.selectTrack(event as unknown as React.MouseEvent, item.id);
              }
            }}
            onPointerCancel={context.reorder.onPointerCancel}
            onPointerDown={(event) => {
              if (event.ctrlKey || event.metaKey || event.shiftKey) return;
              context.reorder.onPointerDown(event, item.id);
            }}
            onPointerMove={context.reorder.onPointerMove}
            onPointerUp={context.reorder.onPointerUp}
            role="option"
            style={{ "--playlist-tree-level": level } as React.CSSProperties}
            tabIndex={0}
          >
            <span className="track-index">
              {current ? (
                <span aria-label={context.t("playback.playing")} className="playlist-playing-indicator track-playing-indicator">
                  <i /><i /><i />
                </span>
              ) : position + 1}
            </span>
            <div className="track-copy">
              <OverflowMarquee
                className="track-title"
                observe={false}
                text={playlistTrackTitle(item, summary)}
              />
            </div>
            <Text c="dimmed" className="track-duration" size="xs">
              {duration === null ? "—" : formatDuration(duration)}
            </Text>
          </Paper>
        </div>;
  });
}

function formatAggregateDuration(summary: { complete: boolean; durationMs: number }) {
  if (!summary.complete && summary.durationMs === 0) return "—";
  const duration = formatDuration(summary.durationMs);
  return summary.complete ? duration : `${duration}+`;
}

function sameLogicalTrack(
  left: Pick<PlaylistTrackListItem, "path" | "cue">,
  right: { path: string; cue?: import("../model/library").CueTrackSource | null },
) {
  if (left.path !== right.path) return false;
  if (!left.cue && !right.cue) return true;
  return left.cue?.cuePath === right.cue?.cuePath
    && left.cue?.trackNumber === right.cue?.trackNumber
    && left.cue?.startMs === right.cue?.startMs;
}
