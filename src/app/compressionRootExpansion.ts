import type { CompressionScanNode } from "../shared/model/compression";

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

export function indexCompressionAncestors(nodes: CompressionScanNode[]) {
  const index = new Map<string, string[]>();
  const visit = (nodes: CompressionScanNode[], ancestors: string[]) => {
    for (const node of nodes) {
      index.set(node.path, ancestors);
      if (node.kind !== "file") visit(node.children, [...ancestors, node.path]);
    }
  };
  visit(nodes, []);
  return index;
}

export interface CompressionActivityExpansion {
  taskId: number;
  seenItems: Set<string>;
  collapsed: Set<string>;
}

export function reconcileCompressionActivityExpansion(
  tracked: CompressionActivityExpansion,
  taskId: number,
  runningSources: string[],
  ancestors: Map<string, string[]>,
) {
  const sameTask = tracked.taskId === taskId;
  const seenItems = sameTask ? new Set(tracked.seenItems) : new Set<string>();
  const collapsed = sameTask ? tracked.collapsed : new Set<string>();
  const pathsToExpand = new Set<string>();
  for (const source of runningSources) {
    if (seenItems.has(source)) continue;
    const parents = ancestors.get(source);
    if (!parents) continue; // The scan snapshot can arrive after the task snapshot.
    seenItems.add(source);
    if (parents.some((path) => collapsed.has(path))) continue;
    parents.forEach((path) => pathsToExpand.add(path));
  }
  return { taskId, seenItems, collapsed, pathsToExpand };
}

export function toggleCompressionDirectory(
  tracked: CompressionActivityExpansion,
  path: string,
  open: boolean,
  ancestors: Map<string, string[]>,
) {
  if (open) {
    tracked.collapsed.add(path);
    return;
  }
  // Explicit reopening also releases manual suppression inside this branch.
  for (const collapsed of tracked.collapsed) {
    if (collapsed === path || ancestors.get(collapsed)?.includes(path)) tracked.collapsed.delete(collapsed);
  }
  for (const source of tracked.seenItems) {
    if (ancestors.get(source)?.includes(path)) tracked.seenItems.delete(source);
  }
}
