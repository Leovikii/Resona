// SPDX-License-Identifier: GPL-3.0-only

import { appendFileSync, readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const semverPattern =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+[0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*)?$/;

export function parseReleaseVersion(value) {
  const match = semverPattern.exec(value);
  if (!match) throw new Error(`Invalid SemVer release version: ${value}`);
  return {
    version: value,
    prerelease: match[4] !== undefined,
    prereleaseIdentifiers: match[4]?.split(".") ?? [],
  };
}

function compareNumericIdentifier(left, right) {
  if (left.length !== right.length) return left.length < right.length ? -1 : 1;
  if (left === right) return 0;
  return left < right ? -1 : 1;
}

export function compareReleaseVersions(left, right) {
  const leftMatch = semverPattern.exec(left);
  const rightMatch = semverPattern.exec(right);
  if (!leftMatch) throw new Error(`Invalid SemVer release version: ${left}`);
  if (!rightMatch) throw new Error(`Invalid SemVer release version: ${right}`);

  for (let index = 1; index <= 3; index += 1) {
    const comparison = compareNumericIdentifier(leftMatch[index], rightMatch[index]);
    if (comparison !== 0) return comparison;
  }

  const leftPrerelease = leftMatch[4]?.split(".") ?? [];
  const rightPrerelease = rightMatch[4]?.split(".") ?? [];
  if (leftPrerelease.length === 0 || rightPrerelease.length === 0) {
    if (leftPrerelease.length === rightPrerelease.length) return 0;
    return leftPrerelease.length === 0 ? 1 : -1;
  }

  const identifierCount = Math.max(leftPrerelease.length, rightPrerelease.length);
  for (let index = 0; index < identifierCount; index += 1) {
    const leftIdentifier = leftPrerelease[index];
    const rightIdentifier = rightPrerelease[index];
    if (leftIdentifier === undefined) return -1;
    if (rightIdentifier === undefined) return 1;
    if (leftIdentifier === rightIdentifier) continue;

    const leftIsNumeric = /^\d+$/.test(leftIdentifier);
    const rightIsNumeric = /^\d+$/.test(rightIdentifier);
    if (leftIsNumeric && rightIsNumeric) {
      return compareNumericIdentifier(leftIdentifier, rightIdentifier);
    }
    if (leftIsNumeric !== rightIsNumeric) return leftIsNumeric ? -1 : 1;
    return leftIdentifier < rightIdentifier ? -1 : 1;
  }
  return 0;
}

export function readProjectVersion(root = projectRoot) {
  const packageVersion = JSON.parse(
    readFileSync(resolve(root, "package.json"), "utf8"),
  ).version;
  const tauriVersion = JSON.parse(
    readFileSync(resolve(root, "src-tauri", "tauri.conf.json"), "utf8"),
  ).version;
  const cargoText = readFileSync(resolve(root, "src-tauri", "Cargo.toml"), "utf8");
  const cargoVersion = /^version\s*=\s*"([^"]+)"/m.exec(cargoText)?.[1];
  if (!cargoVersion) throw new Error("Cargo package version was not found");
  const versions = [packageVersion, tauriVersion, cargoVersion];
  if (new Set(versions).size !== 1) {
    throw new Error(`Package, Tauri and Cargo versions differ: ${versions.join(", ")}`);
  }
  return parseReleaseVersion(packageVersion);
}

export function previousPackageVersion(base, root = projectRoot) {
  if (!base) return null;
  try {
    const contents = execFileSync(
      "git",
      ["show", `${base}:package.json`],
      { cwd: root, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
    );
    return JSON.parse(contents).version ?? null;
  } catch (error) {
    throw new Error(`Could not read package.json from base commit ${base}`, {
      cause: error,
    });
  }
}

export function shouldCreateRelease({ currentVersion, previousVersion, tagExists }) {
  if (previousVersion === null) return false;
  const comparison = compareReleaseVersions(currentVersion, previousVersion);
  if (comparison < 0) {
    throw new Error(
      `Release version must not move backwards: ${previousVersion} -> ${currentVersion}`,
    );
  }
  if (comparison === 0 && currentVersion !== previousVersion) {
    throw new Error(
      `Release version must advance in SemVer precedence: ${previousVersion} -> ${currentVersion}`,
    );
  }
  if (previousVersion !== currentVersion && tagExists) {
    throw new Error(`Release tag already exists: v${currentVersion}`);
  }
  return !tagExists;
}

export function releaseInspection({ base, root = projectRoot } = {}) {
  const parsed = readProjectVersion(root);
  const previousVersion = previousPackageVersion(base, root);
  const tag = `v${parsed.version}`;
  const tagExists = execFileSync(
    "git",
    ["tag", "--list", tag],
    { cwd: root, encoding: "utf8" },
  ).trim().length > 0;
  const release = shouldCreateRelease({
    currentVersion: parsed.version,
    previousVersion,
    tagExists,
  });
  return {
    ...parsed,
    previousVersion,
    release,
    tag,
  };
}

function writeGithubOutputs(result) {
  const output = process.env.GITHUB_OUTPUT;
  if (!output) return;
  appendFileSync(
    output,
    [
      `release=${result.release}`,
      `version=${result.version}`,
      `tag=${result.tag}`,
      `prerelease=${result.prerelease}`,
      "",
    ].join("\n"),
  );
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const baseIndex = process.argv.indexOf("--base");
  const base = baseIndex >= 0 ? process.argv[baseIndex + 1] : null;
  const result = releaseInspection({ base });
  writeGithubOutputs(result);
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
}
