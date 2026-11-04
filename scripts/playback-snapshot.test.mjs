import assert from "node:assert/strict";
import test from "node:test";
import { mergePlaybackSnapshot, previewSnapshot } from "../src/shared/model/playback.ts";

test("unchanged playback snapshots preserve stable queue and output references", () => {
  const previous = previewSnapshot();
  assert.equal(mergePlaybackSnapshot(previous, structuredClone(previous)), previous);
  const next = mergePlaybackSnapshot(previous, { ...structuredClone(previous), positionMs: 100 });
  assert.equal(next.positionMs, 100);
  assert.equal(next.queue, previous.queue);
  assert.equal(next.output, previous.output);
});

test("each active output format field reaches the player without another state change", () => {
  for (const [field, value] of Object.entries({ activeSampleRate: 96_000, activeChannelCount: 6, activeSampleFormat: "i24" })) {
    const previous = previewSnapshot();
    const next = structuredClone(previous);
    next.output[field] = value;
    assert.equal(mergePlaybackSnapshot(previous, next).output[field], value, field);
  }
});

test("CUE metadata and boundaries update even when the physical file is unchanged", () => {
  const previous = previewSnapshot();
  previous.queue[0].cue = { cuePath: "album.cue", trackNumber: 1, title: "First", performer: "Artist", album: "Album", startMs: 0, endMs: 194_000 };
  for (const [field, value] of Object.entries({ cuePath: "other.cue", trackNumber: 2, title: "Second", performer: "Guest", album: "Other", startMs: 1000, endMs: null })) {
    const next = structuredClone(previous);
    next.queue[0].cue[field] = value;
    assert.equal(mergePlaybackSnapshot(previous, next).queue[0].cue[field], value, field);
  }
  const next = structuredClone(previous);
  next.queue[0].cue = null;
  assert.equal(mergePlaybackSnapshot(previous, next).queue[0].cue, null);
});
