import assert from "node:assert/strict";
import test from "node:test";

import {
  desktopLyricsLineTiming,
  desktopLyricsPlaybackNeedsResync,
} from "../src/features/lyrics/desktopLyricsTiming.ts";

const lines = [
  { startMs: 1_000, endMs: null, text: "First" },
  { startMs: 5_000, endMs: null, text: "Second" },
];

test("desktop lyric scrolling follows the active line interval", () => {
  assert.deepEqual(desktopLyricsLineTiming(lines, 0, 2_500, 9_000), {
    delayMs: -1_500,
    durationMs: 4_000,
  });
  assert.deepEqual(desktopLyricsLineTiming(lines, 1, 7_000, 9_000), {
    delayMs: -2_000,
    durationMs: 4_000,
  });
});

test("desktop lyric timing clamps seeks and rejects unknown intervals", () => {
  assert.deepEqual(desktopLyricsLineTiming(lines, 0, 99_000, 9_000), {
    delayMs: -4_000,
    durationMs: 4_000,
  });
  assert.equal(desktopLyricsLineTiming(lines, null, 0, null), null);
  assert.equal(desktopLyricsLineTiming([lines[1]], 0, 5_000, null), null);
});

test("normal polling leaves the running animation clock alone", () => {
  const previous = { observedAtMs: 1_000, positionMs: 2_000, status: "playing" };
  assert.equal(desktopLyricsPlaybackNeedsResync(previous, {
    observedAtMs: 1_250,
    positionMs: 2_250,
    status: "playing",
  }), false);
  assert.equal(desktopLyricsPlaybackNeedsResync(previous, {
    observedAtMs: 1_250,
    positionMs: 6_000,
    status: "playing",
  }), true);
});

test("pausing and seeking while paused resynchronize once", () => {
  assert.equal(desktopLyricsPlaybackNeedsResync(
    { observedAtMs: 1_000, positionMs: 2_000, status: "playing" },
    { observedAtMs: 1_200, positionMs: 2_200, status: "paused" },
  ), true);
  assert.equal(desktopLyricsPlaybackNeedsResync(
    { observedAtMs: 1_200, positionMs: 2_200, status: "paused" },
    { observedAtMs: 2_000, positionMs: 2_200, status: "paused" },
  ), false);
});
