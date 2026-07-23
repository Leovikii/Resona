export interface ApplicationUpdateSnapshot {
  currentVersion: string;
  currentIsPrerelease: boolean;
  receivePrereleaseUpdates: boolean;
  updaterConfigured: boolean;
}

export interface ApplicationUpdateRelease {
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
  currentVersion: "0.1.0-rc.1",
  currentIsPrerelease: true,
  receivePrereleaseUpdates: true,
  updaterConfigured: false,
};

