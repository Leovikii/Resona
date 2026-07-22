import { memo, useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { ActionIcon, Paper, Portal, ScrollArea, Tabs, Text, Tooltip } from "@mantine/core";
import { History, Plus, Settings, Wrench } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { ActivePlaylistSnapshot, PlaylistSummary } from "../model/library";

export type CompactNavigationSelection =
  | { kind: "recent" }
  | { kind: "default" }
  | { kind: "user"; playlistId: number }
  | { kind: "tools" }
  | { kind: "settings" };

interface CompactTopNavigationProps {
  activePlaylist: ActivePlaylistSnapshot | null;
  onClearDefault: () => void;
  onCreatePlaylist: () => void;
  onDeletePlaylist: (playlistId: number) => void;
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
  onClearDefault,
  onCreatePlaylist,
  onDeletePlaylist,
  onRenamePlaylist,
  onSelect,
  playlists,
  selection,
}: CompactTopNavigationProps) {
  const { t } = useTranslation();
  const [contextMenu, setContextMenu] = useState<TabMenu | null>(null);
  const contextMenuRef = useRef<HTMLDivElement | null>(null);
  const activeTab = selection.kind === "default"
    ? "default"
    : selection.kind === "user"
      ? `user:${selection.playlistId}`
      : null;

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
        <Text className="compact-brand-name" fw={700}>{t("app.name")}</Text>
        <nav className="compact-global-actions" aria-label={t("app.name")}>
          <CompactNavButton
            active={selection.kind === "recent"}
            icon={<History size={17} />}
            label={t("nav.recent")}
            onClick={() => onSelect({ kind: "recent" })}
          />
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
        <ScrollArea className="compact-playlist-scroll" scrollbars="x" scrollbarSize={4} type="hover">
          <Tabs
            className="compact-playlist-tabs"
            onChange={(value) => {
              if (value === "default") onSelect({ kind: "default" });
              else if (value?.startsWith("user:")) {
                const playlistId = Number(value.slice(5));
                if (Number.isFinite(playlistId)) onSelect({ kind: "user", playlistId });
              }
            }}
            value={activeTab}
            variant="pills"
          >
            <Tabs.List>
              <Tabs.Tab
                data-playing={activePlaylist?.kind === "default" || undefined}
                onContextMenu={(event) => {
                  event.preventDefault();
                  setContextMenu({ x: event.clientX, y: event.clientY, kind: "default" });
                }}
                value="default"
              >
                <PlaylistTabLabel label={t("library.default")} playing={activePlaylist?.kind === "default"} />
              </Tabs.Tab>
              {playlists.map((playlist) => {
                const playing = activePlaylist?.kind === "user" && activePlaylist.playlistId === playlist.id;
                return (
                  <Tabs.Tab
                    data-playing={playing || undefined}
                    key={playlist.id}
                    onContextMenu={(event) => {
                      event.preventDefault();
                      setContextMenu({
                        x: event.clientX,
                        y: event.clientY,
                        kind: "user",
                        playlistId: playlist.id,
                      });
                    }}
                    title={playlist.name}
                    value={`user:${playlist.id}`}
                  >
                    <PlaylistTabLabel label={playlist.name} playing={playing} />
                  </Tabs.Tab>
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
            <button onClick={() => { onRenamePlaylist(contextMenu.playlistId); setContextMenu(null); }} type="button">{t("library.rename")}</button>
            <button onClick={() => { onDeletePlaylist(contextMenu.playlistId); setContextMenu(null); }} type="button">{t("library.delete")}</button>
          </>}
        </Paper>
      </Portal>}
    </header>
  );
});

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
        color={active ? undefined : "gray"}
        onClick={onClick}
        size="sm"
        variant={active ? "light" : "subtle"}
      >
        {icon}
      </ActionIcon>
    </Tooltip>
  );
}

function PlaylistTabLabel({ label, playing }: { label: string; playing: boolean }) {
  return (
    <span className="compact-playlist-tab-label">
      {playing && <span aria-label="Playing" className="playlist-playing-indicator"><i /><i /><i /></span>}
      <span>{label}</span>
    </span>
  );
}
