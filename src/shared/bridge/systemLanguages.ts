import { invokeTauri, isTauriRuntime } from "./tauri";

export function readSystemLanguages() {
  const browserLanguages = navigator.languages.length > 0 ? [...navigator.languages] : [navigator.language];
  return isTauriRuntime()
    ? invokeTauri<string[]>("get_system_languages", { browserLanguages })
    : Promise.resolve(browserLanguages);
}
