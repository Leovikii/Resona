import assert from "node:assert/strict";
import test from "node:test";

import {
  normalizeSteppedPreference,
  parseNumberPreference,
} from "../src/app/preferenceValue.ts";

test("missing and blank numeric preferences use their defaults", () => {
  assert.equal(parseNumberPreference(null, 30), 30);
  assert.equal(parseNumberPreference("", 100), 100);
  assert.equal(parseNumberPreference("   ", 20), 20);
});

test("stored numeric preferences remain authoritative", () => {
  assert.equal(parseNumberPreference("28", 30), 28);
  assert.equal(parseNumberPreference("0", 20), 0);
});

test("invalid numeric preferences use their defaults", () => {
  assert.equal(parseNumberPreference("not-a-number", 30), 30);
});

test("stepped percentage preferences snap and clamp to their supported range", () => {
  assert.equal(normalizeSteppedPreference(14, 10, 100, 10, 100), 10);
  assert.equal(normalizeSteppedPreference(86, 10, 100, 10, 100), 90);
  assert.equal(normalizeSteppedPreference(-20, 0, 90, 10, 20), 0);
  assert.equal(normalizeSteppedPreference(100, 0, 90, 10, 20), 90);
  assert.equal(normalizeSteppedPreference(Number.NaN, 0, 90, 10, 20), 20);
});
