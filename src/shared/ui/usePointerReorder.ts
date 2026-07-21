import { useCallback, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";

interface ReorderItem {
  id: number;
}

interface PointerReorderOptions<T extends ReorderItem> {
  disabled: boolean;
  items: T[];
  onDragStart?: (itemId: number) => void;
  onMove: (itemId: number, toIndex: number) => void;
}

export function usePointerReorder<T extends ReorderItem>({
  disabled,
  items,
  onDragStart,
  onMove,
}: PointerReorderOptions<T>) {
  const [draggedId, setDraggedId] = useState<number | null>(null);
  const [insertionPosition, setInsertionPosition] = useState<number | null>(null);
  const [targetIndex, setTargetIndex] = useState<number | null>(null);
  const listRef = useRef<HTMLDivElement | null>(null);
  const dragRef = useRef<{
    active: boolean;
    itemId: number;
    pointerId: number;
    startX: number;
    startY: number;
  } | null>(null);
  const targetIndexRef = useRef<number | null>(null);
  const suppressClickRef = useRef(false);

  const reset = useCallback(() => {
    dragRef.current = null;
    targetIndexRef.current = null;
    setDraggedId(null);
    setInsertionPosition(null);
    setTargetIndex(null);
  }, []);

  const onPointerDown = useCallback((
    event: ReactPointerEvent<HTMLElement>,
    itemId: number,
  ) => {
    if (disabled || event.button !== 0 || !event.isPrimary) return;
    dragRef.current = {
      active: false,
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
    const from = items.findIndex((candidate) => candidate.id === drag.itemId);
    const insertion = listInsertionPositionAtY(listRef.current, event.clientY);
    const target = movedItemIndex(items.length, from, insertion);
    setInsertionPosition((current) => current === insertion ? current : insertion);
    if (targetIndexRef.current !== target) {
      targetIndexRef.current = target;
      setTargetIndex(target);
    }
  }, [items, onDragStart]);

  const finish = useCallback((event: ReactPointerEvent<HTMLElement>, cancelled: boolean) => {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
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
    onPointerCancel: (event: ReactPointerEvent<HTMLElement>) => finish(event, true),
    onPointerDown,
    onPointerMove,
    onPointerUp: (event: ReactPointerEvent<HTMLElement>) => finish(event, false),
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

function movedItemIndex(itemCount: number, from: number, insertion: number) {
  return Math.max(
    0,
    Math.min(itemCount - 1, from < insertion ? insertion - 1 : insertion),
  );
}
