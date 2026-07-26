import { useEffect, useState } from "react";

import { invokeTauri, isTauriRuntime } from "../../shared/bridge/tauri";
import type { TrackDetails } from "../../shared/model/metadata";
import { fileNameFromPath } from "../../shared/utils/format";

export function useAudioFileInfo(path: string | null) {
  const [state, setState] = useState<{
    details: TrackDetails | null;
    loading: boolean;
    error: string | null;
  }>({ details: null, loading: false, error: null });

  useEffect(() => {
    if (!path) {
      setState({ details: null, loading: false, error: null });
      return;
    }
    if (!isTauriRuntime()) {
      setState({ details: previewDetails(path), loading: false, error: null });
      return;
    }
    let disposed = false;
    setState({ details: null, loading: true, error: null });
    void invokeTauri<TrackDetails>("get_audio_file_info", { path })
      .then((details) => {
        if (!disposed) setState({ details, loading: false, error: null });
      })
      .catch((error) => {
        if (!disposed) setState({ details: null, loading: false, error: String(error) });
      });
    return () => { disposed = true; };
  }, [path]);

  return state;
}

function previewDetails(path: string): TrackDetails {
  return {
    path,
    fileName: fileNameFromPath(path),
    title: "Midnight Signal",
    artist: "Resona",
    album: "Local Sessions",
    genre: "Electronic",
    trackNumber: 2,
    trackTotal: 8,
    discNumber: 1,
    discTotal: 1,
    date: "2026",
    durationMs: 247_000,
    sampleRate: 96_000,
    bitDepth: 24,
    channels: 2,
    audioBitrate: 2_304,
    codec: "FLAC",
    fileSize: 58_240_000,
    artworkDataUrl: null,
    metadataWarning: null,
    quality: "hi_res",
  };
}
