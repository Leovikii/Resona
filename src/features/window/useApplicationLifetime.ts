import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import { invokeTauri, isTauriRuntime } from "../../shared/bridge/tauri";
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
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    void invokeTauri<ApplicationLifetimeSnapshot>("get_application_lifetime_state")
      .then((next) => {
        setSnapshot(next);
        setError(null);
      })
      .catch((nextError) => setError(errorMessage(nextError)));
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen("resona://exit-confirmation-requested", () => {
      setClosePromptOpen(false);
      setExitConfirmationOpen(true);
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen("resona://close-requested", () => setClosePromptOpen(true)).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
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
    }
  }, [snapshot.closeBehavior]);

  const confirmExit = useCallback(async () => {
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
    }
  }, []);

  return {
    closePromptOpen,
    dismissClosePrompt: () => setClosePromptOpen(false),
    dismissExitConfirmation: () => setExitConfirmationOpen(false),
    error,
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
