// SPDX-License-Identifier: GPL-3.0-only
import { chmodSync, copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { buildIdentity, readProjectVersion } from "./release-channel.mjs";

export function archVersion(version) {
  if (!/^\d+\.\d+\.\d+(?:-(?:alpha|beta|rc)\.\d+)?$/.test(version)) {
    throw new Error(`Unsupported Arch version mapping: ${version}`);
  }
  return version.replace(/-(alpha|beta|rc)\./, "$1");
}

export function archRecipe({ version, identity, asset, checksum }) {
  if (!/^[a-zA-Z0-9_.-]+\.tar\.gz$/.test(asset) || !/^[a-f0-9]{64}$/.test(checksum)) {
    throw new Error("Invalid Arch payload name or checksum");
  }
  const preview = version.includes("-");
  return `# Maintained by the Resona release workflow; source: ${identity.sha}
pkgname=${preview ? "resona-preview-bin" : "resona-bin"}
pkgver=${archVersion(version)}
pkgrel=1
pkgdesc='Local audio player with desktop lyrics (Arch Wayland, precompiled)'
arch=('x86_64')
url='https://github.com/Leovikii/Resona'
license=('GPL-3.0-only')
depends=('desktop-file-utils' 'hicolor-icon-theme' 'glibc' 'gcc-libs' 'alsa-lib' 'gtk3' 'webkit2gtk-4.1' 'libsoup3' 'gtk-layer-shell' 'libayatana-appindicator' 'dbus' 'openssl')
optdepends=('ffmpeg: WAV to FLAC conversion' 'pipewire-alsa: ALSA output via PipeWire' 'xdg-desktop-portal-kde: KDE file chooser')
provides=('resona')
conflicts=('resona' '${preview ? 'resona-bin' : 'resona-preview-bin'}')
options=('!strip' '!debug')
source=("https://github.com/Leovikii/Resona/releases/download/${identity.tag}/${asset}")
sha256sums=('${checksum}')

package() {
  cp -a "$srcdir/usr" "$pkgdir/"
}
`;
}

export function packageArch(root, output, binary, identity, version) {
  const stage = resolve(output, "arch-root");
  rmSync(stage, { recursive: true, force: true });
  const install = (source, destination, mode = 0o644) => {
    const target = resolve(stage, destination);
    mkdirSync(resolve(target, ".."), { recursive: true });
    copyFileSync(resolve(root, source), target);
    // Copies from a local build must not leak unusual permissions into the package.
    chmodSync(target, mode);
  };
  install(binary, "usr/bin/resona", 0o755);
  install("src-tauri/icons/default-artwork.png", "usr/lib/Resona/icons/default-artwork.png");
  install("packaging/arch/io.github.vki.resona.desktop", "usr/share/applications/io.github.vki.resona.desktop");
  install("src-tauri/icons/128x128.png", "usr/share/icons/hicolor/128x128/apps/io.github.vki.resona.png");
  const name = version.includes("-") ? "resona-preview-bin" : "resona-bin";
  install("LICENSE", `usr/share/licenses/${name}/LICENSE`);
  install("docs/THIRD_PARTY_LICENSES.md", `usr/share/licenses/${name}/THIRD_PARTY_LICENSES.md`);
  mkdirSync(output, { recursive: true });
  const metadata = { version, ...identity };
  writeFileSync(resolve(stage, "usr/lib/Resona/build.json"), `${JSON.stringify(metadata, null, 2)}\n`);
  const asset = `Resona_${version}_arch_x86_64.tar.gz`;
  execFileSync("tar", ["-czf", resolve(output, asset), "-C", stage, "usr"]);
  const checksum = createHash("sha256").update(readFileSync(resolve(output, asset))).digest("hex");
  const recipe = resolve(output, "aur");
  mkdirSync(recipe, { recursive: true });
  writeFileSync(resolve(recipe, "PKGBUILD"), archRecipe({ version, identity, asset, checksum }));
  // makepkg can validate the real checksum offline before this asset is public.
  copyFileSync(resolve(output, asset), resolve(recipe, asset));
  writeFileSync(resolve(output, "build.json"), `${JSON.stringify(metadata, null, 2)}\n`);
  return { asset, checksum, recipe };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const { version } = readProjectVersion();
  const identity = buildIdentity(version, process.env.RESONA_BUILD_NUMBER, process.env.RESONA_BUILD_ATTEMPT, process.env.RESONA_BUILD_SHA);
  packageArch(resolve("."), resolve("src-tauri/target/delivery"), "src-tauri/target/release/resona", identity, version);
}
