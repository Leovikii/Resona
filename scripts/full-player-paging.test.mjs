import assert from "node:assert/strict";
import test from "node:test";

import {
  adjacentFullPlayerPage,
  createFullPlayerPagingState,
  reconcileFullPlayerPaging,
  selectFullPlayerPage,
} from "../src/app/fullPlayerPaging.ts";

const input = (overrides = {}) => ({
  compact: false,
  lineCount: 0,
  lyricsStatus: "idle",
  trackKey: "track-a",
  ...overrides,
});

test("resolved lyrics choose the expected wide and compact defaults", () => {
  assert.equal(createFullPlayerPagingState(input({ lyricsStatus: "ready", lineCount: 2 })).page, "lyrics");
  assert.equal(createFullPlayerPagingState(input({ lyricsStatus: "missing" })).page, "details");
  assert.equal(createFullPlayerPagingState(input({ compact: true, lyricsStatus: "missing" })).page, "artwork");
});

test("late lyrics resolution does not replace a manual page selection", () => {
  const initial = createFullPlayerPagingState(input());
  const selected = selectFullPlayerPage(initial, "details");
  const resolved = reconcileFullPlayerPaging(selected, input({ lyricsStatus: "ready", lineCount: 3 }));
  assert.equal(resolved.page, "details");
});

test("a new track receives its own default page", () => {
  const selected = selectFullPlayerPage(
    createFullPlayerPagingState(input({ lyricsStatus: "ready", lineCount: 3 })),
    "details",
  );
  const next = reconcileFullPlayerPaging(selected, input({ compact: true, lyricsStatus: "missing", trackKey: "track-b" }));
  assert.equal(next.page, "artwork");
  assert.equal(next.manuallySelected, false);
});

test("a new track waits for its own lyrics result before applying the no-lyrics default", () => {
  const previous = createFullPlayerPagingState(input({ lyricsStatus: "ready", lineCount: 3 }));
  const loading = reconcileFullPlayerPaging(previous, input({ compact: true, trackKey: "track-b" }));
  assert.equal(loading.page, "lyrics");
  assert.equal(loading.lyricsResolved, false);

  const missing = reconcileFullPlayerPaging(
    loading,
    input({ compact: true, lyricsStatus: "missing", trackKey: "track-b" }),
  );
  assert.equal(missing.page, "artwork");
});

test("page arrows stop at both boundaries", () => {
  const pages = ["artwork", "lyrics", "details"];
  assert.equal(adjacentFullPlayerPage(pages, "artwork", -1), null);
  assert.equal(adjacentFullPlayerPage(pages, "artwork", 1), "lyrics");
  assert.equal(adjacentFullPlayerPage(pages, "details", 1), null);
});
