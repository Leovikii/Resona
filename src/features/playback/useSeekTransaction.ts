import { useCallback, useEffect, useRef, useState } from "react";

import type { PlaybackSnapshot } from "../../shared/model/playback";

type SeekCommand = (positionMs: number) => Promise<PlaybackSnapshot | null>;

export interface SeekTransaction {
  displayPositionMs: number;
  dragging: boolean;
  pending: boolean;
  setDragPosition: (positionMs: number) => void;
  requestSeek: (positionMs: number) => Promise<boolean>;
}

export function useSeekTransaction(
  authoritativePositionMs: number,
  runSeek: SeekCommand,
): SeekTransaction {
  const [dragging, setDragging] = useState(false);
  const [pendingTarget, setPendingTarget] = useState<number | null>(null);
  const [dragPosition, setDragPositionState] = useState(authoritativePositionMs);
  const authoritativeRef = useRef(authoritativePositionMs);
  const transactionRef = useRef(0);

  useEffect(() => {
    authoritativeRef.current = authoritativePositionMs;
    if (
      !dragging
      && pendingTarget !== null
      && Math.abs(authoritativePositionMs - pendingTarget) <= 1_000
    ) {
      setPendingTarget(null);
      setDragPositionState(authoritativePositionMs);
      return;
    }
    if (!dragging && pendingTarget === null) {
      setDragPositionState(authoritativePositionMs);
    }
  }, [authoritativePositionMs, dragging, pendingTarget]);

  const setDragPosition = useCallback((positionMs: number) => {
    setDragging(true);
    setDragPositionState(positionMs);
  }, []);

  const requestSeek = useCallback(async (positionMs: number) => {
    const target = Math.max(0, Math.round(positionMs));
    const transaction = ++transactionRef.current;
    setDragging(false);
    setDragPositionState(target);
    setPendingTarget(target);

    const result = await runSeek(target);
    if (transaction !== transactionRef.current) return result !== null;

    if (result) {
      authoritativeRef.current = result.positionMs;
      setDragPositionState(result.positionMs);
      setPendingTarget(result.positionMs);
    } else {
      setDragPositionState(authoritativeRef.current);
      setPendingTarget(null);
    }
    return result !== null;
  }, [runSeek]);

  return {
    displayPositionMs: dragging ? dragPosition : pendingTarget ?? authoritativePositionMs,
    dragging,
    pending: pendingTarget !== null,
    setDragPosition,
    requestSeek,
  };
}
