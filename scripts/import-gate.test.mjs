import assert from "node:assert/strict";
import test from "node:test";
import { createImportGate } from "../src/shared/bridge/importGate.ts";

test("an import stays locked through scan and commit, rejects duplicates, then releases", async () => {
  const states = []; let finish; let calls = 0;
  const run = createImportGate((active) => states.push(active));
  const first = run(async () => { calls++; await new Promise((resolve) => { finish = resolve; }); return 14; });
  assert.equal(await run(async () => { calls++; return 2; }), null);
  assert.equal(calls, 1);
  assert.deepEqual(states, [true]);
  finish(); assert.equal(await first, 14);
  assert.equal(await run(async () => 2), 2);
  assert.deepEqual(states, [true, false, true, false]);
});

test("failed imports release the UI gate without hiding the failure", async () => {
  const states = []; const run = createImportGate((active) => states.push(active));
  await assert.rejects(run(async () => { throw new Error("unavailable"); }), /unavailable/);
  assert.equal(await run(async () => 1), 1);
  assert.deepEqual(states, [true, false, true, false]);
});
