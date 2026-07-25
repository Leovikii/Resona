import assert from "node:assert/strict";
import test from "node:test";

import { reconcileCompressionRootExpansion } from "../src/app/compressionRootExpansion.ts";

test("roots that arrive after a scan starts expand once", () => {
  const first = reconcileCompressionRootExpansion(0, new Set(), 1, []);
  const arrived = reconcileCompressionRootExpansion(
    first.scanId,
    first.seenRoots,
    1,
    ["C:\\Music"],
  );
  assert.deepEqual(arrived.pathsToExpand, ["C:\\Music"]);
});

test("polling does not reopen a root after manual collapse", () => {
  const result = reconcileCompressionRootExpansion(
    1,
    new Set(["C:\\Music"]),
    1,
    ["C:\\Music"],
  );
  assert.deepEqual(result.pathsToExpand, []);
  assert.equal(result.reset, false);
});

test("new roots expand once and a new scan clears the previous expansion", () => {
  const added = reconcileCompressionRootExpansion(
    1,
    new Set(["C:\\Music"]),
    1,
    ["C:\\Music", "D:\\Audio"],
  );
  assert.deepEqual(added.pathsToExpand, ["D:\\Audio"]);

  const nextScan = reconcileCompressionRootExpansion(
    added.scanId,
    added.seenRoots,
    2,
    ["E:\\Library"],
  );
  assert.deepEqual(nextScan.pathsToExpand, ["E:\\Library"]);
  assert.equal(nextScan.reset, true);
});
