import assert from "node:assert/strict";
import test from "node:test";
import { createControlQueue, playbackControlForKey } from "../src/features/playback/playbackShortcuts.ts";

const key = (value, extra = {}) => ({ key: value, altKey: false, ctrlKey: false, shiftKey: false, metaKey: false, isComposing: false, defaultPrevented: false, repeat: false, ...extra });
const tick = () => new Promise((resolve) => setImmediate(resolve));
test("only bare playback keys match; composition, handled keys and repeated Space keep their semantics", () => {
  assert.equal(playbackControlForKey(key(" ")), "toggle");
  assert.equal(playbackControlForKey(key("ArrowUp", { repeat: true })), "volume_up");
  assert.equal(playbackControlForKey(key("ArrowDown")), "volume_down");
  assert.equal(playbackControlForKey(key("ArrowLeft")), "seek_backward");
  assert.equal(playbackControlForKey(key("ArrowRight")), "seek_forward");
  for (const flag of ["altKey", "ctrlKey", "shiftKey", "metaKey", "isComposing", "defaultPrevented"]) assert.equal(playbackControlForKey(key("ArrowUp", { [flag]: true })), null);
  assert.equal(playbackControlForKey(key(" ", { repeat: true })), null);
  assert.equal(playbackControlForKey(key("Enter")), null);
});

test("relative commands serialize and coalesce bounded repeats without stale absolute state", async () => {
  const calls = [], results = [];
  let release;
  const queue = createControlQueue(async (action, steps) => {
    calls.push([action, steps]);
    if (calls.length === 1) await new Promise((resolve) => { release = resolve; });
    return calls.length;
  }, (action, result) => results.push([action, result]), assert.fail);
  queue.push("volume_up", false);
  for (let index = 0; index < 100; index++) queue.push("volume_up", true);
  queue.push("seek_backward", false);
  assert.deepEqual(calls, [["volume_up", 1]]);
  release(); await tick();
  assert.deepEqual(calls, [["volume_up", 1], ["volume_up", 20], ["seek_backward", 1]]);
  assert.equal(results.length, 3);
  queue.dispose();
});

test("key release drops queued repeats; blur clears queued taps; disposal suppresses late feedback", async () => {
  let release;
  const calls = [], results = [];
  const queue = createControlQueue(async (action) => { calls.push(action); await new Promise((resolve) => { release = resolve; }); return 1; }, (action) => results.push(action), assert.fail);
  queue.push("toggle", false);
  queue.push("volume_up", true);
  queue.release("volume_up");
  release(); await tick();
  assert.deepEqual(calls, ["toggle"]);
  queue.push("volume_down", false);
  queue.push("seek_forward", false);
  queue.clear(); queue.dispose(); release(); await tick();
  assert.deepEqual(calls, ["toggle", "volume_down"]);
  assert.deepEqual(results, ["toggle"]);
});

test("a failed command is diagnosed and does not stall the next intent", async () => {
  const errors = [], values = [];
  const queue = createControlQueue(async (action) => { if (action === "toggle") throw new Error("output unavailable"); return 45; }, (_, value) => values.push(value), (error) => errors.push(error.message));
  queue.push("toggle", false); queue.push("volume_up", false); await tick();
  assert.deepEqual(errors, ["output unavailable"]); assert.deepEqual(values, [45]);
  queue.dispose();
});
