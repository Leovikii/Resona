import { getCurrentWindow } from "@tauri-apps/api/window";

import { invokeTauri, isTauriRuntime } from "./tauri";

export type WindowMaterial = "mica" | "solid";
export type WindowTheme = "auto" | "light" | "dark";

let initialization: Promise<WindowMaterial> | null = null;
let synchronizedTheme: WindowTheme | null = null;

export function prepareWindowAppearance() {
  const requested = new URLSearchParams(window.location.search).get("material");
  const material = requested === "mica" ? "mica" : "solid";
  document.documentElement.toggleAttribute(
    "data-window-material-preview",
    !isTauriRuntime() && material === "mica",
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
  initialization = invokeTauri<WindowMaterial>("sync_window_theme", { label, theme })
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
  return new URLSearchParams(window.location.search).get("material") === "mica"
    ? "mica"
    : "solid";
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
  document.documentElement.dataset.windowMaterial = material;
}
