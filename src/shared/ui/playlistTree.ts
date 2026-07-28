export interface PlaylistTreeItem {
  id: number;
  path: string;
  folderRoot: string | null;
  displayName?: string;
  cue?: {
    trackNumber: number;
    title: string | null;
    startMs: number;
    endMs: number | null;
  } | null;
}

export interface PlaylistTrackSummary {
  title: string | null;
  trackNumber: number | null;
  durationMs: number | null;
}

export interface PlaylistDurationSummary {
  complete: boolean;
  durationMs: number;
}

export interface PlaylistFolderState {
  expandedPaths: string[];
  seenRootPaths: string[];
}

export interface PlaylistTrackNode<T extends PlaylistTreeItem> {
  kind: "track";
  item: T;
  position: number;
}

export interface PlaylistFolderNode<T extends PlaylistTreeItem> {
  kind: "folder";
  id: string;
  path: string;
  name: string;
  startPosition: number;
  endPosition: number;
  itemCount: number;
  children: PlaylistTreeNode<T>[];
}

export type PlaylistTreeNode<T extends PlaylistTreeItem> =
  | PlaylistTrackNode<T>
  | PlaylistFolderNode<T>;

interface PreparedItem<T extends PlaylistTreeItem> {
  item: T;
  position: number;
  folders: FolderPath[];
}

interface FolderPath {
  path: string;
  name: string;
}

export function buildPlaylistTree<T extends PlaylistTreeItem>(items: T[]): PlaylistTreeNode<T>[] {
  const prepared = items.map((item, position) => ({
    item,
    position,
    folders: folderAncestry(item.path, item.folderRoot),
  }));
  return buildRange(prepared, 0, prepared.length, 0);
}

export function playlistRootPaths(items: PlaylistTreeItem[]): string[] {
  const roots = new Map<string, string>();
  for (const item of items) {
    const root = folderAncestry(item.path, item.folderRoot)[0]?.path;
    if (root) roots.set(pathKey(root), root);
  }
  return [...roots.values()];
}

export function playlistFolderPaths<T extends PlaylistTreeItem>(nodes: PlaylistTreeNode<T>[]): string[] {
  const paths: string[] = [];
  for (const node of nodes) {
    if (node.kind !== "folder") continue;
    paths.push(node.path, ...playlistFolderPaths(node.children));
  }
  return paths;
}

export function playlistTrackFolderPaths(item: PlaylistTreeItem): string[] {
  return folderAncestry(item.path, item.folderRoot).map((folder) => folder.path);
}

export function resolvePlaylistFolderState(
  state: PlaylistFolderState | undefined,
  rootPaths: string[],
  folderPaths: string[],
): PlaylistFolderState {
  const validFolders = new Map(folderPaths.map((path) => [pathKey(path), path]));
  const validRoots = new Map(rootPaths.map((path) => [pathKey(path), path]));
  const expanded = new Set(
    (state?.expandedPaths ?? [])
      .map((path) => validFolders.get(pathKey(path)))
      .filter((path): path is string => Boolean(path)),
  );
  const seenRoots = new Set(
    (state?.seenRootPaths ?? [])
      .map((path) => validRoots.get(pathKey(path)))
      .filter((path): path is string => Boolean(path)),
  );
  for (const root of rootPaths) {
    if (seenRoots.has(root)) continue;
    seenRoots.add(root);
    expanded.add(root);
  }
  return {
    expandedPaths: folderPaths.filter((path) => expanded.has(path)),
    seenRootPaths: rootPaths.filter((path) => seenRoots.has(path)),
  };
}

export function playlistTrackTitle(
  item: Pick<PlaylistTreeItem, "path" | "displayName" | "cue">,
  summary: PlaylistTrackSummary | undefined,
) {
  const fileName = baseName(item.path);
  const fallback = fileName.replace(/\.[^.]+$/, "") || fileName;
  const title = item.cue?.title?.trim()
    || (item.cue ? item.displayName?.replace(/\.[^.]+$/, "") : null)
    || summary?.title?.trim()
    || fallback;
  const trackNumber = item.cue?.trackNumber ?? summary?.trackNumber;
  return trackNumber ? `${trackNumber} · ${title}` : title;
}

export function playlistTrackDuration(
  item: Pick<PlaylistTreeItem, "cue">,
  summary: PlaylistTrackSummary | undefined,
): number | null {
  const physicalDuration = summary?.durationMs ?? null;
  if (!item.cue) return physicalDuration;
  const startMs = item.cue.startMs;
  const endMs = item.cue.endMs ?? physicalDuration;
  return endMs === null ? null : Math.max(0, endMs - startMs);
}

export function playlistDurationSummary<T extends PlaylistTreeItem>(
  items: T[],
  summaries: Map<string, PlaylistTrackSummary>,
  startPosition = 0,
  endPosition = items.length,
): PlaylistDurationSummary {
  let durationMs = 0;
  let complete = true;
  for (let position = startPosition; position < endPosition; position += 1) {
    const item = items[position];
    const duration = playlistTrackDuration(item, summaries.get(item.path));
    if (duration === null) complete = false;
    else durationMs += duration;
  }
  return { complete, durationMs };
}

function buildRange<T extends PlaylistTreeItem>(
  items: PreparedItem<T>[],
  start: number,
  end: number,
  level: number,
): PlaylistTreeNode<T>[] {
  const nodes: PlaylistTreeNode<T>[] = [];
  let index = start;
  while (index < end) {
    const prepared = items[index];
    const folder = prepared.folders[level];
    if (!folder) {
      nodes.push({ kind: "track", item: prepared.item, position: prepared.position });
      index += 1;
      continue;
    }

    let next = index + 1;
    while (next < end && samePath(items[next].folders[level]?.path, folder.path)) next += 1;
    nodes.push({
      kind: "folder",
      id: `${pathKey(folder.path)}\0${prepared.item.id}`,
      path: folder.path,
      name: folder.name,
      startPosition: prepared.position,
      endPosition: items[next - 1].position + 1,
      itemCount: next - index,
      children: buildRange(items, index, next, level + 1),
    });
    index = next;
  }
  return nodes;
}

function folderAncestry(path: string, folderRoot: string | null): FolderPath[] {
  if (!folderRoot) return [];
  const root = trimTrailingSeparators(folderRoot);
  const parent = parentPath(path);
  if (!root || !parent || !isWithin(parent, root)) return [];

  const separator = root.includes("\\") ? "\\" : "/";
  const relative = parent.slice(root.length).replace(/^[\\/]+/, "");
  const folders: FolderPath[] = [{ path: root, name: baseName(root) }];
  let current = root;
  for (const part of relative.split(/[\\/]+/).filter(Boolean)) {
    current = `${current}${separator}${part}`;
    folders.push({ path: current, name: part });
  }
  return folders;
}

function parentPath(path: string) {
  const index = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  return index > 0 ? trimTrailingSeparators(path.slice(0, index)) : "";
}

function baseName(path: string) {
  const parts = path.split(/[\\/]+/).filter(Boolean);
  return parts.at(-1) ?? path;
}

function trimTrailingSeparators(path: string) {
  return path.replace(/[\\/]+$/, "");
}

function isWithin(path: string, root: string) {
  const candidate = pathKey(path);
  const parent = pathKey(root);
  return candidate === parent
    || candidate.startsWith(`${parent}\\`)
    || candidate.startsWith(`${parent}/`);
}

function samePath(left: string | undefined, right: string) {
  return left !== undefined && pathKey(left) === pathKey(right);
}

function pathKey(path: string) {
  return path.replace(/\//g, "\\").toLowerCase();
}
