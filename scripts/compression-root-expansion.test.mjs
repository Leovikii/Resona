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

const { indexCompressionAncestors, reconcileCompressionActivityExpansion, toggleCompressionDirectory } =
  await import("../src/app/compressionRootExpansion.ts");
const tree = [{ path: "root", kind: "root", children: [
  { path: "a", kind: "directory", children: [
    { path: "nested", kind: "directory", children: [{ path: "one", kind: "file", children: [] }] },
    { path: "two", kind: "file", children: [] },
  ] },
  { path: "b", kind: "directory", children: [{ path: "three", kind: "file", children: [] }] },
] }];
const ancestors = indexCompressionAncestors(tree);
const activity = () => ({ taskId: 0, seenItems: new Set(), collapsed: new Set() });

test("parallel active files open all ancestors once, without opening unrelated nodes", () => {
  const first = reconcileCompressionActivityExpansion(activity(), 1, ["one", "three"], ancestors);
  assert.deepEqual([...first.pathsToExpand], ["root", "a", "nested", "b"]);
  assert.equal(reconcileCompressionActivityExpansion(first, 1, ["one", "three"], ancestors).pathsToExpand.size, 0);
});

test("manual collapse suppresses later files in the branch, explicit reopen releases descendants", () => {
  const first = reconcileCompressionActivityExpansion(activity(), 1, ["one"], ancestors);
  toggleCompressionDirectory(first, "nested", true, ancestors);
  toggleCompressionDirectory(first, "a", true, ancestors);
  const next = reconcileCompressionActivityExpansion(first, 1, ["two", "three"], ancestors);
  assert.deepEqual([...next.pathsToExpand], ["root", "b"]);
  toggleCompressionDirectory(next, "a", false, ancestors);
  assert.equal(next.collapsed.size, 0);
  assert.deepEqual([...reconcileCompressionActivityExpansion(next, 1, ["one", "two"], ancestors).pathsToExpand], ["root", "a", "nested"]);
});

test("new batches and reopening with running files restore branches without stale suppression", () => {
  const first = reconcileCompressionActivityExpansion(activity(), 1, ["one"], ancestors);
  toggleCompressionDirectory(first, "root", true, ancestors);
  assert.deepEqual([...reconcileCompressionActivityExpansion(first, 2, ["one"], ancestors).pathsToExpand], ["root", "a", "nested"]);
  assert.deepEqual([...reconcileCompressionActivityExpansion(activity(), 2, ["three"], ancestors).pathsToExpand], ["root", "b"]);
  assert.equal(reconcileCompressionActivityExpansion(activity(), 2, [], ancestors).pathsToExpand.size, 0);
});

test("a task snapshot arriving before the scan is retried when ancestors arrive", () => {
  const early = reconcileCompressionActivityExpansion(activity(), 1, ["one"], new Map());
  assert.equal(early.seenItems.size, 0);
  assert.deepEqual([...reconcileCompressionActivityExpansion(early, 1, ["one"], ancestors).pathsToExpand], ["root", "a", "nested"]);
});
