export interface TrackDetails {
  path: string;
  fileName: string;
  title: string | null;
  artist: string | null;
  album: string | null;
  durationMs: number | null;
  sampleRate: number | null;
  bitDepth: number | null;
  channels: number | null;
  audioBitrate: number | null;
  codec: string;
  fileSize: number | null;
  artworkDataUrl: string | null;
  metadataWarning: string | null;
  quality: "hi_res" | "sq" | "hq" | null;
}
