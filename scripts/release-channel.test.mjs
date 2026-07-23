// SPDX-License-Identifier: GPL-3.0-only

import assert from "node:assert/strict";
import test from "node:test";

import { parseReleaseVersion } from "./release-channel.mjs";

test("alpha, beta and rc are all preview releases", () => {
  for (const version of [
    "0.1.0-alpha.1",
    "0.1.0-beta.2",
    "0.1.0-rc.3",
    "0.1.0-preview.windows.1",
  ]) {
    assert.equal(parseReleaseVersion(version).prerelease, true, version);
  }
});

test("a version without a prerelease segment is stable", () => {
  assert.equal(parseReleaseVersion("0.1.0").prerelease, false);
});

test("invalid and ambiguous versions are rejected", () => {
  for (const version of ["v0.1.0", "0.1", "0.1.0-01", "0.1.0-"]) {
    assert.throws(() => parseReleaseVersion(version), undefined, version);
  }
});

