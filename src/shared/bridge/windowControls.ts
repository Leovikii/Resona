import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { invokeTauri, isTauriRuntime } from "./tauri";

export type WindowAction = "minimize" | "maximize" | "close" | "menu";

export async function resizeWindow(direction: Parameters<ReturnType<typeof getCurrentWindow>["startResizeDragging"]>[0]) {
  if (isTauriRuntime()) await getCurrentWindow().startResizeDragging(direction);
}

// close() requests a native close event; the existing lifetime service owns the decision.
export async function performWindowAction(action: WindowAction) {
  if (!isTauriRuntime()) return;
  const window = getCurrentWindow();
  switch (action) {
    case "minimize": return window.minimize();
    case "maximize": return window.toggleMaximize();
    case "close": return window.close();
    case "menu": return invokeTauri<void>("show_window_menu");
  }
}

export async function observeWindowState(update: (state: { focused: boolean; maximized: boolean; nativeMaximizeHovered: boolean }) => void) {
  if (!isTauriRuntime()) return () => {};
  const window = getCurrentWindow();
  let active = true;
  let nativeMaximizeHovered = false;
  const refresh = async () => {
    const [focused, maximized] = await Promise.all([window.isFocused(), window.isMaximized()]);
    if (active) update({ focused, maximized, nativeMaximizeHovered });
  };
  const stops: (() => void)[] = [];
  try {
    stops.push(await listen<boolean>("resona://caption-maximize-hover", (event) => {
      nativeMaximizeHovered = event.payload;
      void refresh().catch(console.error);
    }));
    stops.push(await window.onResized(() => void refresh().catch(console.error)));
    stops.push(await window.onFocusChanged(() => void refresh().catch(console.error)));
    await refresh();
  } catch (error) {
    active = false;
    stops.forEach((stop) => stop());
    throw error;
  }
  return () => { active = false; stops.forEach((stop) => stop()); };
}
