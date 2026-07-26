import assert from "node:assert/strict";
import test from "node:test";

import {
  adjacentFullPlayerPage,
  createFullPlayerPagingState,
  reconcileFullPlayerPaging,
  selectFullPlayerPage,
} from "../src/app/fullPlayerPaging.ts";

const input = (overrides = {}) => ({
  artworkResolved: false,
  compact: false,
  hasArtwork: false,
  lineCount: 0,
  lyricsStatus: "idle",
  trackKey: "track-a",
  ...overrides,
});

test("resolved lyrics choose the expected wide and compact defaults", () => {
  assert.equal(createFullPlayerPagingState(input({ lyricsStatus: "ready", lineCount: 2 })).page, "lyrics");
  assert.equal(createFullPlayerPagingState(input({ lyricsStatus: "missing" })).page, "details");
  assert.equal(createFullPlayerPagingState(input({
    artworkResolved: true,
    compact: true,
    hasArtwork: true,
    lyricsStatus: "missing",
  })).page, "artwork");
  assert.equal(createFullPlayerPagingState(input({
    artworkResolved: true,
    compact: true,
    lyricsStatus: "missing",
  })).page, "details");
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
  const next = reconcileFullPlayerPaging(selected, input({
    artworkResolved: true,
    compact: true,
    hasArtwork: true,
    lyricsStatus: "missing",
    trackKey: "track-b",
  }));
  assert.equal(next.page, "artwork");
  assert.equal(next.manuallySelected, false);
});

test("a new track waits for its own lyrics result before applying the no-lyrics default", () => {
  const previous = selectFullPlayerPage(
    createFullPlayerPagingState(input({ lyricsStatus: "ready", lineCount: 3 })),
    "details",
  );
  const loading = reconcileFullPlayerPaging(previous, input({ compact: true, trackKey: "track-b" }));
  assert.equal(loading.page, "details");
  assert.equal(loading.lyricsResolved, false);

  const missing = reconcileFullPlayerPaging(
    loading,
    input({
      artworkResolved: true,
      compact: true,
      hasArtwork: true,
      lyricsStatus: "missing",
      trackKey: "track-b",
    }),
  );
  assert.equal(missing.page, "artwork");
});

test("an unresolved track change preserves the current available page", () => {
  const previous = selectFullPlayerPage(createFullPlayerPagingState(input()), "details");
  const loading = reconcileFullPlayerPaging(previous, input({ trackKey: "track-b" }));
  assert.equal(loading.page, "details");
  assert.equal(loading.manuallySelected, false);

  const ready = reconcileFullPlayerPaging(
    loading,
    input({ lyricsStatus: "ready", lineCount: 3, trackKey: "track-b" }),
  );
  assert.equal(ready.page, "lyrics");
});

test("compact playback falls back to details when lyrics and artwork are both absent", () => {
  const previous = selectFullPlayerPage(createFullPlayerPagingState(input()), "details");
  const loading = reconcileFullPlayerPaging(
    previous,
    input({ compact: true, trackKey: "track-b" }),
  );
  assert.equal(loading.page, "details");

  const resolved = reconcileFullPlayerPaging(
    loading,
    input({
      artworkResolved: true,
      compact: true,
      lyricsStatus: "missing",
      trackKey: "track-b",
    }),
  );
  assert.equal(resolved.page, "details");
  assert.equal(resolved.defaultResolved, true);
});

test("page arrows stop at both boundaries", () => {
  const pages = ["artwork", "lyrics", "details"];
  assert.equal(adjacentFullPlayerPage(pages, "artwork", -1), null);
  assert.equal(adjacentFullPlayerPage(pages, "artwork", 1), "lyrics");
  assert.equal(adjacentFullPlayerPage(pages, "details", 1), null);
});
