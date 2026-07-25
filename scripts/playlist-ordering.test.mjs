import assert from "node:assert/strict";
import test from "node:test";

import { movePlaylistSummary } from "../src/features/library/playlistOrdering.ts";

const playlists = [
  { id: 10, name: "First", position: 0 },
  { id: 20, name: "Second", position: 1 },
  { id: 30, name: "Third", position: 2 },
];

test("playlist navigation moves an item and keeps positions dense", () => {
  const moved = movePlaylistSummary(playlists, 30, 0);
  assert.deepEqual(moved.map(({ id, position }) => ({ id, position })), [
    { id: 30, position: 0 },
    { id: 10, position: 1 },
    { id: 20, position: 2 },
  ]);
});

test("playlist navigation clamps positions at both boundaries", () => {
  assert.deepEqual(movePlaylistSummary(playlists, 10, 99).map((playlist) => playlist.id), [20, 30, 10]);
  assert.deepEqual(movePlaylistSummary(playlists, 30, -9).map((playlist) => playlist.id), [30, 10, 20]);
});

test("playlist navigation preserves identity for missing and no-op moves", () => {
  assert.equal(movePlaylistSummary(playlists, 404, 1), playlists);
  assert.equal(movePlaylistSummary(playlists, 20, 1), playlists);
});
