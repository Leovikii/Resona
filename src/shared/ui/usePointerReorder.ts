import { useCallback, useEffect, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";

import { scrollViewportAtPointer, type ScrollAxis } from "./edgeAutoScroll";

interface ReorderItem {
  id: number;
}

interface PointerReorderOptions<T extends ReorderItem> {
  axis?: ScrollAxis;
  disabled: boolean;
  items: T[];
  onDragStart?: (itemId: number) => void;
  onMove: (itemId: number, toIndex: number) => void;
  scrollViewportRef?: { current: HTMLElement | null };
}

export function usePointerReorder<T extends ReorderItem>({
  axis = "vertical",
  disabled,
  items,
  onDragStart,
  onMove,
  scrollViewportRef,
}: PointerReorderOptions<T>) {
  const [draggedId, setDraggedId] = useState<number | null>(null);
  const [insertionPosition, setInsertionPosition] = useState<number | null>(null);
  const [targetIndex, setTargetIndex] = useState<number | null>(null);
  const listRef = useRef<HTMLDivElement | null>(null);
  const dragRef = useRef<{
    active: boolean;
    captureElement: HTMLElement;
    itemId: number;
    pointerId: number;
    startX: number;
    startY: number;
  } | null>(null);
  const targetIndexRef = useRef<number | null>(null);
  const suppressClickRef = useRef(false);
  const pointerRef = useRef<{ x: number; y: number } | null>(null);
  const edgeScrollFrameRef = useRef<number | null>(null);

  const reset = useCallback(() => {
    if (edgeScrollFrameRef.current !== null) {
      window.cancelAnimationFrame(edgeScrollFrameRef.current);
      edgeScrollFrameRef.current = null;
    }
    dragRef.current = null;
    pointerRef.current = null;
    targetIndexRef.current = null;
    setDraggedId(null);
    setInsertionPosition(null);
    setTargetIndex(null);
  }, []);

  const updateTarget = useCallback((clientX: number, clientY: number) => {
    const drag = dragRef.current;
    if (!drag) return;
    const from = items.findIndex((candidate) => candidate.id === drag.itemId);
    const insertion = listInsertionPosition(
      listRef.current,
      axis === "horizontal" ? clientX : clientY,
      axis,
    );
    const target = movedItemIndex(items.length, from, insertion);
    setInsertionPosition((current) => current === insertion ? current : insertion);
    if (targetIndexRef.current !== target) {
      targetIndexRef.current = target;
      setTargetIndex(target);
    }
  }, [axis, items]);

  const scheduleEdgeScroll = useCallback(() => {
    if (!scrollViewportRef?.current || edgeScrollFrameRef.current !== null) return;
    const tick = () => {
      edgeScrollFrameRef.current = null;
      const drag = dragRef.current;
      const pointer = pointerRef.current;
      const viewport = scrollViewportRef.current;
      if (!drag?.active || !pointer || !viewport) return;
      if (!scrollViewportAtPointer(viewport, axis, pointer.x, pointer.y)) return;
      updateTarget(pointer.x, pointer.y);
      edgeScrollFrameRef.current = window.requestAnimationFrame(tick);
    };
    edgeScrollFrameRef.current = window.requestAnimationFrame(tick);
  }, [axis, scrollViewportRef, updateTarget]);

  const onPointerDown = useCallback((
    event: ReactPointerEvent<HTMLElement>,
    itemId: number,
  ) => {
    if (disabled || event.button !== 0 || !event.isPrimary) return;
    dragRef.current = {
      active: false,
      captureElement: event.currentTarget,
      itemId,
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
  }, [disabled]);

  const onPointerMove = useCallback((event: ReactPointerEvent<HTMLElement>) => {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    if (!drag.active) {
      const distance = Math.hypot(
        event.clientX - drag.startX,
        event.clientY - drag.startY,
      );
      if (distance < 5) return;
      drag.active = true;
      suppressClickRef.current = true;
      onDragStart?.(drag.itemId);
      setDraggedId(drag.itemId);
    }
    pointerRef.current = { x: event.clientX, y: event.clientY };
    updateTarget(event.clientX, event.clientY);
    scheduleEdgeScroll();
  }, [onDragStart, scheduleEdgeScroll, updateTarget]);

  const finish = useCallback((pointerId: number, cancelled: boolean) => {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== pointerId) return;
    if (drag.captureElement.hasPointerCapture(pointerId)) {
      drag.captureElement.releasePointerCapture(pointerId);
    }
    if (drag.active && !cancelled) {
      suppressClickRef.current = true;
      window.setTimeout(() => {
        suppressClickRef.current = false;
      }, 0);
      const from = items.findIndex((candidate) => candidate.id === drag.itemId);
      const target = targetIndexRef.current ?? from;
      if (from >= 0 && from !== target) onMove(drag.itemId, target);
    } else if (cancelled) {
      suppressClickRef.current = false;
    }
    reset();
  }, [items, onMove, reset]);

  useEffect(() => {
    const finishPointer = (event: PointerEvent) => finish(event.pointerId, false);
    const cancelPointer = (event: PointerEvent) => finish(event.pointerId, true);
    window.addEventListener("pointerup", finishPointer);
    window.addEventListener("pointercancel", cancelPointer);
    return () => {
      window.removeEventListener("pointerup", finishPointer);
      window.removeEventListener("pointercancel", cancelPointer);
    };
  }, [finish]);

  const consumeClick = useCallback(() => {
    if (!suppressClickRef.current) return false;
    suppressClickRef.current = false;
    return true;
  }, []);

  return {
    consumeClick,
    draggedId,
    insertionPosition,
    listRef,
    onPointerCancel: (event: ReactPointerEvent<HTMLElement>) => finish(event.pointerId, true),
    onPointerDown,
    onPointerMove,
    onPointerUp: (event: ReactPointerEvent<HTMLElement>) => finish(event.pointerId, false),
    targetIndex,
  };
}

export function listInsertionPositionAtY(container: HTMLElement | null, clientY: number) {
  if (!container) return 0;
  const rows = Array.from(container.querySelectorAll<HTMLElement>("[data-track-position]"));
  for (const row of rows) {
    const bounds = row.getBoundingClientRect();
    if (clientY < bounds.top + bounds.height / 2) {
      const position = Number(row.dataset.trackPosition);
      return Number.isFinite(position) ? position : 0;
    }
  }
  return rows.length;
}

function listInsertionPosition(
  container: HTMLElement | null,
  coordinate: number,
  axis: ScrollAxis,
) {
  if (!container) return 0;
  const rows = Array.from(container.querySelectorAll<HTMLElement>("[data-reorder-position]"));
  for (const row of rows) {
    const bounds = row.getBoundingClientRect();
    const midpoint = axis === "horizontal"
      ? bounds.left + bounds.width / 2
      : bounds.top + bounds.height / 2;
    if (coordinate < midpoint) {
      const position = Number(row.dataset.reorderPosition);
      return Number.isFinite(position) ? position : 0;
    }
  }
  return rows.length;
}

function movedItemIndex(itemCount: number, from: number, insertion: number) {
  return Math.max(
    0,
    Math.min(itemCount - 1, from < insertion ? insertion - 1 : insertion),
  );
}
