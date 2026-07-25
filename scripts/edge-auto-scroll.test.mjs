import assert from "node:assert/strict";
import test from "node:test";

import { edgeScrollDelta } from "../src/shared/ui/edgeAutoScroll.ts";

test("edge scrolling stays idle away from both edges", () => {
  assert.equal(edgeScrollDelta(100, 300, 200), 0);
});

test("edge scrolling moves toward the nearest edge", () => {
  assert.ok(edgeScrollDelta(100, 300, 110) < 0);
  assert.ok(edgeScrollDelta(100, 300, 290) > 0);
});

test("edge scrolling caps its speed outside the viewport", () => {
  assert.equal(edgeScrollDelta(100, 300, 0), -12);
  assert.equal(edgeScrollDelta(100, 300, 400), 12);
});

test("edge scrolling handles tiny and invalid viewports", () => {
  assert.equal(edgeScrollDelta(10, 10, 10), 0);
  assert.equal(edgeScrollDelta(10, 20, 10), -12);
  assert.equal(edgeScrollDelta(10, 20, 20), 12);
});
