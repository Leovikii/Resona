import { useCallback, useEffect, useRef, useState } from "react";

import { invokeTauri, isTauriRuntime, listenTauri } from "../../shared/bridge/tauri";
import {
  defaultApplicationLifetimeSnapshot,
  type ApplicationLifetimeSnapshot,
  type CloseBehavior,
  type CloseDecision,
} from "../../shared/model/applicationLifetime";

export function useApplicationLifetime() {
  const preview = import.meta.env.DEV && !isTauriRuntime();
  const [snapshot, setSnapshot] = useState<ApplicationLifetimeSnapshot>(
    defaultApplicationLifetimeSnapshot,
  );
  const [closePromptOpen, setClosePromptOpen] = useState(
    preview && new URLSearchParams(window.location.search).get("closePrompt") === "1",
  );
  const [exitConfirmationOpen, setExitConfirmationOpen] = useState(
    preview && new URLSearchParams(window.location.search).get("exitPrompt") === "1",
  );
  const pendingRef = useRef(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    void invokeTauri<ApplicationLifetimeSnapshot>("get_application_lifetime_state")
      .then((next) => {
        if (disposed) return;
        setSnapshot(next);
        setError(null);
      })
      .catch((nextError) => { if (!disposed) setError(errorMessage(nextError)); });
    return () => { disposed = true; };
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listenTauri("resona://exit-confirmation-requested", () => {
      if (disposed) return;
      setClosePromptOpen(false);
      setExitConfirmationOpen(true);
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    }).catch((cause) => { if (!disposed) setError(errorMessage(cause)); });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listenTauri("resona://close-requested", () => { if (!disposed) setClosePromptOpen(true); }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    }).catch((cause) => { if (!disposed) setError(errorMessage(cause)); });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const setCloseBehavior = useCallback(async (behavior: CloseBehavior) => {
    try {
      const next = isTauriRuntime()
        ? await invokeTauri<ApplicationLifetimeSnapshot>("set_close_behavior", { behavior })
        : { closeBehavior: behavior };
      setSnapshot(next);
      setError(null);
      return true;
    } catch (nextError) {
      setError(errorMessage(nextError));
      return false;
    }
  }, []);

  const resolveClose = useCallback(async (decision: CloseDecision, remember: boolean) => {
    if (pendingRef.current) return false;
    pendingRef.current = true;
    setPending(true);
    try {
      const next = isTauriRuntime()
        ? await invokeTauri<ApplicationLifetimeSnapshot>("resolve_main_window_close", {
            decision,
            remember,
          })
        : { closeBehavior: remember ? decision : snapshot.closeBehavior };
      setSnapshot(next);
      setClosePromptOpen(false);
      setError(null);
      return true;
    } catch (nextError) {
      setError(errorMessage(nextError));
      return false;
    } finally {
      pendingRef.current = false;
      setPending(false);
    }
  }, [snapshot.closeBehavior]);

  const confirmExit = useCallback(async () => {
    if (pendingRef.current) return false;
    pendingRef.current = true;
    setPending(true);
    try {
      if (isTauriRuntime()) {
        await invokeTauri<void>("confirm_application_exit");
      } else {
        setExitConfirmationOpen(false);
      }
      setError(null);
      return true;
    } catch (nextError) {
      setError(errorMessage(nextError));
      return false;
    } finally {
      pendingRef.current = false;
      setPending(false);
    }
  }, []);

  return {
    closePromptOpen,
    dismissClosePrompt: () => { if (!pendingRef.current) setClosePromptOpen(false); },
    dismissExitConfirmation: () => { if (!pendingRef.current) setExitConfirmationOpen(false); },
    error,
    pending,
    exitConfirmationOpen,
    confirmExit,
    resolveClose,
    setCloseBehavior,
    snapshot,
  };
}

function errorMessage(error: unknown) {
  if (typeof error === "object" && error !== null && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}
