import { Fragment, memo, useEffect, useRef, useState } from "react";
import type { ReactNode, WheelEvent } from "react";
import { ActionIcon, Paper, Portal, ScrollArea, Tabs, Tooltip } from "@mantine/core";
import { ArrowDown, ArrowUp, Plus, Settings, Wrench } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { ActivePlaylistSnapshot, PlaylistSummary } from "../model/library";
import { BrandWordmark } from "./BrandWordmark";
import { usePointerReorder } from "./usePointerReorder";

export type CompactNavigationSelection =
  | { kind: "default" }
  | { kind: "user"; playlistId: number }
  | { kind: "tools" }
  | { kind: "settings" };

interface CompactTopNavigationProps {
  activePlaylist: ActivePlaylistSnapshot | null;
  defaultDropActive: boolean;
  dropPlaylistPosition: number | null;
  dropPlaylistId: number | null;
  externalDragActive: boolean;
  moveDisabled: boolean;
  onClearDefault: () => void;
  onCreatePlaylist: () => void;
  onDeletePlaylist: (playlistId: number) => void;
  onMovePlaylist: (playlistId: number, toPosition: number) => Promise<boolean>;
  onRenamePlaylist: (playlistId: number) => void;
  onSelect: (selection: CompactNavigationSelection) => void;
  playlists: PlaylistSummary[];
  selection: CompactNavigationSelection;
}

type TabMenu =
  | { x: number; y: number; kind: "default" }
  | { x: number; y: number; kind: "user"; playlistId: number };

export const CompactTopNavigation = memo(function CompactTopNavigation({
  activePlaylist,
  defaultDropActive,
  dropPlaylistPosition,
  dropPlaylistId,
  externalDragActive,
  moveDisabled,
  onClearDefault,
  onCreatePlaylist,
  onDeletePlaylist,
  onMovePlaylist,
  onRenamePlaylist,
  onSelect,
  playlists,
  selection,
}: CompactTopNavigationProps) {
  const { t } = useTranslation();
  const [contextMenu, setContextMenu] = useState<TabMenu | null>(null);
  const contextMenuRef = useRef<HTMLDivElement | null>(null);
  const playlistViewportRef = useRef<HTMLDivElement | null>(null);
  const reorder = usePointerReorder({
    axis: "horizontal",
    disabled: externalDragActive || moveDisabled,
    items: playlists,
    onMove: (playlistId, toPosition) => void onMovePlaylist(playlistId, toPosition),
    scrollViewportRef: playlistViewportRef,
  });
  const activeTab = selection.kind === "default"
    ? "default"
    : selection.kind === "user"
      ? `user:${selection.playlistId}`
      : null;
  const handlePlaylistWheel = (event: WheelEvent<HTMLDivElement>) => {
    const viewport = playlistViewportRef.current;
    if (!viewport || viewport.scrollWidth <= viewport.clientWidth) return;
    const delta = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY;
    if (delta === 0) return;
    event.preventDefault();
    viewport.scrollLeft += delta;
  };

  useEffect(() => {
    const dismiss = (event: Event) => {
      if (event.target instanceof Node && contextMenuRef.current?.contains(event.target)) return;
      setContextMenu(null);
    };
    window.addEventListener("pointerdown", dismiss);
    window.addEventListener("blur", dismiss);
    return () => {
      window.removeEventListener("pointerdown", dismiss);
      window.removeEventListener("blur", dismiss);
    };
  }, []);

  return (
    <header className="compact-top-navigation">
      <div className="compact-global-navigation">
        <BrandWordmark className="compact-brand-wordmark" />
        <nav className="compact-global-actions" aria-label={t("app.name")}>
          <CompactNavButton
            active={selection.kind === "tools"}
            icon={<Wrench size={17} />}
            label={t("nav.tools")}
            onClick={() => onSelect({ kind: "tools" })}
          />
          <CompactNavButton
            active={selection.kind === "settings"}
            icon={<Settings size={17} />}
            label={t("nav.settings")}
            onClick={() => onSelect({ kind: "settings" })}
          />
        </nav>
      </div>

      <div className="compact-playlist-navigation">
        <ScrollArea
          className="compact-playlist-scroll"
          data-drag-scroll-axis="horizontal"
          onWheel={handlePlaylistWheel}
          offsetScrollbars="x"
          scrollbars="x"
          scrollbarSize={4}
          scrollHideDelay={700}
          type="scroll"
          viewportRef={playlistViewportRef}
        >
          <Tabs
            className="compact-playlist-tabs"
            onChange={(value) => {
              if (reorder.consumeClick()) return;
              if (value === "default") onSelect({ kind: "default" });
              else if (value?.startsWith("user:")) {
                const playlistId = Number(value.slice(5));
                if (Number.isFinite(playlistId)) onSelect({ kind: "user", playlistId });
              }
            }}
            value={activeTab}
            variant="pills"
          >
            <Tabs.List
              data-external-drag={externalDragActive || undefined}
              data-internal-drag={reorder.draggedId !== null || undefined}
              ref={reorder.listRef}
            >
              <Tabs.Tab
                data-drop-active={defaultDropActive || undefined}
                data-drop-kind="default"
                data-playing={activePlaylist?.kind === "default" || undefined}
                onContextMenu={(event) => {
                  event.preventDefault();
                  setContextMenu({ x: event.clientX, y: event.clientY, kind: "default" });
                }}
                value="default"
              >
                <PlaylistTabLabel label={t("library.default")} order={1} playing={activePlaylist?.kind === "default"} />
              </Tabs.Tab>
              <CompactPlaylistGap active={dropPlaylistPosition === 0 || reorder.insertionPosition === 0} position={0} />
              {playlists.map((playlist, index) => {
                const playing = activePlaylist?.kind === "user" && activePlaylist.playlistId === playlist.id;
                return (
                  <Fragment key={playlist.id}>
                    <Tabs.Tab
                      data-drop-active={dropPlaylistId === playlist.id || undefined}
                      data-dragging={reorder.draggedId === playlist.id || undefined}
                      data-drop-kind="playlist"
                      data-playlist-id={playlist.id}
                      data-playing={playing || undefined}
                      data-reorder-position={index}
                      onContextMenu={(event) => {
                        event.preventDefault();
                        setContextMenu({
                          x: event.clientX,
                          y: event.clientY,
                          kind: "user",
                          playlistId: playlist.id,
                        });
                      }}
                      onPointerCancel={reorder.onPointerCancel}
                      onPointerDown={(event) => reorder.onPointerDown(event, playlist.id)}
                      onPointerMove={reorder.onPointerMove}
                      onPointerUp={reorder.onPointerUp}
                      title={playlist.name}
                      value={`user:${playlist.id}`}
                    >
                      <PlaylistTabLabel label={playlist.name} order={index + 2} playing={playing} />
                    </Tabs.Tab>
                    <CompactPlaylistGap active={dropPlaylistPosition === index + 1 || reorder.insertionPosition === index + 1} position={index + 1} />
                  </Fragment>
                );
              })}
            </Tabs.List>
          </Tabs>
        </ScrollArea>
        <Tooltip label={t("library.create")}>
          <ActionIcon aria-label={t("library.create")} onClick={onCreatePlaylist} size="sm" variant="subtle">
            <Plus size={16} />
          </ActionIcon>
        </Tooltip>
      </div>

      {contextMenu && <Portal>
        <Paper className="app-context-menu" ref={contextMenuRef} shadow="md" style={{ left: contextMenu.x, top: contextMenu.y }} withBorder>
          {contextMenu.kind === "default" ? (
            <button onClick={() => { onClearDefault(); setContextMenu(null); }} type="button">{t("common.clear")}</button>
          ) : <>
            <button
              disabled={playlists.findIndex((playlist) => playlist.id === contextMenu.playlistId) <= 0}
              onClick={() => {
                const position = playlists.findIndex((playlist) => playlist.id === contextMenu.playlistId);
                if (position > 0) void onMovePlaylist(contextMenu.playlistId, position - 1);
                setContextMenu(null);
              }}
              type="button"
            ><ArrowUp size={14} />{t("library.moveUp")}</button>
            <button
              disabled={playlists.findIndex((playlist) => playlist.id === contextMenu.playlistId) >= playlists.length - 1}
              onClick={() => {
                const position = playlists.findIndex((playlist) => playlist.id === contextMenu.playlistId);
                if (position >= 0 && position < playlists.length - 1) void onMovePlaylist(contextMenu.playlistId, position + 1);
                setContextMenu(null);
              }}
              type="button"
            ><ArrowDown size={14} />{t("library.moveDown")}</button>
            <button onClick={() => { onRenamePlaylist(contextMenu.playlistId); setContextMenu(null); }} type="button">{t("library.rename")}</button>
            <button onClick={() => { onDeletePlaylist(contextMenu.playlistId); setContextMenu(null); }} type="button">{t("library.delete")}</button>
          </>}
        </Paper>
      </Portal>}
    </header>
  );
});

function CompactPlaylistGap({ active, position }: { active: boolean; position: number }) {
  return (
    <span
      aria-hidden="true"
      className="compact-playlist-gap"
      data-active={active || undefined}
      data-drop-kind="playlist-gap"
      data-position={position}
    >
      <i />
    </span>
  );
}

function CompactNavButton({ active, icon, label, onClick }: {
  active: boolean;
  icon: ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <Tooltip label={label}>
      <ActionIcon
        aria-current={active ? "page" : undefined}
        aria-label={label}
        className="compact-global-action"
        data-active={active || undefined}
        onClick={onClick}
        size="sm"
        variant="subtle"
      >
        {icon}
      </ActionIcon>
    </Tooltip>
  );
}

function PlaylistTabLabel({ label, order, playing }: { label: string; order: number; playing: boolean }) {
  const { t } = useTranslation();
  return (
    <span className="compact-playlist-tab-label">
      <span className="compact-playlist-order-slot">{playing
        ? <span aria-label={t("playback.playing")} className="playlist-playing-indicator"><i /><i /><i /></span>
        : <span className="playlist-order">{order}</span>}
      </span>
      <span>{label}</span>
    </span>
  );
}
