import { useEffect, useRef, useState } from "react";
import { isAudioDialogOpen } from "../../shared/bridge/audioDialog";
import { invokeTauri, isTauriRuntime } from "../../shared/bridge/tauri";
import { previewSnapshot, type PlaybackFailure, type PlaybackSnapshot } from "../../shared/model/playback";
import { createControlQueue, interactiveSelector, previewPlaybackControl, playbackControlForKey, type PlaybackControl } from "./playbackShortcuts";

export function usePlaybackShortcuts(options: {
  blocked?: boolean;
  run?: (action: PlaybackControl, steps: number) => Promise<PlaybackSnapshot | null>;
  onResult?: (action: PlaybackControl, snapshot: PlaybackSnapshot) => void;
} = {}) {
  const optionsRef = useRef(options);
  optionsRef.current = options;
  const [feedback, setFeedback] = useState<{ id: number; action: PlaybackControl; snapshot: PlaybackSnapshot } | null>(null);
  const [error, setError] = useState<PlaybackFailure | null>(null);
  const sequence = useRef(0);
  useEffect(() => {
    let fixture = previewSnapshot();
    const queue = createControlQueue(
      (action, steps) => optionsRef.current.run
        ? optionsRef.current.run(action, steps)
        : import.meta.env.DEV && !isTauriRuntime()
          ? Promise.resolve(fixture = previewPlaybackControl(fixture, action, steps))
          : invokeTauri<PlaybackSnapshot>("control_playback", { action, steps }),
      (action, snapshot) => {
        if (!snapshot) return;
        setError(null);
        optionsRef.current.onResult?.(action, snapshot);
        setFeedback({ id: ++sequence.current, action, snapshot });
      },
      (failure) => setError(typeof failure === "object" && failure !== null && "code" in failure
        ? failure as PlaybackFailure : { code: "task_failed", message: String(failure) }),
    );
    const blocked = () => optionsRef.current.blocked || isAudioDialogOpen()
      || document.visibilityState !== "visible" || !document.hasFocus()
      || [...document.querySelectorAll('[aria-modal="true"],[role="menu"],[role="dialog"]')].some((element) => element.getClientRects().length > 0);
    const down = (event: KeyboardEvent) => {
      const action = playbackControlForKey(event);
      if (!action || blocked() || event.composedPath().some((node) => node instanceof Element && node.matches(interactiveSelector))) return;
      event.preventDefault();
      queue.push(action, event.repeat);
    };
    const up = (event: KeyboardEvent) => {
      const action = playbackControlForKey({ key: event.key, repeat: false, defaultPrevented: false, isComposing: false, altKey: false, ctrlKey: false, shiftKey: false, metaKey: false });
      if (action) queue.release(action);
    };
    const clear = () => queue.clear();
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    window.addEventListener("blur", clear);
    document.addEventListener("visibilitychange", clear);
    return () => {
      queue.dispose();
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
      window.removeEventListener("blur", clear);
      document.removeEventListener("visibilitychange", clear);
    };
  }, []);
  return { feedback, error };
}
