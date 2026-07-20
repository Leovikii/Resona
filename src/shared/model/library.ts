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
  path: string;
  displayName: string;
}

export interface DefaultPlaylistSnapshot {
  revision: number;
  sourceDirectory: string | null;
  selectedIndex: number | null;
  items: DefaultPlaylistItem[];
}

export interface OpenMediaResult {
  playback: import("./playback").PlaybackSnapshot;
  defaultPlaylist: DefaultPlaylistSnapshot;
}

export interface PlaylistItem {
  id: number;
  playlistId: number;
  path: string;
  displayName: string;
  position: number;
}

export interface RecentPlayRecord {
  path: string;
  displayName: string;
  lastPlayedAt: number;
  playCount: number;
}

export interface RejectedPath {
  path: string;
  reason: "missing" | "unsupported" | "unreadable" | "empty_folder";
}

export interface PlaylistMutationResult extends PlaylistDetails {
  rejected: RejectedPath[];
}
