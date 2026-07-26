import assert from "node:assert/strict";
import test from "node:test";

import {
  buildPlaylistTree,
  playlistFolderPaths,
  playlistRootPaths,
  playlistTrackFolderPaths,
  playlistTrackTitle,
  resolvePlaylistFolderState,
} from "../src/shared/ui/playlistTree.ts";

function item(id, path, folderRoot = null) {
  return { id, path, folderRoot };
}

test("folder imports become a recursive tree without changing track order", () => {
  const root = "C:\\Music\\Album";
  const items = [
    item(1, `${root}\\Disc 1\\01.flac`, root),
    item(2, `${root}\\Disc 1\\02.flac`, root),
    item(3, `${root}\\Disc 2\\03.flac`, root),
    item(4, "D:\\Loose.mp3"),
  ];
  const tree = buildPlaylistTree(items);

  assert.equal(tree[0].kind, "folder");
  assert.equal(tree[0].name, "Album");
  assert.equal(tree[0].startPosition, 0);
  assert.equal(tree[0].endPosition, 3);
  assert.deepEqual(tree[0].children.map((node) => node.kind === "folder" ? node.name : node.item.id), [
    "Disc 1",
    "Disc 2",
  ]);
  assert.equal(tree[1].kind, "track");
  assert.equal(tree[1].position, 3);
  assert.deepEqual(playlistRootPaths(items), [root]);
});

test("manual ordering keeps non-contiguous folder segments in sequence", () => {
  const root = "C:\\Music\\Album";
  const tree = buildPlaylistTree([
    item(1, `${root}\\A\\one.flac`, root),
    item(2, "D:\\loose.wav"),
    item(3, `${root}\\A\\two.flac`, root),
  ]);

  assert.deepEqual(tree.map((node) => node.kind === "folder" ? node.startPosition : node.position), [0, 1, 2]);
  assert.equal(tree[0].kind === "folder" && tree[0].endPosition, 1);
  assert.equal(tree[2].kind === "folder" && tree[2].endPosition, 3);
});

test("invalid folder metadata falls back to a flat track", () => {
  const [node] = buildPlaylistTree([
    item(1, "C:\\Music\\track.flac", "D:\\Other"),
  ]);
  assert.equal(node.kind, "track");
  assert.equal(node.position, 0);
});

test("track titles use available tags and always retain a filename fallback", () => {
  const track = item(1, "C:\\Music\\01 - Signal.flac");
  assert.equal(playlistTrackTitle(track, { title: "Signal", trackNumber: 1 }), "1 · Signal");
  assert.equal(playlistTrackTitle(track, { title: null, trackNumber: 1 }), "1 · 01 - Signal");
  assert.equal(playlistTrackTitle(track, undefined), "01 - Signal");
});

test("folder expansion survives remounts while new roots expand once", () => {
  const root = "C:\\Music\\Album";
  const otherRoot = "D:\\Music\\Other";
  const items = [
    item(1, `${root}\\Disc 1\\one.flac`, root),
    item(2, `${otherRoot}\\two.flac`, otherRoot),
  ];
  const tree = buildPlaylistTree(items);
  const roots = playlistRootPaths(items);
  const folders = playlistFolderPaths(tree);
  const initial = resolvePlaylistFolderState(undefined, roots, folders);
  assert.deepEqual(initial.expandedPaths, [root, otherRoot]);

  const collapsed = {
    expandedPaths: [otherRoot],
    seenRootPaths: initial.seenRootPaths,
  };
  assert.deepEqual(resolvePlaylistFolderState(collapsed, roots, folders), collapsed);

  const nextRoot = "E:\\Music\\New";
  const nextItems = [items[0], item(3, `${nextRoot}\\three.flac`, nextRoot)];
  const nextTree = buildPlaylistTree(nextItems);
  assert.deepEqual(
    resolvePlaylistFolderState(collapsed, playlistRootPaths(nextItems), playlistFolderPaths(nextTree)),
    {
      expandedPaths: [nextRoot],
      seenRootPaths: [root, nextRoot],
    },
  );
});

test("locating a nested track returns every ancestor folder", () => {
  const root = "C:\\Music\\Album";
  assert.deepEqual(
    playlistTrackFolderPaths(item(1, `${root}\\Disc 1\\Live\\one.flac`, root)),
    [root, `${root}\\Disc 1`, `${root}\\Disc 1\\Live`],
  );
  assert.deepEqual(playlistTrackFolderPaths(item(2, "C:\\Music\\loose.flac")), []);
});
