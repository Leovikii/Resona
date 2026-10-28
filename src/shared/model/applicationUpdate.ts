import packageMetadata from "../../../package.json";

export const applicationVersion = packageMetadata.version;
export const applicationIsPrerelease = applicationVersion
  .split("+", 1)[0]
  .includes("-");

export interface ApplicationUpdateSnapshot {
  distribution: "nsis" | "pacman";
  stage: { phase: string; changes: string[] };
  supported: boolean;
  currentVersion: string;
  currentBuild: string | null;
  currentIsPrerelease: boolean;
  receivePrereleaseUpdates: boolean;
  updaterConfigured: boolean;
}

export interface ApplicationUpdateRelease {
  tag: string;
  version: string;
  title: string;
  notes: string;
  publishedAt: string | null;
  releaseUrl: string;
  installerSize: number | null;
  prerelease: boolean;
}

export interface ApplicationUpdateCheckResult {
  currentVersion: string;
  update: ApplicationUpdateRelease | null;
}

export interface ApplicationUpdateProgress {
  downloadedBytes: number;
  totalBytes: number | null;
}

export interface ApplicationUpdateFailure {
  code: string;
  message: string;
}

export const defaultApplicationUpdateSnapshot: ApplicationUpdateSnapshot = {
  distribution: "nsis",
  stage: { phase: "", changes: [] },
  supported: false,
  currentVersion: applicationVersion,
  currentBuild: null,
  currentIsPrerelease: applicationIsPrerelease,
  receivePrereleaseUpdates: true,
  updaterConfigured: false,
};
