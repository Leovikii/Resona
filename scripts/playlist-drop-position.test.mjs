import assert from "node:assert/strict";
import test from "node:test";
import { playlistDropPosition } from "../src/shared/ui/playlistDropPosition.ts";

test("external insertion gaps have a stable 28px hit area and preserve row centers", () => {
  const gaps = [{ center: 102.5, position: 0 }, { center: 145.5, position: 1 }];
  assert.equal(playlistDropPosition(88.5, gaps), 0);
  assert.equal(playlistDropPosition(116.5, gaps), 0);
  assert.equal(playlistDropPosition(88.4, gaps), null);
  assert.equal(playlistDropPosition(124, gaps), null);
  assert.equal(playlistDropPosition(131.5, gaps), 1);
  assert.equal(playlistDropPosition(159.5, gaps), 1);
  assert.equal(playlistDropPosition(160, gaps), null);
});

test("horizontal, scrolled and empty lists use the same nearest-gap rule", () => {
  assert.equal(playlistDropPosition(200, []), null);
  assert.equal(playlistDropPosition(8, [{ center: 6, position: 8 }]), 8);
  assert.equal(playlistDropPosition(16, [{ center: 6, position: 8 }, { center: 24, position: 9 }]), 9);
});
