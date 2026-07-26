import { Fragment, memo, useRef } from "react";
import type { ReactNode, WheelEvent } from "react";
import { ActionIcon, ScrollArea, Tabs, Tooltip } from "@mantine/core";
import { ArrowDown, ArrowUp, Eraser, ListX, Pencil, Plus, Settings, Trash2, Wrench } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { ActivePlaylistSnapshot, PlaylistSummary } from "../model/library";
import { BrandWordmark } from "./BrandWordmark";
import { AppContextMenu } from "./AppContextMenu";
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
  onDeleteOtherPlaylists: (keepId: number | null) => void;
  onDeletePlaylist: (playlistId: number) => void;
  onMovePlaylist: (playlistId: number, toPosition: number) => Promise<boolean>;
  onRenamePlaylist: (playlistId: number) => void;
  onSelect: (selection: CompactNavigationSelection) => void;
  playlists: PlaylistSummary[];
  selection: CompactNavigationSelection;
}

export const CompactTopNavigation = memo(function CompactTopNavigation({
  activePlaylist,
  defaultDropActive,
  dropPlaylistPosition,
  dropPlaylistId,
  externalDragActive,
  moveDisabled,
  onClearDefault,
  onCreatePlaylist,
  onDeleteOtherPlaylists,
  onDeletePlaylist,
  onMovePlaylist,
  onRenamePlaylist,
  onSelect,
  playlists,
  selection,
}: CompactTopNavigationProps) {
  const { t } = useTranslation();
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
              <AppContextMenu items={[
                {
                  destructive: true,
                  icon: Eraser,
                  id: "clear",
                  label: t("common.clear"),
                  onSelect: onClearDefault,
                },
                {
                  destructive: true,
                  disabled: playlists.length === 0,
                  dividerBefore: true,
                  icon: ListX,
                  id: "delete-others",
                  label: t("library.closeOthers"),
                  onSelect: () => onDeleteOtherPlaylists(null),
                },
              ]}>
                <Tabs.Tab
                  data-drop-active={defaultDropActive || undefined}
                  data-drop-kind="default"
                  data-playing={activePlaylist?.kind === "default" || undefined}
                  value="default"
                >
                  <PlaylistTabLabel label={t("library.default")} order={1} playing={activePlaylist?.kind === "default"} />
                </Tabs.Tab>
              </AppContextMenu>
              <CompactPlaylistGap active={dropPlaylistPosition === 0 || reorder.insertionPosition === 0} position={0} />
              {playlists.map((playlist, index) => {
                const playing = activePlaylist?.kind === "user" && activePlaylist.playlistId === playlist.id;
                return (
                  <Fragment key={playlist.id}>
                    <AppContextMenu items={[
                      {
                        disabled: index <= 0,
                        icon: ArrowUp,
                        id: "move-up",
                        label: t("library.moveUp"),
                        onSelect: () => void onMovePlaylist(playlist.id, index - 1),
                      },
                      {
                        disabled: index >= playlists.length - 1,
                        icon: ArrowDown,
                        id: "move-down",
                        label: t("library.moveDown"),
                        onSelect: () => void onMovePlaylist(playlist.id, index + 1),
                      },
                      {
                        dividerBefore: true,
                        icon: Pencil,
                        id: "rename",
                        label: t("library.rename"),
                        onSelect: () => onRenamePlaylist(playlist.id),
                      },
                      {
                        destructive: true,
                        disabled: playlists.length <= 1,
                        dividerBefore: true,
                        icon: ListX,
                        id: "delete-others",
                        label: t("library.closeOthers"),
                        onSelect: () => onDeleteOtherPlaylists(playlist.id),
                      },
                      {
                        destructive: true,
                        icon: Trash2,
                        id: "delete",
                        label: t("library.delete"),
                        onSelect: () => onDeletePlaylist(playlist.id),
                      },
                    ]}>
                      <Tabs.Tab
                        data-drop-active={dropPlaylistId === playlist.id || undefined}
                        data-dragging={reorder.draggedId === playlist.id || undefined}
                        data-drop-kind="playlist"
                        data-playlist-id={playlist.id}
                        data-playing={playing || undefined}
                        data-reorder-position={index}
                        onPointerCancel={reorder.onPointerCancel}
                        onPointerDown={(event) => reorder.onPointerDown(event, playlist.id)}
                        onPointerMove={reorder.onPointerMove}
                        onPointerUp={reorder.onPointerUp}
                        title={playlist.name}
                        value={`user:${playlist.id}`}
                      >
                        <PlaylistTabLabel label={playlist.name} order={index + 2} playing={playing} />
                      </Tabs.Tab>
                    </AppContextMenu>
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
