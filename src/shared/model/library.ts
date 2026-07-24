export interface PlaylistSummary {
  id: number;
  name: string;
  position: number;
  itemCount: number;
  createdAt: number;
  updatedAt: number;
}

export interface PlaylistDetails {
  playlist: PlaylistSummary;
  items: PlaylistItem[];
}

export interface DefaultPlaylistItem {
  id: number;
  path: string;
  displayName: string;
}

export interface DefaultPlaylistSnapshot {
  revision: number;
  sourceDirectory: string | null;
  selectedIndex: number | null;
  items: DefaultPlaylistItem[];
}

export interface ActivePlaylistSnapshot {
  kind: "default" | "user";
  playlistId: number | null;
}

export interface OpenMediaResult {
  playback: import("./playback").PlaybackSnapshot;
  defaultPlaylist: DefaultPlaylistSnapshot;
  activePlaylist: ActivePlaylistSnapshot;
}

export interface PlaylistPlaybackResult {
  playback: import("./playback").PlaybackSnapshot;
  activePlaylist: ActivePlaylistSnapshot;
}

export interface DefaultPlaylistMutationResult {
  defaultPlaylist: DefaultPlaylistSnapshot;
  rejected: RejectedPath[];
}

export interface PlaylistItem {
  id: number;
  playlistId: number;
  path: string;
  displayName: string;
  position: number;
}

export interface RejectedPath {
  path: string;
  reason: "missing" | "unsupported" | "unreadable" | "empty_folder" | "duplicate";
}

export interface PlaylistMutationResult extends PlaylistDetails {
  rejected: RejectedPath[];
}
