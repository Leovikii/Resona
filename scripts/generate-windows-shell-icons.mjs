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
const sources = [
  ["assets/resona-file-mp3.svg", "file-mp3.ico"],
  ["assets/resona-file-wav.svg", "file-wav.ico"],
  ["assets/resona-file-flac.svg", "file-flac.ico"],
  ["src-tauri/icons/taskbar-previous.svg", "taskbar-previous.ico"],
  ["src-tauri/icons/taskbar-play.svg", "taskbar-play.ico"],
  ["src-tauri/icons/taskbar-pause.svg", "taskbar-pause.ico"],
  ["src-tauri/icons/taskbar-next.svg", "taskbar-next.ico"],
];
const checkOnly = process.argv.includes("--check");

const tauriCli = resolve(projectRoot, "node_modules", "@tauri-apps", "cli", "tauri.js");
for (const [sourcePath, outputName] of sources) {
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
    const generated = join(staging, "icon.ico");
    const output = join(iconRoot, outputName);
    if (checkOnly) {
      if (!readFileSync(generated).equals(readFileSync(output))) {
        throw new Error(`${outputName} is not reproducible from ${sourcePath}`);
      }
    } else {
      copyFileSync(generated, output);
    }
  } finally {
    rmSync(staging, { recursive: true, force: true });
  }
}

for (const [, outputName] of sources) {
  const output = join(iconRoot, outputName);
  process.stdout.write(`${basename(output)} ${statSync(output).size} bytes\n`);
}
if (checkOnly) process.stdout.write("Windows shell icons are reproducible.\n");
