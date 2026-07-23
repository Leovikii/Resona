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

export function releaseInspection({ base, root = projectRoot } = {}) {
  const parsed = readProjectVersion(root);
  const previousVersion = previousPackageVersion(base, root);
  const release = previousVersion !== null && previousVersion !== parsed.version;
  if (release) {
    const tag = `v${parsed.version}`;
    const existing = execFileSync(
      "git",
      ["tag", "--list", tag],
      { cwd: root, encoding: "utf8" },
    ).trim();
    if (existing) throw new Error(`Release tag already exists: ${tag}`);
  }
  return {
    ...parsed,
    previousVersion,
    release,
    tag: `v${parsed.version}`,
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
