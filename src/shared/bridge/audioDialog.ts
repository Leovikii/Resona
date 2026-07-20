import { open } from "@tauri-apps/plugin-dialog";

import { isTauriRuntime } from "./tauri";

let pending = false;

export async function selectAudioFiles(multiple: boolean): Promise<string[]> {
  return selectFiles(multiple, { name: "MP3 / WAV / FLAC", extensions: ["mp3", "wav", "flac"] });
}

export async function selectWavFiles(): Promise<string[]> {
  return selectFiles(true, { name: "WAV", extensions: ["wav"] });
}

export async function selectCompressionFolders(): Promise<string[]> {
  if (pending || !isTauriRuntime()) return [];
  pending = true;
  try {
    const selected = await open({
      title: "Resona",
      multiple: true,
      directory: true,
    });
    return Array.isArray(selected) ? selected : selected ? [selected] : [];
  } finally {
    pending = false;
  }
}

async function selectFiles(
  multiple: boolean,
  filter: { name: string; extensions: string[] },
): Promise<string[]> {
  if (pending || !isTauriRuntime()) return [];
  pending = true;
  try {
    const selected = await open({
      title: "Resona",
      multiple,
      directory: false,
      filters: [filter],
    });
    return Array.isArray(selected) ? selected : selected ? [selected] : [];
  } finally {
    pending = false;
  }
}
