export type CompressionPreset = "fast" | "balanced" | "smallest";
export type FfmpegDependencyStatus =
  | "checking"
  | "missing"
  | "downloading"
  | "installing"
  | "cancelling"
  | "cancelled"
  | "ready"
  | "failed";

export interface FfmpegDependencyFailure {
  code: string;
  message: string;
}

export interface FfmpegDependencySnapshot {
  status: FfmpegDependencyStatus;
  version: string;
  sourceUrl: string;
  license: string;
  downloadedBytes: number;
  totalBytes: number | null;
  installedBytes: number;
  error: FfmpegDependencyFailure | null;
}
export type CompressionStatus = "idle" | "running" | "cancelling" | "completed" | "completed_with_errors" | "cancelled";

export interface CompressionItem {
  source: string;
  output: string;
  status: "pending" | "running" | "completed" | "failed" | "cancelled";
  message: string | null;
  sourceDeleted: boolean;
}

export interface CompressionSnapshot {
  taskId: number;
  status: CompressionStatus;
  completed: number;
  total: number;
  currentProgress: number;
  items: CompressionItem[];
}

export type CompressionScanStatus = "idle" | "scanning" | "cancelling" | "ready" | "cancelled" | "failed";

export interface CompressionScanNode {
  path: string;
  name: string;
  kind: "root" | "directory" | "file";
  ready: boolean;
  issueCode: string | null;
  children: CompressionScanNode[];
}

export interface CompressionScanWarning {
  path: string;
  code: string;
  message: string;
}

export interface CompressionScanSnapshot {
  scanId: number;
  status: CompressionScanStatus;
  inputRoots: string[];
  scannedEntries: number;
  candidateFiles: number;
  validatedFiles: number;
  readyFiles: number;
  roots: CompressionScanNode[];
  warnings: CompressionScanWarning[];
}

export const emptyCompressionSnapshot: CompressionSnapshot = {
  taskId: 0,
  status: "idle",
  completed: 0,
  total: 0,
  currentProgress: 0,
  items: [],
};

export const checkingFfmpegDependency: FfmpegDependencySnapshot = {
  status: "checking",
  version: "8.1.2",
  sourceUrl: "https://github.com/GyanD/codexffmpeg/releases/download/8.1.2/ffmpeg-8.1.2-essentials_build.zip",
  license: "GPL-3.0-or-later",
  downloadedBytes: 0,
  totalBytes: null,
  installedBytes: 0,
  error: null,
};

export const emptyCompressionScanSnapshot: CompressionScanSnapshot = {
  scanId: 0,
  status: "idle",
  inputRoots: [],
  scannedEntries: 0,
  candidateFiles: 0,
  validatedFiles: 0,
  readyFiles: 0,
  roots: [],
  warnings: [],
};
