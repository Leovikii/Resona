export type ScrollAxis = "horizontal" | "vertical";

export function edgeScrollDelta(start: number, end: number, coordinate: number, maximumStep = 12) {
  const length = end - start;
  if (length <= 0 || maximumStep <= 0) return 0;
  const threshold = Math.min(32, Math.max(16, length / 4), length / 2);
  if (coordinate < start + threshold) {
    const intensity = Math.min(1, (start + threshold - coordinate) / threshold);
    return -Math.ceil(intensity * maximumStep);
  }
  if (coordinate > end - threshold) {
    const intensity = Math.min(1, (coordinate - (end - threshold)) / threshold);
    return Math.ceil(intensity * maximumStep);
  }
  return 0;
}

export function scrollViewportAtPointer(
  viewport: HTMLElement,
  axis: ScrollAxis,
  clientX: number,
  clientY: number,
) {
  const bounds = viewport.getBoundingClientRect();
  const coordinate = axis === "horizontal" ? clientX : clientY;
  const start = axis === "horizontal" ? bounds.left : bounds.top;
  const end = axis === "horizontal" ? bounds.right : bounds.bottom;
  const delta = edgeScrollDelta(start, end, coordinate);
  if (delta === 0) return false;

  const before = axis === "horizontal" ? viewport.scrollLeft : viewport.scrollTop;
  if (axis === "horizontal") viewport.scrollLeft += delta;
  else viewport.scrollTop += delta;
  const after = axis === "horizontal" ? viewport.scrollLeft : viewport.scrollTop;
  return Math.abs(after - before) > 0.5;
}
