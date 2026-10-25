// SPDX-License-Identifier: GPL-3.0-only

import assert from "node:assert/strict";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const config = (name) => JSON.parse(readFileSync(new URL(`../src-tauri/${name}`, import.meta.url), "utf8"));

test("Linux builds cannot inherit the Windows installer or signed updater artifacts", () => {
  const common = config("tauri.conf.json");
  assert.deepEqual(common.bundle.targets, []);
  assert.equal(common.bundle.createUpdaterArtifacts, false);
  assert.equal(common.bundle.windows, undefined);
  assert.equal(common.app.windows[0].transparent, undefined);
  assert.deepEqual(Object.keys(common.bundle.resources), ["icons/default-artwork.png"]);
});

test("Windows retains NSIS, signed updater artifacts and its native resources", () => {
  const windows = config("tauri.windows.conf.json");
  assert.deepEqual(windows.bundle.targets, ["nsis"]);
  assert.equal(windows.bundle.createUpdaterArtifacts, true);
  assert.equal(windows.bundle.windows.nsis.installMode, "currentUser");
  assert.equal(windows.app.windows[0].transparent, true);
  const common = config("tauri.conf.json");
  for (const field of ["width", "height", "minWidth", "minHeight"]) {
    assert.equal(common.app.windows[0][field], windows.app.windows[0][field]);
  }
  assert.ok(windows.bundle.resources["icons/taskbar-play.ico"]);
});

test("Linux test launcher isolates app data while retaining desktop settings and user fonts", { skip: process.platform !== "linux" }, () => {
  const root = mkdtempSync(join(tmpdir(), "resona font test "));
  try {
    const bundle = join(root, "bundle");
    const desktopData = join(root, "desktop-data");
    const desktopConfig = join(root, "desktop-config");
    mkdirSync(join(bundle, "bin"), { recursive: true });
    mkdirSync(join(desktopData, "fonts"), { recursive: true });
    mkdirSync(join(desktopConfig, "gtk-3.0"), { recursive: true });
    writeFileSync(join(desktopData, "fonts", "font-probe"), "desktop font");
    writeFileSync(join(desktopConfig, "gtk-3.0", "settings.ini"), "desktop settings");
    copyFileSync(new URL("./start-linux-test.sh", import.meta.url), join(bundle, "start-resona.sh"));
    writeFileSync(join(bundle, "bin", "resona"), `#!/bin/sh
set -eu
cat "$XDG_CONFIG_HOME/gtk-3.0/settings.ini"
cat "$XDG_DATA_HOME/fonts/font-probe"
printf '\\n%s\\n%s\\n%s\\n' "$XDG_DATA_HOME" "$XDG_CACHE_HOME" "$1"
`, { mode: 0o755 });
    for (let run = 0; run < 2; run++) {
      const result = spawnSync("sh", [join(bundle, "start-resona.sh"), "歌曲.flac"], {
        cwd: tmpdir(),
        env: { ...process.env, XDG_CONFIG_HOME: desktopConfig, XDG_DATA_HOME: desktopData },
        encoding: "utf8",
      });
      assert.equal(result.status, 0, result.stderr);
      assert.equal(result.stdout, `desktop settingsdesktop font\n${bundle}/test-profile/data\n${bundle}/test-profile/cache\n歌曲.flac\n`);
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
