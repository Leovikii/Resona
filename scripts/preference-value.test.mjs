import assert from "node:assert/strict";
import test from "node:test";

import { parseNumberPreference } from "../src/app/preferenceValue.ts";

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
