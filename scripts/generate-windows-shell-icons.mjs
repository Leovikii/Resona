// SPDX-License-Identifier: GPL-3.0-only

import {
  copyFileSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const iconRoot = resolve(projectRoot, "src-tauri", "icons");
const iconSets = [
  [
    "assets/resona-icon.svg",
    [
      ["32x32.png", "32x32.png"],
      ["64x64.png", "64x64.png"],
      ["128x128.png", "128x128.png"],
      ["128x128@2x.png", "128x128@2x.png"],
      ["icon.png", "icon.png"],
      ["icon.ico", "icon.ico"],
    ],
  ],
  [
    "assets/resona-default-artwork.svg",
    [["icon.png", "default-artwork.png"]],
  ],
  ["assets/resona-file-mp3.svg", [["icon.ico", "file-mp3.ico"]]],
  ["assets/resona-file-wav.svg", [["icon.ico", "file-wav.ico"]]],
  ["assets/resona-file-flac.svg", [["icon.ico", "file-flac.ico"]]],
  [
    "src-tauri/icons/taskbar-previous.svg",
    [["icon.ico", "taskbar-previous.ico"]],
  ],
  ["src-tauri/icons/taskbar-play.svg", [["icon.ico", "taskbar-play.ico"]]],
  ["src-tauri/icons/taskbar-pause.svg", [["icon.ico", "taskbar-pause.ico"]]],
  ["src-tauri/icons/taskbar-next.svg", [["icon.ico", "taskbar-next.ico"]]],
];
const checkOnly = process.argv.includes("--check");

const tauriCli = resolve(projectRoot, "node_modules", "@tauri-apps", "cli", "tauri.js");
const sourceGenerator = resolve(
  projectRoot,
  "scripts",
  "generate-file-icon-sources.mjs",
);
const sourceGeneration = spawnSync(
  process.execPath,
  [sourceGenerator, ...(checkOnly ? ["--check"] : [])],
  { cwd: projectRoot, stdio: "inherit" },
);
if (sourceGeneration.error) throw sourceGeneration.error;
if (sourceGeneration.status !== 0) {
  throw new Error("File icon SVG generation failed");
}

const generatedOutputs = [];
for (const [sourcePath, outputs] of iconSets) {
  const staging = mkdtempSync(join(tmpdir(), "resona-icon-"));
  try {
    const source = resolve(projectRoot, sourcePath);
    const result = spawnSync(
      process.execPath,
      [tauriCli, "icon", source, "--output", staging],
      { cwd: projectRoot, stdio: "inherit" },
    );
    if (result.error) throw result.error;
    if (result.status !== 0) {
      throw new Error(`Tauri icon generation failed for ${sourcePath}`);
    }
    for (const [generatedName, outputName] of outputs) {
      const generated = join(staging, generatedName);
      const output = join(iconRoot, outputName);
      if (checkOnly) {
        if (!readFileSync(generated).equals(readFileSync(output))) {
          throw new Error(`${outputName} is not reproducible from ${sourcePath}`);
        }
      } else {
        copyFileSync(generated, output);
      }
      generatedOutputs.push(outputName);
    }
  } finally {
    rmSync(staging, { recursive: true, force: true });
  }
}

for (const outputName of generatedOutputs) {
  const output = join(iconRoot, outputName);
  process.stdout.write(`${basename(output)} ${statSync(output).size} bytes\n`);
}
if (checkOnly) process.stdout.write("Windows shell icons are reproducible.\n");
