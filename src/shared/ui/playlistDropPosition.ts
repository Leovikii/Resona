// Keep the visible gaps stable; only the external file drop hit area expands.
export function playlistDropPosition(
  pointer: number,
  gaps: readonly { center: number; position: number }[],
): number | null {
  let nearest: number | null = null;
  let distance = 14;
  for (const gap of gaps) {
    const next = Math.abs(pointer - gap.center);
    if (next <= distance) {
      nearest = gap.position;
      distance = next;
    }
  }
  return nearest;
}
