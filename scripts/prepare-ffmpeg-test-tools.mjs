// SPDX-License-Identifier: GPL-3.0-only

import {
  copyFileSync,
  createReadStream,
  createWriteStream,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  rmSync,
} from "node:fs";
import { createHash } from "node:crypto";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { pipeline } from "node:stream/promises";
import { Readable } from "node:stream";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const archiveUrl =
  "https://github.com/GyanD/codexffmpeg/releases/download/8.1.2/ffmpeg-8.1.2-essentials_build.zip";
const expected = {
  archive: "DB580001CAA24AC104C8CB856CD113A87B0A443F7BDF47D8C12B1D740584A2EC",
  ffmpeg: "1326DDE4C84FF1F96FE6B8916C5BED29E163E9B5DCCF995F6F3DB069D143EC5E",
  ffprobe: "B49CCC7C6547B141AD5A2F6EC69CC04323D7133D7704D70B331B904C63EECB07",
};
if (process.platform !== "win32" || process.arch !== "x64") {
  throw new Error(
    `No audited FFmpeg test-tool asset is configured for ${process.platform}-${process.arch}`,
  );
}

const projectRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const binaryRoot = join(projectRoot, "src-tauri", "binaries");
const targets = {
  ffmpeg: join(binaryRoot, "ffmpeg-x86_64-pc-windows-msvc.exe"),
  ffprobe: join(binaryRoot, "ffprobe-x86_64-pc-windows-msvc.exe"),
};
const force = process.argv.includes("--force");

async function sha256(path) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return hash.digest("hex").toUpperCase();
}

async function assertHash(path, expectedHash, label) {
  const actual = await sha256(path);
  if (actual !== expectedHash) {
    throw new Error(`${label} SHA-256 mismatch: expected ${expectedHash}, received ${actual}`);
  }
}

function findFile(directory, fileName) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      const nested = findFile(path, fileName);
      if (nested) return nested;
    } else if (entry.name.toLowerCase() === fileName) {
      return path;
    }
  }
  return null;
}

if (
  !force
  && existsSync(targets.ffmpeg)
  && existsSync(targets.ffprobe)
) {
  await assertHash(targets.ffmpeg, expected.ffmpeg, "ffmpeg");
  await assertHash(targets.ffprobe, expected.ffprobe, "ffprobe");
  process.stdout.write("FFmpeg test tools are present and verified.\n");
  process.exit(0);
}

const temporary = mkdtempSync(join(tmpdir(), "resona-ffmpeg-"));
try {
  const archive = join(temporary, "ffmpeg.zip");
  const extracted = join(temporary, "extract");
  mkdirSync(extracted);

  process.stdout.write("Downloading pinned FFmpeg 8.1.2 essentials build...\n");
  const response = await fetch(archiveUrl);
  if (!response.ok || !response.body) {
    throw new Error(`FFmpeg download failed with HTTP ${response.status}`);
  }
  await pipeline(Readable.fromWeb(response.body), createWriteStream(archive));
  await assertHash(archive, expected.archive, "FFmpeg archive");

  const extraction = spawnSync("tar", ["-xf", archive, "-C", extracted], {
    stdio: "inherit",
  });
  if (extraction.error) throw extraction.error;
  if (extraction.status !== 0) throw new Error("Archive extraction failed");

  const ffmpeg = findFile(extracted, "ffmpeg.exe");
  const ffprobe = findFile(extracted, "ffprobe.exe");
  if (!ffmpeg || !ffprobe) {
    throw new Error("Verified archive does not contain ffmpeg.exe and ffprobe.exe");
  }
  await assertHash(ffmpeg, expected.ffmpeg, "ffmpeg");
  await assertHash(ffprobe, expected.ffprobe, "ffprobe");

  mkdirSync(binaryRoot, { recursive: true });
  copyFileSync(ffmpeg, targets.ffmpeg);
  copyFileSync(ffprobe, targets.ffprobe);
  await assertHash(targets.ffmpeg, expected.ffmpeg, "installed ffmpeg");
  await assertHash(targets.ffprobe, expected.ffprobe, "installed ffprobe");
  process.stdout.write("FFmpeg test tools downloaded and verified.\n");
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
