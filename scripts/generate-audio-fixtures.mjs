// SPDX-License-Identifier: GPL-3.0-only

import {
  mkdirSync,
  readFileSync,
  readdirSync,
  writeFileSync,
} from "node:fs";
import { createHash } from "node:crypto";
import { isAbsolute, join, relative, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const outputIndex = process.argv.indexOf("--output-dir");
const output = resolve(
  outputIndex >= 0
    ? process.argv[outputIndex + 1]
    : join(projectRoot, "tests", "fixtures", "audio"),
);
const outputRelative = relative(projectRoot, output);
if (
  !outputRelative
  || outputRelative.startsWith("..")
  || isAbsolute(outputRelative)
) {
  throw new Error(`Fixture output must remain inside the repository: ${output}`);
}
mkdirSync(output, { recursive: true });

function digestFile(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

if (process.argv.includes("--verify")) {
  const checksumPath = join(output, "SHA256SUMS.txt");
  const checksums = readFileSync(checksumPath, "utf8")
    .trim()
    .split(/\r?\n/)
    .filter(Boolean);
  for (const line of checksums) {
    const match = /^([0-9a-f]{64}) {2}(.+)$/.exec(line);
    if (!match) throw new Error(`Invalid checksum entry: ${line}`);
    const actual = digestFile(join(output, match[2]));
    if (actual !== match[1]) {
      throw new Error(`Fixture checksum mismatch: ${match[2]}`);
    }
  }
  process.stdout.write(`Verified ${checksums.length} audio fixtures in ${output}\n`);
  process.exit(0);
}

function run(command, args, label) {
  const result = spawnSync(command, args, { cwd: projectRoot, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${label} failed`);
}

const common = [
  "-hide_banner", "-loglevel", "error", "-y",
  "-fflags", "+bitexact", "-flags:a", "+bitexact",
  "-f", "lavfi", "-i", "sine=frequency=997:duration=0.35",
  "-map_metadata", "-1", "-ac", "2",
];
const encode = (name, args) =>
  run("ffmpeg", [...common, ...args.map(String), join(output, name)], `FFmpeg ${name}`);
const wavCodecs = new Map([
  [16, "pcm_s16le"],
  [24, "pcm_s24le"],
  [32, "pcm_s32le"],
]);

for (const sampleRate of [44100, 48000, 96000, 192000]) {
  for (const bitDepth of [16, 24, 32]) {
    encode(`wav_${sampleRate}_${bitDepth}_stereo.wav`, [
      "-ar", sampleRate, "-c:a", wavCodecs.get(bitDepth),
    ]);
  }
  encode(`wav_${sampleRate}_f32_stereo.wav`, [
    "-ar", sampleRate, "-c:a", "pcm_f32le",
  ]);
  for (const bitDepth of [16, 24]) {
    encode(`flac_${sampleRate}_${bitDepth}_stereo.flac`, [
      "-ar", sampleRate,
      "-sample_fmt", bitDepth === 16 ? "s16" : "s32",
      "-bits_per_raw_sample", bitDepth,
      "-c:a", "flac",
    ]);
  }
  run(
    "flac",
    [
      "--force", "--silent", "--no-padding", "--no-seektable",
      "--no-preserve-modtime", "--no-mid-side",
      `--output-name=${join(output, `flac_${sampleRate}_32_stereo.flac`)}`,
      join(output, `wav_${sampleRate}_32_stereo.wav`),
    ],
    `FLAC ${sampleRate} Hz`,
  );
}

for (const sampleRate of [44100, 48000]) {
  encode(`mp3_${sampleRate}_cbr128_stereo.mp3`, [
    "-ar", sampleRate, "-c:a", "libmp3lame", "-b:a", "128k",
  ]);
  encode(`mp3_${sampleRate}_cbr320_stereo.mp3`, [
    "-ar", sampleRate, "-c:a", "libmp3lame", "-b:a", "320k",
  ]);
  encode(`mp3_${sampleRate}_vbr0_stereo.mp3`, [
    "-ar", sampleRate, "-c:a", "libmp3lame", "-q:a", "0",
  ]);
}

run(
  "ffmpeg",
  [
    ...common, "-ar", "44100", "-ac", "1", "-c:a", "pcm_s16le",
    join(output, "wav_44100_16_mono.wav"),
  ],
  "FFmpeg mono fixture",
);
run(
  "ffmpeg",
  [
    "-hide_banner", "-loglevel", "error", "-y",
    "-fflags", "+bitexact", "-flags:a", "+bitexact",
    "-f", "lavfi", "-i", "sine=frequency=997:duration=4",
    "-map_metadata", "-1", "-ac", "2", "-ar", "48000",
    "-sample_fmt", "s32", "-bits_per_raw_sample", "24",
    "-c:a", "flac", join(output, "seek_48000_24_stereo.flac"),
  ],
  "FFmpeg seek fixture",
);

writeFileSync(join(output, "empty.wav"), Buffer.alloc(0));
const validWav = readFileSync(join(output, "wav_44100_16_stereo.wav"));
writeFileSync(join(output, "truncated.wav"), validWav.subarray(0, 32));
writeFileSync(join(output, "wav_content_as_flac.flac"), validWav);

const fixtureNames = readdirSync(output)
  .filter((name) => /\.(?:wav|flac|mp3)$/i.test(name))
  .sort();
const hashes = fixtureNames.map((name) => {
  return `${digestFile(join(output, name))}  ${name}`;
});
writeFileSync(join(output, "SHA256SUMS.txt"), `${hashes.join("\n")}\n`);
process.stdout.write(`Generated ${fixtureNames.length} audio fixtures in ${output}\n`);
