// SPDX-License-Identifier: GPL-3.0-only
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { affectsDelivery, buildIdentity, shouldCreateRelease } from "./release-channel.mjs";
import { archRecipe, archVersion } from "./package-arch.mjs";
import { collectArtifacts, publishDraft, updateManifest, verifyDraftAssets } from "./publish-release.mjs";
const version = "0.3.0-alpha.1";
const identity = buildIdentity(version, "42", "1", "a".repeat(40));

test("main PRs require dual-platform CI; main pushes only run CD", () => {
  const ci = readFileSync(new URL("../.github/workflows/pr-validation.yml", import.meta.url), "utf8").replaceAll("\r\n", "\n");
  const cd = readFileSync(new URL("../.github/workflows/main-merge-delivery.yml", import.meta.url), "utf8").replaceAll("\r\n", "\n");
  assert.match(ci, /on:\s+pull_request:\s+branches:\s+- main\s+env:/);
  assert.doesNotMatch(ci, /\n  push:|environment:|secrets\.|queue: max/);
  assert.match(cd, /on:\s+push:\s+branches:\s+- main\s+env:/);
  assert.doesNotMatch(cd, /pull_request:|windows-validation:|linux-validation:/);
  for (const workflow of [ci, cd]) assert.doesNotMatch(workflow, /pull_request_target:|workflow_dispatch:|\n\s+- dev\b/);
  assert.equal(((ci + cd).match(/cache: npm/g) || []).length, 6);
  assert.equal(((ci + cd).match(/uses: Swatinem\/rust-cache@[a-f0-9]{40}/g) || []).length, 4);
  assert.equal(((ci + cd).match(/key: \$\{\{ env\.RESONA_NATIVE_CACHE \}\}/g) || []).length, 2);
  const job = (workflow, name) => workflow.split(`\n  ${name}:`)[1].split(/\n  [a-z][a-z-]+:/)[0];
  assert.match(job(ci, "windows-validation"), /BASE_SHA: \$\{\{ github.event.pull_request.base.sha \}\}/);
  assert.match(job(ci, "validation"), /name: PR validation/);
  assert.match(job(ci, "validation"), /if: always\(\) && github.event_name == 'pull_request'/);
  assert.match(job(ci, "validation"), /needs: \[windows-validation, linux-validation\]/);
  assert.match(job(ci, "validation"), /test "\$WINDOWS_RESULT" = success && test "\$LINUX_RESULT" = success/);
  assert.match(job(cd, "verify"), /if: github.event_name == 'push' && github.ref == 'refs\/heads\/main'/);
  for (const name of ["windows-build", "arch-build"]) {
    assert.match(job(cd, name), /needs: \[verify\]/);
    assert.match(job(cd, name), /if: needs.verify.outputs.release == 'true'/);
    assert.match(job(cd, name), /environment: release/);
  }
  assert.match(job(cd, "publish"), /needs: \[verify, windows-build, arch-build\]/);
  assert.match(job(cd, "publish"), /environment: release/);
  assert.match(ci, /cancel-in-progress: true/);
  assert.match(cd, /cancel-in-progress: false\n  queue: max/);
});

test("one version keeps one tag across build attempts", () => {
  assert.equal(identity.tag, "v0.3.0-alpha.1");
  assert.equal(identity.tag, buildIdentity(version, 42, 2, "a".repeat(40)).tag);
  assert.notEqual(identity.id, buildIdentity(version, 42, 2, "a".repeat(40)).id);
  assert.equal(shouldCreateRelease({ currentVersion: version, previousVersion: version, tagExists: true }), false);
  assert.equal(shouldCreateRelease({ currentVersion: version, previousVersion: version, tagExists: false }), true);
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
  const versions = ["0.3.0alpha1-1", "0.3.0alpha2-1", "0.3.0beta1-1", "0.3.0rc1-1", "0.3.0-1"];
  for (let i = 1; i < versions.length; i++) assert.equal(spawnSync("vercmp", [versions[i - 1], versions[i]], { encoding: "utf8" }).stdout.trim(), "-1");
  assert.equal(archVersion(version), "0.3.0alpha1");
  assert.equal(archVersion("0.3.0-rc.10"), "0.3.0rc10");
  assert.throws(() => archVersion("0.3.0-weird.1"));
});

test("preview and stable releases share one Arch package identity", () => {
  for (const v of ["0.3.0-alpha.4", "0.3.0-beta.1", "0.3.0-rc.1", "0.3.0"]) {
    const recipe = archRecipe({ version: v });
    assert.match(recipe, /^pkgname=resona$/m);
    assert.doesNotMatch(recipe, /^provides=|^conflicts=|^replaces=/m);
    const pkg = `resona-${archVersion(v)}-1-x86_64.pkg.tar.zst`;
    const id = buildIdentity(v, 42, 1, "a".repeat(40));
    const installer = `Resona_${v}_windows_x86_64-setup.exe`;
    const manifest = updateManifest(v, id, installer, "signature", pkg, "signature");
    assert.equal(manifest.channel, v.includes("-") ? "preview" : "stable");
    assert.ok(manifest.platforms["linux-x86_64"].url.endsWith(pkg));
  }
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
    for (const field of ["pkgbase = resona", "pkgver = 0.3.0alpha1", "pkgrel = 1", "depends = gtk-layer-shell", "depends = polkit"]) assert.ok(result.stdout.includes(field), field);
  } finally { rmSync(dir, { recursive: true, force: true }); }
});

test("publishing requires both identities and an exact Arch package", () => {
  const dir = mkdtempSync(join(tmpdir(), 'resona-delivery-'));
  const input = join(dir, 'artifacts');
  const windows = join(input, 'windows-delivery'), linux = join(input, 'arch-delivery');
  const output = join(dir, 'output');
  try {
    for (const folder of [windows, linux]) { mkdirSync(folder, {recursive:true}); writeFileSync(join(folder, 'build.json'), JSON.stringify({version,...identity})); }
    const installer = 'Resona_' + version + '_windows_x86_64-setup.exe';
    writeFileSync(join(windows, installer), 'binary'); writeFileSync(join(windows, installer + '.sig'), 'signature');
    assert.throws(() => collectArtifacts(input, output, version, identity), /Exactly one Arch/);
    const pkg = 'resona-0.3.0alpha1-1-x86_64.pkg.tar.zst';
    writeFileSync(join(linux, pkg), 'package');
    const manifest = collectArtifacts(input, output, version, identity).manifest;
    assert.deepEqual(Object.keys(manifest.platforms), ['windows-x86_64', 'linux-x86_64']);
    assert.ok(manifest.platforms['linux-x86_64'].url.endsWith(pkg));
    assert.throws(() => updateManifest(version, identity, installer, 'signature', 'other.pkg.tar.zst', 'signature'), /Invalid signed Arch/);
    assert.throws(() => updateManifest(version, identity, installer, 'signature', pkg, ''), /Invalid signed Arch/);
    writeFileSync(join(linux, 'build.json'), JSON.stringify({ version, ...buildIdentity(version,43,1,'a'.repeat(40)) }));
    assert.throws(() => collectArtifacts(input, output, version, identity), /identity mismatch/);
  } finally { rmSync(dir,{recursive:true,force:true}); }
});

test("release verification rejects GitHub-renamed, missing, extra or duplicate assets and non-drafts", () => {
  const expected = ["PKGBUILD", "SRCINFO", "SHA256SUMS"];
  const draft = (names, isDraft = true) => ({ isDraft, assets: names.map((name) => ({ name })) });
  verifyDraftAssets(draft([...expected].reverse()), expected);
  assert.throws(() => verifyDraftAssets(draft(["PKGBUILD", "default.SRCINFO", "SHA256SUMS"]), ["PKGBUILD", ".SRCINFO", "SHA256SUMS"]), /expected:.*\.SRCINFO.*actual:.*default\.SRCINFO/);
  for (const names of [expected.slice(1), [...expected, "unexpected.txt"], [...expected, "SRCINFO"]]) {
    assert.throws(() => verifyDraftAssets(draft(names), expected), /Draft release asset list is incomplete/);
  }
  assert.throws(() => verifyDraftAssets(draft(expected, false), expected), /isDraft=false/);
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

test("publication uses one version tag and minimal notes, retries drafts and preserves published releases", () => {
  const dir = mkdtempSync(join(tmpdir(), "resona-publish-"));
  writeFileSync(join(dir, "installer.exe"), "installer");
  const notes = "docs/releases/0.3.0-alpha.1.md";
  const run = ({ existing, failUpload = false, missingAsset = false, externallyPublished = false, failEdit = false } = {}) => {
    const calls = [];
    const gh = (args) => {
      calls.push(args);
      if (args[0] === "api") return JSON.stringify(existing ? [{ tag_name: identity.tag, draft: existing === "draft" }] : []);
      if (args[1] === "upload" && failUpload) throw new Error("upload failed");
      if (args[1] === "view") return JSON.stringify({ isDraft: !externallyPublished, assets: missingAsset ? [] : [{ name: "installer.exe" }] });
      if (args[1] === "edit" && failEdit) throw new Error("publish response lost");
      return "";
    };
    let error;
    try { publishDraft(gh, identity, version, true, dir, notes); } catch (caught) { error = caught; }
    return { calls, error };
  };
  try {
    for (const existing of [undefined, "draft"]) {
      const { calls, error } = run({ existing });
      assert.ifError(error);
      const create = calls.find((args) => args[1] === "create");
      assert.equal(create[2], "v0.3.0-alpha.1");
      assert.equal(create[create.indexOf("--title") + 1], "Resona 0.3.0-alpha.1");
      assert.equal(create[create.indexOf("--notes-file") + 1], notes);
      assert.ok(create.includes("--prerelease"));
      assert.ok(!create.includes("--generate-notes"));
      assert.equal(calls.filter((args) => args[1] === "delete").length, existing ? 1 : 0);
      assert.equal(calls.at(-1)[1], "edit");
    }
    const published = run({ existing: "published" });
    assert.match(published.error.message, /already published/);
    assert.equal(published.calls.length, 1);
    for (const options of [{ failUpload: true }, { missingAsset: true }]) {
      const { calls, error } = run(options);
      assert.ok(error);
      assert.equal(calls.at(-1)[1], "delete");
      assert.ok(!calls.some((args) => args[1] === "edit"));
    }
    for (const options of [{ failUpload: true, externallyPublished: true }, { failEdit: true }]) {
      const { calls, error } = run(options);
      assert.ok(error);
      assert.ok(!calls.some((args) => args[1] === "delete"));
    }
  } finally { rmSync(dir, { recursive: true, force: true }); }
});
