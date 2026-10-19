import assert from "node:assert/strict";
import test from "node:test";
import { compressionProgress } from "../src/features/compression/compressionProgress.ts";
const batch = (status, entries) => ({ taskId: 1, status, total: entries.length, completed: 0,
  items: entries.map(([status, progress]) => ({ status, progress, sourceBytes: 100, outputBytes: 60 })) });

test("empty and parallel batches use bounded file-weighted processing progress", () => {
  assert.equal(compressionProgress(batch("idle", [])).percent, 0);
  const result = compressionProgress(batch("running", [["completed", 1], ["running", .5], ["running", .25], ["pending", 0]]));
  assert.equal(result.percent, 44);
  assert.equal(result.running, 2);
  assert.equal(result.succeeded, 1);
  assert.equal(result.sourceBytes, 100);
  assert.equal(result.outputBytes, 60);
});

test("failed files count as processed but never successful savings", () => {
  const result = compressionProgress(batch("completed_with_errors", [["completed", 1], ["failed", .1]]));
  assert.equal(result.percent, 100);
  assert.equal(result.failed, 1);
  assert.equal(result.succeeded, 1);
  assert.equal(result.sourceBytes - result.outputBytes, 40);
});

test("cancellation retains actual progress instead of filling unfinished files", () => {
  const result = compressionProgress(batch("cancelled", [["completed", 1], ["failed", .1], ["cancelled", .4], ["cancelled", 0]]));
  assert.equal(result.percent, 60);
  assert.equal(result.cancelled, 2);
  assert.equal(result.failed, 1);
});

test("only Rust terminal state permits full progress and every new batch starts fresh", () => {
  for (const status of ["running", "cancelling"]) assert.equal(compressionProgress(batch(status, [["running", 1]])).percent, 99);
  assert.equal(compressionProgress(batch("completed", [["completed", 1]])).percent, 100);
  assert.equal(compressionProgress(batch("running", [["pending", 0]])).percent, 0);
  assert.equal(compressionProgress(batch("running", [["running", NaN], ["running", -1], ["running", 5]])).percent, 33);
});
