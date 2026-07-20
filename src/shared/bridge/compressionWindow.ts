import { invokeTauri, isTauriRuntime } from "./tauri";

export async function showAudioCompressionWindow() {
  if (isTauriRuntime()) {
    await invokeTauri<void>("show_audio_compression_window");
    return;
  }
  const url = new URL(window.location.href);
  url.search = "?window=audio-compression";
  window.location.assign(url);
}
