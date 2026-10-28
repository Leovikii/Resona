import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";

import { invokeTauri, isTauriRuntime } from "./tauri";

export type WindowMaterial = "mica" | "kde_blur" | "solid";
export type WindowTheme = "auto" | "light" | "dark";

let initialization: Promise<WindowMaterial> | null = null;
let synchronizedTheme: WindowTheme | null = null;
let regionObserver: ResizeObserver | null = null;

export function prepareWindowAppearance() {
  const requested = new URLSearchParams(window.location.search).get("material");
  const material = requested === "mica" || requested === "kde_blur" ? requested : "solid";
  document.documentElement.toggleAttribute(
    "data-window-material-preview",
    !isTauriRuntime() && material !== "solid",
  );
  applyWindowMaterial(material);
}

export function initializeCurrentWindowMaterial(): Promise<WindowMaterial> {
  if (initialization) return initialization;
  if (!isTauriRuntime()) {
    const material = previewMaterial();
    applyWindowMaterial(material);
    return Promise.resolve(material);
  }

  const label = getCurrentWindow().label;
  if (label === "desktop-lyrics") {
    applyWindowMaterial("solid");
    return Promise.resolve("solid");
  }

  const theme = initialWindowTheme();
  initialization = listen<WindowMaterial>("resona://window-material", (event) => applyWindowMaterial(event.payload))
    .then(() => invokeTauri<WindowMaterial>("sync_window_theme", { label, theme }))
    .then((material) => {
      synchronizedTheme = theme;
      applyWindowMaterial(material);
      return material;
    })
    .catch((error) => {
      console.error("Unable to initialize native window material", error);
      applyWindowMaterial("solid");
      return "solid" as const;
    });
  return initialization;
}

export async function syncCurrentWindowTheme(theme: WindowTheme) {
  if (!isTauriRuntime() || getCurrentWindow().label === "desktop-lyrics") return;
  try {
    await initializeCurrentWindowMaterial();
    if (synchronizedTheme === theme) return;
    const material = await invokeTauri<WindowMaterial>("sync_window_theme", {
      label: getCurrentWindow().label,
      theme,
    });
    synchronizedTheme = theme;
    applyWindowMaterial(material);
  } catch (error) {
    console.error("Unable to synchronize native window theme", error);
  }
}

function previewMaterial(): WindowMaterial {
  const material = new URLSearchParams(window.location.search).get("material");
  return material === "mica" || material === "kde_blur" ? material : "solid";
}

function initialWindowTheme(): WindowTheme {
  try {
    const preference = localStorage.getItem("resona-color-scheme");
    if (preference === "auto" || preference === "light" || preference === "dark") {
      return preference;
    }
  } catch (error) {
    console.warn("Unable to read the initial window theme", error);
  }
  return "auto";
}

function applyWindowMaterial(material: WindowMaterial) {
  if (document.documentElement.dataset.windowMaterial === material) return;
  document.documentElement.dataset.windowMaterial = material;
  regionObserver?.disconnect();
  regionObserver = null;
  if (material !== "kde_blur" || !isTauriRuntime()) return;
  const update = () => {
    const content = document.querySelector(".main-region, .compression-workspace");
    if (!content) return;
    const { x, y, width, height } = content.getBoundingClientRect();
    void invokeTauri("set_window_blur_content_rect", {
      rect: { x: Math.round(x), y: Math.round(y), width: Math.round(width), height: Math.round(height) },
    }).catch((error) => console.error("Unable to update KDE blur region", error));
  };
  // Only geometry changes produce requests; playback updates do not touch blur.
  regionObserver = new ResizeObserver(update);
  const content = document.querySelector(".main-region, .compression-workspace");
  if (content) regionObserver.observe(content);
  regionObserver.observe(document.documentElement);
  update();
}
