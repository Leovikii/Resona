// SPDX-License-Identifier: GPL-3.0-only
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { affectsDelivery, buildIdentity, shouldCreateRelease } from "./release-channel.mjs";
import { archRecipe, archVersion } from "./package-arch.mjs";
import { collectArtifacts, updateManifest } from "./publish-release.mjs";
const version = "0.3.0-alpha.1";
const identity = buildIdentity(version, "42", "1", "a".repeat(40));

test("repeat preview builds have immutable, numeric identities without changing application version", () => {
  assert.equal(identity.tag, "preview-v0.3.0-alpha.1-b42.1-gaaaaaaa");
  assert.notEqual(identity.tag, buildIdentity(version, 42, 2, "a".repeat(40)).tag);
  assert.equal(buildIdentity("0.3.0", 43, 1, "a".repeat(40)).tag, "v0.3.0");
  for (const invalid of [0, -1, "01", "1.1", "x", Number.MAX_SAFE_INTEGER + 1]) assert.throws(() => buildIdentity(version, invalid, 1, "a".repeat(40)));
  assert.throws(() => buildIdentity(version, 1, 1, "short"));
  assert.equal(shouldCreateRelease({ currentVersion: version, previousVersion: version, tagExists: false, deliveryChanged: false }), false);
});

test("delivery paths include runtime resources, packaging, locks and workflow changes", () => {
  for (const path of ["src/app/App.tsx", "src-tauri/icons/icon.ico", "src-tauri/Cargo.lock", "package-lock.json", "scripts/package-arch.mjs", "packaging/arch/io.github.vki.resona.desktop", ".github/workflows/main-merge-delivery.yml", "docs/THIRD_PARTY_LICENSES.md"]) assert.equal(affectsDelivery([path]), true, path);
  assert.equal(affectsDelivery(["docs/STATUS.md", "README.md", "assets/README.md"]), false);
});

test("Arch package revisions and preview mapping sort before final releases", { skip: process.platform !== "linux" }, () => {
  const probe = spawnSync("vercmp", ["1", "1"]);
  if (probe.error?.code === "ENOENT") return;
  assert.ifError(probe.error);
  const versions = ["0.3.0alpha1-42.1", "0.3.0alpha1-42.2", "0.3.0alpha1-43.1", "0.3.0alpha2-1", "0.3.0-1"];
  for (let i = 1; i < versions.length; i++) assert.equal(spawnSync("vercmp", [versions[i - 1], versions[i]], { encoding: "utf8" }).stdout.trim(), "-1");
  assert.equal(archVersion(version), "0.3.0alpha1");
  assert.equal(archVersion("0.3.0-rc.10"), "0.3.0rc10");
  assert.throws(() => archVersion("0.3.0-weird.1"));
});

test("makepkg can parse the generated recipe without evaluating remote input", { skip: process.platform !== "linux" || process.getuid?.() === 0 }, () => {
  const probe = spawnSync("makepkg", ["--version"]);
  if (probe.error?.code === "ENOENT") return;
  assert.ifError(probe.error);
  const dir = mkdtempSync(join(tmpdir(), "resona-recipe-"));
  try {
    writeFileSync(join(dir, "PKGBUILD"), archRecipe({ version, identity, asset: "Resona.tar.gz", checksum: "b".repeat(64) }));
    const result = spawnSync("makepkg", ["--printsrcinfo"], { cwd: dir, encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
    for (const field of ["pkgbase = resona-preview-bin", "pkgver = 0.3.0alpha1", "pkgrel = 42.1", "depends = gtk-layer-shell", `sha256sums = ${"b".repeat(64)}`]) assert.ok(result.stdout.includes(field), field);
  } finally { rmSync(dir, { recursive: true, force: true }); }
});

test("publishing requires both platform identities and the exact checksummed payload", () => {
  const dir = mkdtempSync(join(tmpdir(), "resona-delivery-"));
  const input = join(dir, "artifacts");
  const windows = join(input, "windows-delivery");
  const linux = join(input, "arch-delivery");
  const output = join(dir, "output");
  try {
    mkdirSync(windows, { recursive: true });
    mkdirSync(linux, { recursive: true });
    const metadata = JSON.stringify({ version, ...identity });
    writeFileSync(join(windows, "build.json"), metadata);
    assert.throws(() => collectArtifacts(input, output, version, identity));
    writeFileSync(join(linux, "build.json"), metadata);
    const installer = `Resona_${version}_${identity.id}_windows_x86_64-setup.exe`;
    const payload = `Resona_${version}_${identity.id}_arch_x86_64.tar.gz`;
    writeFileSync(join(windows, installer), "binary");
    writeFileSync(join(windows, `${installer}.sig`), "signature");
    writeFileSync(join(linux, payload), "payload");
    const checksum = createHash("sha256").update("payload").digest("hex");
    writeFileSync(join(linux, "PKGBUILD"), archRecipe({ version, identity, asset: payload, checksum }));
    writeFileSync(join(linux, ".SRCINFO"), `sha256sums = ${checksum}`);
    writeFileSync(join(linux, "arch-packages.txt"), "arch packages");
    assert.throws(() => collectArtifacts(input, output, version, identity), /Exactly one Arch/);
    writeFileSync(join(linux, "resona-preview-bin.pkg.tar.zst"), "package");
    assert.equal(collectArtifacts(input, output, version, identity).manifest.version, version);
    assert.equal(readFileSync(join(output, installer), "utf8"), "binary");
    writeFileSync(join(linux, payload), "tampered");
    assert.throws(() => collectArtifacts(input, output, version, identity), /does not match/);
    writeFileSync(join(linux, "build.json"), JSON.stringify({ version, ...buildIdentity(version, 43, 1, "a".repeat(40)) }));
    assert.throws(() => collectArtifacts(input, output, version, identity), /identity mismatch/);
    const manifest = updateManifest(version, identity, installer, "signature");
    assert.deepEqual(Object.keys(manifest.platforms), ["windows-x86_64"]);
    assert.equal(manifest.build.runNumber, 42);
  } finally { rmSync(dir, { recursive: true, force: true }); }
});


test("desktop registration advertises the actual MP3, WAV and FLAC MIME types", { skip: process.platform !== "linux" }, () => {
  const desktop = new URL("../packaging/arch/io.github.vki.resona.desktop", import.meta.url);
  const content = readFileSync(desktop, "utf8");
  assert.match(content, /^Exec=resona %F$/m);
  const mimeTypes = /^MimeType=(.*)$/m.exec(content)[1].split(";");
  for (const fixture of ["mp3_44100_cbr128_stereo.mp3", "wav_44100_16_stereo.wav", "flac_44100_16_stereo.flac"]) {
    const path = new URL(`../tests/fixtures/audio/${fixture}`, import.meta.url);
    const result = spawnSync("xdg-mime", ["query", "filetype", fileURLToPath(path)], { encoding: "utf8" });
    if (result.error?.code === "ENOENT") return;
    assert.ifError(result.error);
    assert.equal(result.status, 0, result.stderr);
    assert.ok(mimeTypes.includes(result.stdout.trim()), result.stdout);
  }
});

test("AUR sync is idempotent, rejects downgrades and verifies the public payload before pushing", { skip: process.platform !== "linux" }, () => {
  const probe = spawnSync("vercmp", ["1", "1"]);
  if (probe.error?.code === "ENOENT") return;
  assert.ifError(probe.error);
  const dir = mkdtempSync(join(tmpdir(), "resona-aur-sync-"));
  const bin = join(dir, "bin");
  const incoming = join(dir, "incoming");
  const old = join(dir, "old");
  const log = join(dir, "git.log");
  const payload = join(dir, "payload");
  const checksum = createHash("sha256").update("payload").digest("hex");
  const srcinfo = (revision) => `pkgbase = resona-preview-bin\n\tpkgver = 0.3.0alpha1\n\tpkgrel = ${revision}\n\tsource = https://github.com/Leovikii/Resona/releases/download/${identity.tag}/Resona.tar.gz\n\tsha256sums = ${checksum}\n`;
  try {
    for (const path of [bin, incoming, old]) mkdirSync(path, { recursive: true });
    writeFileSync(payload, "payload");
    writeFileSync(join(incoming, "PKGBUILD"), "recipe\n");
    writeFileSync(join(incoming, ".SRCINFO"), srcinfo("42.1"));
    writeFileSync(join(old, "PKGBUILD"), "recipe\n");
    writeFileSync(join(bin, "git"), `#!/bin/sh
set -eu
if [ "$1" = clone ]; then
  mkdir -p "$3"
  cp "$AUR_TEST_OLD/PKGBUILD" "$AUR_TEST_OLD/.SRCINFO" "$3/"
else
  printf '%s\\n' "$*" >> "$AUR_TEST_GIT_LOG"
fi
`, { mode: 0o755 });
    writeFileSync(join(bin, "curl"), `#!/bin/sh
set -eu
previous=''
for argument do
  if [ "$previous" = -o ]; then cp "$AUR_TEST_PAYLOAD" "$argument"; exit 0; fi
  previous=$argument
done
exit 1
`, { mode: 0o755 });
    const run = () => {
      writeFileSync(log, "");
      return spawnSync("bash", [fileURLToPath(new URL("./sync-aur.sh", import.meta.url)), incoming], {
        encoding: "utf8",
        env: { ...process.env, PATH: `${bin}:${process.env.PATH}`, RESONA_AUR_SSH_KEY: "disposable-test-value", RESONA_AUR_KNOWN_HOSTS: "test-host", RESONA_RELEASE_TAG: identity.tag, AUR_TEST_OLD: old, AUR_TEST_PAYLOAD: payload, AUR_TEST_GIT_LOG: log },
      });
    };
    writeFileSync(join(old, ".SRCINFO"), srcinfo("41.9"));
    assert.equal(run().status, 0);
    assert.match(readFileSync(log, "utf8"), /push origin HEAD:master/);
    writeFileSync(join(old, ".SRCINFO"), srcinfo("43.1"));
    const downgrade = run();
    assert.notEqual(downgrade.status, 0);
    assert.match(downgrade.stderr, /Refusing AUR downgrade/);
    assert.equal(readFileSync(log, "utf8"), "");
    writeFileSync(join(old, ".SRCINFO"), srcinfo("42.1"));
    assert.equal(run().status, 0);
    assert.equal(readFileSync(log, "utf8"), "");
    writeFileSync(join(old, "PKGBUILD"), "different recipe\n");
    assert.notEqual(run().status, 0);
    assert.equal(readFileSync(log, "utf8"), "");
    writeFileSync(payload, "tampered");
    assert.notEqual(run().status, 0);
    assert.equal(readFileSync(log, "utf8"), "");
  } finally { rmSync(dir, { recursive: true, force: true }); }
});
