// SPDX-License-Identifier: GPL-3.0-only
import { copyFileSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { buildIdentity, readProjectVersion } from "./release-channel.mjs";

const repository = "Leovikii/Resona";
export function updateManifest(version, identity, installer, signature) {
  if (installer !== `Resona_${version}_${identity.id}_windows_x86_64-setup.exe` || !signature.trim()) throw new Error("Invalid signed installer");
  return {
    schema: 1, version, tag: identity.tag, channel: version.includes("-") ? "preview" : "stable",
    build: { runNumber: identity.runNumber, runAttempt: identity.runAttempt, sha: identity.sha },
    notes: `Resona ${version} · ${identity.id}`,
    pub_date: new Date().toISOString(),
    platforms: { "windows-x86_64": { signature: signature.trim(), url: `https://github.com/${repository}/releases/download/${identity.tag}/${installer}` } },
  };
}

export function collectArtifacts(input, output, version, identity) {
  const windows = resolve(input, "windows-delivery");
  const linux = resolve(input, "arch-delivery");
  for (const platform of [windows, linux]) {
    const actual = JSON.parse(readFileSync(resolve(platform, "build.json"), "utf8"));
    if (JSON.stringify(actual) !== JSON.stringify({ version, ...identity })) throw new Error("Platform build identity mismatch");
  }
  const installer = `Resona_${version}_${identity.id}_windows_x86_64-setup.exe`;
  const payload = `Resona_${version}_${identity.id}_arch_x86_64.tar.gz`;
  const packages = readdirSync(linux).filter((name) => name.endsWith(".pkg.tar.zst"));
  if (packages.length !== 1) throw new Error("Exactly one Arch package is required");
  const payloadHash = createHash("sha256").update(readFileSync(resolve(linux, payload))).digest("hex");
  const recipe = readFileSync(resolve(linux, "PKGBUILD"), "utf8");
  const srcinfo = readFileSync(resolve(linux, ".SRCINFO"), "utf8");
  if (!recipe.includes(`sha256sums=('${payloadHash}')`) || !srcinfo.includes(payloadHash) || !recipe.includes(`/${identity.tag}/${payload}`)) throw new Error("Arch recipe does not match payload");
  mkdirSync(output, { recursive: true });
  for (const name of [installer, `${installer}.sig`]) copyFileSync(resolve(windows, name), resolve(output, name));
  for (const name of [payload, packages[0], "PKGBUILD", "build.json", "arch-packages.txt"]) copyFileSync(resolve(linux, name), resolve(output, name));
  // GitHub renames dotfiles (for example .SRCINFO to default.SRCINFO) on upload.
  copyFileSync(resolve(linux, ".SRCINFO"), resolve(output, "SRCINFO"));
  return { installer, manifest: updateManifest(version, identity, installer, readFileSync(resolve(output, `${installer}.sig`), "utf8")) };
}

export function verifyDraftAssets(uploaded, expected) {
  const actual = uploaded.assets.map((asset) => asset.name).sort();
  if (!uploaded.isDraft || JSON.stringify(actual) !== JSON.stringify([...expected].sort())) {
    throw new Error(`Draft release asset list is incomplete (isDraft=${uploaded.isDraft}); expected: ${JSON.stringify([...expected].sort())}; actual: ${JSON.stringify(actual)}`);
  }
}

function verifyFile(file, encodedSignature, encodedPublicKey, scratch) {
  const key = resolve(scratch, "public-key");
  const sig = resolve(scratch, "signature");
  writeFileSync(key, Buffer.from(encodedPublicKey.trim(), "base64"));
  writeFileSync(sig, Buffer.from(encodedSignature.trim(), "base64"));
  execFileSync("minisign", ["-Vm", file, "-p", key, "-x", sig, "-Q"], { stdio: "pipe" });
}

function publish() {
  const { version, prerelease } = readProjectVersion();
  const identity = buildIdentity(version, process.env.RESONA_BUILD_NUMBER, process.env.RESONA_BUILD_ATTEMPT, process.env.RESONA_BUILD_SHA);
  const base = resolve("src-tauri/target");
  const output = resolve(base, "release-assets");
  const scratch = resolve(base, "release-verification");
  mkdirSync(scratch, { recursive: true });
  const { installer, manifest } = collectArtifacts(resolve(base, "artifacts"), output, version, identity);
  const publicKey = process.env.RESONA_UPDATER_PUBLIC_KEY;
  if (!publicKey) throw new Error("RESONA_UPDATER_PUBLIC_KEY is required");
  verifyFile(resolve(output, installer), readFileSync(resolve(output, `${installer}.sig`), "utf8"), publicKey, scratch);
  const manifestPath = resolve(output, "latest.json");
  writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  execFileSync(process.execPath, ["node_modules/@tauri-apps/cli/tauri.js", "signer", "sign", manifestPath], { stdio: "pipe" });
  verifyFile(manifestPath, readFileSync(`${manifestPath}.sig`, "utf8"), publicKey, scratch);
  execFileSync("git", ["archive", "--format=tar.gz", `--output=${resolve(output, `Resona_${version}_${identity.id}_source.tar.gz`)}`, identity.sha]);
  for (const name of ["LICENSE", "docs/THIRD_PARTY_LICENSES.md"]) copyFileSync(name, resolve(output, name.split("/").at(-1)));
  const files = readdirSync(output).sort();
  writeFileSync(resolve(output, "SHA256SUMS"), files.map((name) => `${createHash("sha256").update(readFileSync(resolve(output, name))).digest("hex")}  ${name}\n`).join(""));
  const gh = (args) => execFileSync("gh", args[0] === "api" ? args : [...args, "--repo", repository], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
  // Never mutate a previously published release, even when an Actions run is retried.
  const releases = JSON.parse(gh(["api", `repos/${repository}/releases?per_page=100`]));
  if (releases.some((release) => release.tag_name === identity.tag)) throw new Error("Release identity already exists; rerun all jobs to use a new attempt");
  const notes = resolve(base, "release-notes.md");
  writeFileSync(notes, `Build: ${identity.id}\nCommit: ${identity.sha}\n\nWindows: signed Tauri updater / NSIS. This installer is not Authenticode signed; Windows may show SmartScreen. Installing a preview replaces an existing Resona installation. The first build-aware preview must be installed manually.\n\nArch x86_64 / Wayland: use resona-${prerelease ? "preview-" : ""}bin, or download the .pkg.tar.zst and run pacman -U. A local package does not subscribe to an update repository. AUR availability is reported by the separate AUR synchronization job. No AppImage or Linux application self-update. Local test-profile data is not migrated automatically.\n\nVerify downloads against SHA256SUMS. Source and third-party notices are included.\n`);
  gh(["release", "create", identity.tag, "--target", identity.sha, "--title", `Resona ${version}${prerelease ? ` · Preview build ${identity.runNumber}.${identity.runAttempt}` : ""}`, "--draft", "--latest=false", ...(prerelease ? ["--prerelease"] : []), "--notes-file", notes, "--generate-notes"]);
  gh(["release", "upload", identity.tag, ...readdirSync(output).map((name) => resolve(output, name))]);
  const uploaded = JSON.parse(gh(["release", "view", identity.tag, "--json", "assets,isDraft"]));
  const expected = readdirSync(output).sort();
  verifyDraftAssets(uploaded, expected);
  gh(["release", "edit", identity.tag, "--draft=false", `--latest=${prerelease ? "false" : "true"}`]);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) publish();
