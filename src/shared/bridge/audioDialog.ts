import { invokeTauri, isTauriRuntime } from "./tauri";

let pending = false;
export function isAudioDialogOpen() { return pending; }

export function selectAudioFiles(multiple: boolean) { return select("audio", multiple); }
export function selectWavFiles() { return select("wav", true); }
export function selectAudioFolders() { return select("folders", true); }
export function selectCompressionFolders() { return select("folders", true); }

async function select(kind: "audio" | "wav" | "folders", multiple: boolean): Promise<string[]> {
  if (pending || !isTauriRuntime()) return [];
  pending = true;
  try { return await invokeTauri<string[]>("select_media_paths", { kind, multiple }); }
  finally { pending = false; }
}
