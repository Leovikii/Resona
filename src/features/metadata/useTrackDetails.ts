import { useEffect, useState } from "react";

import { invokeTauri, isTauriRuntime } from "../../shared/bridge/tauri";
import type { TrackDetails } from "../../shared/model/metadata";
import { fileNameFromPath } from "../../shared/utils/format";

export function useTrackDetails(path: string | null) {
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
      const fileName = fileNameFromPath(path);
      const featured = fileName === "Midnight Signal.flac";
      setState({
        loading: false,
        error: null,
        details: {
          path,
          fileName,
          title: featured ? "Midnight Signal" : fileName,
          artist: featured ? "Resona" : null,
          album: featured ? "Local Sessions" : null,
          genre: featured ? "Electronic" : null,
          trackNumber: featured ? 2 : null,
          trackTotal: featured ? 8 : null,
          discNumber: null,
          discTotal: null,
          date: featured ? "2026" : null,
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
        },
      });
      return;
    }
    let disposed = false;
    setState((current) => ({ ...current, loading: true, error: null }));
    void invokeTauri<TrackDetails>("get_track_details", { path })
      .then(async (details) => {
        if (disposed) return;
        await decodeArtwork(details.artworkDataUrl);
        if (!disposed) setState({ details, loading: false, error: null });
      })
      .catch((error) => {
        if (!disposed) setState({ details: null, loading: false, error: String(error) });
      });
    return () => { disposed = true; };
  }, [path]);

  return state;
}

async function decodeArtwork(source: string | null) {
  if (!source) return;
  const image = new Image();
  image.src = source;
  try {
    await image.decode();
  } catch (error) {
    console.warn("track artwork pre-decode failed", error);
  }
}
