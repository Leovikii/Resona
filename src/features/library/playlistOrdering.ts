import type { PlaylistSummary } from "../../shared/model/library";

export function movePlaylistSummary(
  playlists: PlaylistSummary[],
  playlistId: number,
  toPosition: number,
) {
  const from = playlists.findIndex((playlist) => playlist.id === playlistId);
  if (from < 0) return playlists;
  const target = clamp(toPosition, 0, playlists.length - 1);
  if (from === target) return playlists;
  const next = [...playlists];
  const [moved] = next.splice(from, 1);
  next.splice(target, 0, moved);
  return next.map((playlist, position) => ({ ...playlist, position }));
}

function clamp(value: number, minimum: number, maximum: number) {
  return Math.max(minimum, Math.min(value, maximum));
}
