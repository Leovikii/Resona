export interface CompressionRootExpansionResult {
  pathsToExpand: string[];
  reset: boolean;
  scanId: number;
  seenRoots: Set<string>;
}

export function reconcileCompressionRootExpansion(
  trackedScanId: number,
  seenRoots: Set<string>,
  scanId: number,
  rootPaths: string[],
): CompressionRootExpansionResult {
  const scanChanged = trackedScanId !== scanId;
  const nextSeenRoots = scanChanged ? new Set<string>() : new Set(seenRoots);
  const pathsToExpand: string[] = [];

  for (const path of rootPaths) {
    if (nextSeenRoots.has(path)) continue;
    nextSeenRoots.add(path);
    pathsToExpand.push(path);
  }

  return { pathsToExpand, reset: scanChanged, scanId, seenRoots: nextSeenRoots };
}
