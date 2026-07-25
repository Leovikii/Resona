import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ActionIcon, Paper, Portal, Text } from "@mantine/core";
import { Play, Trash2, X } from "lucide-react";
import { useTranslation } from "react-i18next";

import { usePointerReorder } from "./usePointerReorder";
import { OverflowMarquee } from "./OverflowMarquee";

export interface PlaylistTrackListItem {
  id: number;
  displayName: string;
  path: string;
}

interface PlaylistTrackListProps {
  busy: boolean;
  currentPath: string | null;
  externalInsertionPosition: number | null;
  items: PlaylistTrackListItem[];
  onAddFiles: () => void;
  onAddFolders: () => void;
  onClear: () => void;
  onMove: (itemId: number, toPosition: number) => void;
  onPlay: (itemId: number) => void;
  onRemove: (itemIds: number[]) => void;
  scrollViewportRef: { current: HTMLDivElement | null };
}

export function PlaylistTrackList({
  busy,
  currentPath,
  externalInsertionPosition,
  items,
  onAddFiles,
  onAddFolders,
  onClear,
  onMove,
  onPlay,
  onRemove,
  scrollViewportRef,
}: PlaylistTrackListProps) {
  const { t } = useTranslation();
  const [selected, setSelected] = useState<Set<number>>(() => new Set());
  const [anchor, setAnchor] = useState<number | null>(null);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; trackId: number | null } | null>(null);
  const contextMenuRef = useRef<HTMLDivElement | null>(null);
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
    const dismiss = (event: PointerEvent) => {
      if (event.target instanceof Node && contextMenuRef.current?.contains(event.target)) return;
      setContextMenu(null);
    };
    window.addEventListener("pointerdown", dismiss);
    const dismissOnBlur = () => setContextMenu(null);
    window.addEventListener("blur", dismissOnBlur);
    return () => {
      window.removeEventListener("pointerdown", dismiss);
      window.removeEventListener("blur", dismissOnBlur);
    };
  }, []);

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
    event.preventDefault();
    if (!selected.has(itemId)) {
      setSelected(new Set([itemId]));
      setAnchor(itemId);
    }
    setContextMenu({ x: event.clientX, y: event.clientY, trackId: itemId });
  }, [selected]);

  const openBlankMenu = useCallback((event: React.MouseEvent) => {
    const target = event.target as Element;
    if (target.closest("[data-track-id], .playlist-selection-actions")) return;
    event.preventDefault();
    setContextMenu({ x: event.clientX, y: event.clientY, trackId: null });
  }, []);

  const removeSelected = useCallback(() => {
    if (selectedIds.length === 0) return;
    onRemove(selectedIds);
    clearSelection();
  }, [clearSelection, onRemove, selectedIds]);

  return (
    <div
      className="playlist-track-area"
      onClick={(event) => {
        if ((event.target as Element).closest("[data-track-id], button, input, [role='button']")) return;
        clearSelection();
      }}
      onContextMenu={openBlankMenu}
    >
      <div className="playlist-selection-actions">
        <Text c="dimmed" size="sm">{t("common.tracks", { count: items.length })}</Text>
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
        {items.map((item, index) => {
          const current = currentPath === item.path;
          return <div className="saved-track-slot" key={item.id}>
          <div
            className="saved-track-gap"
            data-active={reorder.insertionPosition === index || externalInsertionPosition === index || undefined}
          ><span /></div>
          <Paper
            className="saved-track-row"
            data-current={current || undefined}
            data-dragging={reorder.draggedId === item.id || undefined}
            data-reorder-position={index}
            data-selected={selected.has(item.id) || undefined}
            data-track-id={item.id}
            data-track-position={index}
            aria-selected={selected.has(item.id)}
            onClick={(event) => {
              if (!reorder.consumeClick()) selectTrack(event, item.id);
            }}
            onContextMenu={(event) => openTrackMenu(event, item.id)}
            onDoubleClick={() => onPlay(item.id)}
            onKeyDown={(event) => {
              if (event.key === "Enter") onPlay(item.id);
              if (event.key === " " && !event.repeat) {
                event.preventDefault();
                selectTrack(event as unknown as React.MouseEvent, item.id);
              }
            }}
            onPointerCancel={reorder.onPointerCancel}
            onPointerDown={(event) => {
              if (event.ctrlKey || event.metaKey || event.shiftKey) return;
              reorder.onPointerDown(event, item.id);
            }}
            onPointerMove={reorder.onPointerMove}
            onPointerUp={reorder.onPointerUp}
            role="option"
            tabIndex={0}
          >
            <span className="track-index">
              {current ? (
                <span aria-label={t("playback.playing")} className="playlist-playing-indicator track-playing-indicator">
                  <i /><i /><i />
                </span>
              ) : index + 1}
            </span>
            <div className="track-copy">
              <OverflowMarquee className="track-title" observe={false} text={item.displayName} />
              <OverflowMarquee className="track-path" observe={false} text={item.path} />
            </div>
          </Paper>
        </div>;
        })}
        <div
          className="saved-track-gap"
          data-active={reorder.insertionPosition === items.length || externalInsertionPosition === items.length || undefined}
        ><span /></div>
      </div>
      {contextMenu && <Portal>
        <Paper className="app-context-menu" ref={contextMenuRef} shadow="md" style={{ left: contextMenu.x, top: contextMenu.y }} withBorder>
          {contextMenu.trackId !== null && (
            <button onClick={() => { onPlay(contextMenu.trackId!); setContextMenu(null); }} type="button"><Play size={14} />{t("playback.play")}</button>
          )}
          <button disabled={selectedIds.length === 0} onClick={() => { removeSelected(); setContextMenu(null); }} type="button"><Trash2 size={14} />{t("common.remove")}</button>
          <button onClick={() => { setSelected(new Set(items.map((item) => item.id))); setAnchor(items[0]?.id ?? null); setContextMenu(null); }} type="button">{t("library.selectAll")}</button>
          <button onClick={() => { clearSelection(); setContextMenu(null); }} type="button"><X size={14} />{t("library.clearSelection")}</button>
          {contextMenu.trackId === null && <>
            <button onClick={() => { onAddFiles(); setContextMenu(null); }} type="button">{t("library.addFiles")}</button>
            <button onClick={() => { onAddFolders(); setContextMenu(null); }} type="button">{t("library.addFolder")}</button>
            <button onClick={() => { onClear(); setContextMenu(null); }} type="button">{t("common.clear")}</button>
          </>}
        </Paper>
      </Portal>}
    </div>
  );
}
