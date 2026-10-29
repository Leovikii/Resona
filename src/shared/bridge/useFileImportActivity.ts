import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invokeTauri, isTauriRuntime } from "./tauri";
import { createImportGate } from "./importGate";
import { useDocumentVisibility } from "../ui/useDocumentVisibility";

type ImportSnapshot = { busy: boolean; scanning: boolean };
export type FileImportActivity = ReturnType<typeof useFileImportActivity>;

export function useFileImportActivity() {
  const visible = useDocumentVisibility();
  const preview = import.meta.env.DEV && !isTauriRuntime()
    && new URLSearchParams(window.location.search).get("preview") === "importing";
  const [remote, setRemote] = useState<ImportSnapshot>({ busy: preview, scanning: preview });
  const [pending, setPending] = useState(false);
  const [showStatus, setShowStatus] = useState(false);
  const remoteBusy = useRef(remote.busy);
  remoteBusy.current = remote.busy;
  const gate = useMemo(() => createImportGate(setPending), []);
  const refresh = useCallback(async () => {
    if (!isTauriRuntime()) return;
    try { setRemote(await invokeTauri<ImportSnapshot>("get_file_import_state")); }
    catch (error) { console.warn("Unable to read file task activity", error); }
  }, []);
  useEffect(() => {
    if (!visible) return;
    void refresh();
    if (!isTauriRuntime()) return;
    const timer = window.setInterval(() => void refresh(), 500);
    return () => window.clearInterval(timer);
  }, [refresh, visible]);
  const run = useCallback(async <T,>(operation: () => Promise<T>): Promise<T | null> => {
    if (remoteBusy.current) return null;
    return gate(operation).finally(() => void refresh());
  }, [gate, refresh]);
  const scanning = pending || remote.scanning;
  useEffect(() => {
    if (!scanning) { setShowStatus(false); return; }
    const timer = window.setTimeout(() => setShowStatus(true), 200);
    return () => window.clearTimeout(timer);
  }, [scanning]);
  return { busy: pending || remote.busy, showStatus, run };
}
