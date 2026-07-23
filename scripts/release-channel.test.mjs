// SPDX-License-Identifier: GPL-3.0-only

import assert from "node:assert/strict";
import test from "node:test";

import {
  compareReleaseVersions,
  parseReleaseVersion,
  shouldCreateRelease,
} from "./release-channel.mjs";

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

test("SemVer precedence covers preview stages, numeric identifiers and stable releases", () => {
  const ascending = [
    "0.1.0-alpha.1",
    "0.1.0-alpha.2",
    "0.1.0-alpha.10",
    "0.1.0-beta.1",
    "0.1.0-rc.1",
    "0.1.0",
    "0.2.0-alpha.1",
  ];
  for (let index = 1; index < ascending.length; index += 1) {
    assert.equal(compareReleaseVersions(ascending[index - 1], ascending[index]), -1);
    assert.equal(compareReleaseVersions(ascending[index], ascending[index - 1]), 1);
  }
  assert.equal(compareReleaseVersions("0.1.0+build.1", "0.1.0+build.2"), 0);
});

test("an untagged current version can bootstrap its first release", () => {
  assert.equal(
    shouldCreateRelease({
      currentVersion: "0.1.0-rc.1",
      previousVersion: "0.1.0-rc.1",
      tagExists: false,
    }),
    true,
  );
});

test("an existing current-version tag prevents a duplicate release", () => {
  assert.equal(
    shouldCreateRelease({
      currentVersion: "0.1.0-rc.1",
      previousVersion: "0.1.0-rc.1",
      tagExists: true,
    }),
    false,
  );
});

test("a changed version cannot reuse an existing tag", () => {
  assert.throws(
    () =>
      shouldCreateRelease({
        currentVersion: "0.1.0",
        previousVersion: "0.1.0-rc.1",
        tagExists: true,
      }),
    /Release tag already exists/,
  );
});

test("a release version cannot move backwards or change only build metadata", () => {
  assert.throws(
    () =>
      shouldCreateRelease({
        currentVersion: "0.1.0-beta.9",
        previousVersion: "0.1.0-rc.1",
        tagExists: false,
      }),
    /must not move backwards/,
  );
  assert.throws(
    () =>
      shouldCreateRelease({
        currentVersion: "0.1.0+build.2",
        previousVersion: "0.1.0+build.1",
        tagExists: false,
      }),
    /must advance in SemVer precedence/,
  );
});
