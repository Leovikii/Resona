import { lazy, StrictMode, Suspense } from "react";
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { AppProvider } from "./app/preferences";
import { isTauriRuntime } from "./shared/bridge/tauri";
import { prepareWindowAppearance } from "./shared/bridge/windowAppearance";
import "./shared/i18n";
import "./styles.css";

const desktopLyricsWindow = new URLSearchParams(window.location.search).get("window") === "desktop-lyrics"
  || (isTauriRuntime() && getCurrentWindow().label === "desktop-lyrics");
const audioCompressionWindow = new URLSearchParams(window.location.search).get("window") === "audio-compression"
  || (isTauriRuntime() && getCurrentWindow().label === "audio-compression");
document.documentElement.dataset.window = desktopLyricsWindow
  ? "desktop-lyrics"
  : audioCompressionWindow
    ? "audio-compression"
    : "main";
prepareWindowAppearance();
const RootWindow = desktopLyricsWindow
  ? lazy(() => import("./windows/DesktopLyricsWindow"))
  : audioCompressionWindow
    ? lazy(() => import("./windows/AudioCompressionWindow"))
    : lazy(() => import("./App"));

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <AppProvider>
      <Suspense fallback={null}>
        <RootWindow />
      </Suspense>
    </AppProvider>
  </StrictMode>,
);
