import { invoke } from "@tauri-apps/api/core";

export function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function invokeTauri<T>(command: string, args?: Record<string, unknown>) {
  return invoke<T>(command, args);
}
