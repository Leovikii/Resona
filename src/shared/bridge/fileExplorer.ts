import { invokeTauri, isTauriRuntime } from "./tauri";

export async function revealAudioFile(path: string) {
  if (!isTauriRuntime()) return;
  await invokeTauri("reveal_audio_file", { path });
}
