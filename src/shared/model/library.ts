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

export interface CueTrackSource {
  cuePath: string;
  trackNumber: number;
  title: string | null;
  performer: string | null;
  album: string | null;
  startMs: number;
  endMs: number | null;
}

export interface DefaultPlaylistItem {
  id: number;
  path: string;
  displayName: string;
  folderRoot: string | null;
  cue?: CueTrackSource | null;
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
  folderRoot: string | null;
  cue?: CueTrackSource | null;
}

export interface RejectedPath {
  path: string;
  reason: "missing" | "unsupported" | "unreadable" | "linked_path" | "empty_folder" | "duplicate";
}

export interface PlaylistMutationResult extends PlaylistDetails {
  rejected: RejectedPath[];
}
