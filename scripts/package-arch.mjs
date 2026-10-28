// SPDX-License-Identifier: GPL-3.0-only
import { chmodSync, copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { buildIdentity, readProjectVersion } from "./release-channel.mjs";
export function archVersion(version) {
  if (!/^\d+\.\d+\.\d+(?:-(?:alpha|beta|rc)\.\d+)?$/.test(version)) throw new Error(`Unsupported Arch version: ${version}`);
  return version.replace(/-(alpha|beta|rc)\./, "$1");
}
export function archRecipe({ version }) {
  let recipe = readFileSync(new URL("../packaging/arch/PKGBUILD", import.meta.url), "utf8");
  recipe = recipe.replace(/^pkgver=.*$/m, `pkgver=${archVersion(version)}`);
  return recipe;
}
export function packageArch(root, output, binary, identity, version) {
  const stage = resolve(output, "package/stage");
  rmSync(stage, { recursive: true, force: true });
  const install = (source, destination, mode = 0o644) => {
    const target = resolve(stage, destination);
    mkdirSync(resolve(target, ".."), { recursive: true });
    copyFileSync(resolve(root, source), target); chmodSync(target, mode);
  };
  install(binary, "usr/bin/resona", 0o755);
  install("src-tauri/icons/default-artwork.png", "usr/lib/Resona/icons/default-artwork.png");
  install("packaging/arch/io.github.vki.resona.desktop", "usr/share/applications/io.github.vki.resona.desktop");
  install("src-tauri/icons/128x128.png", "usr/share/icons/hicolor/128x128/apps/io.github.vki.resona.png");
  install("LICENSE", "usr/share/licenses/resona/LICENSE");
  install("docs/THIRD_PARTY_LICENSES.md", "usr/share/licenses/resona/THIRD_PARTY_LICENSES.md");
  const metadata = `${JSON.stringify({ version, ...identity }, null, 2)}\n`;
  writeFileSync(resolve(stage, "usr/lib/Resona/build.json"), metadata);
  writeFileSync(resolve(output, "build.json"), metadata);
  writeFileSync(resolve(output, "package/PKGBUILD"), archRecipe({ version }));
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const { version } = readProjectVersion();
  packageArch(resolve("."), resolve("src-tauri/target/delivery"), "src-tauri/target/release/resona", buildIdentity(version, process.env.RESONA_BUILD_NUMBER, process.env.RESONA_BUILD_ATTEMPT, process.env.RESONA_BUILD_SHA), version);
}
