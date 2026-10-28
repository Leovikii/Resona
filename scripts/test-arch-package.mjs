// Runs only inside the disposable Arch Actions container.
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { archVersion, packageArch } from "./package-arch.mjs";
import { buildIdentity, readProjectVersion } from "./release-channel.mjs";
assert.equal(process.env.GITHUB_ACTIONS, "true");
assert.equal(process.getuid(), 0);
assert.match(fs.readFileSync("/etc/os-release", "utf8"), /^ID=arch$/m);
const run = (cmd, args, options = {}) => execFileSync(cmd, args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], ...options });
const root = fs.mkdtempSync(path.join(os.tmpdir(), "resona-package-"));
const { version } = readProjectVersion();
const name = "resona";
let installed = false;
try {
  run("useradd", ["--create-home", "resona-builder"]);
  fs.chmodSync(root, 0o755);
  for (const v of ["0.3.0-alpha.2", version]) {
    const out = path.join(root, archVersion(v));
    packageArch(process.cwd(), out, process.env.RESONA_PACKAGE_BINARY, buildIdentity(v, 1, 1, "a".repeat(40)), v);
    run("chown", ["-R", "resona-builder:resona-builder", out]);
    run("runuser", ["-u", "resona-builder", "--", "makepkg", "--nodeps", "--noconfirm"], { cwd: path.join(out, "package") });
    const file = fs.readdirSync(path.join(out, "package")).find(f => f.endsWith(".pkg.tar.zst"));
    assert.ok(file);
    const packagePath = path.join(out, "package", file);
    if (v === version) {
      execFileSync("cargo", ["test", "--manifest-path", "src-tauri/Cargo.toml", "--locked", "packaged_update_metadata", "--", "--ignored"], { stdio: "inherit", env: { ...process.env, RESONA_TEST_PACKAGE: packagePath } });
    }
    run("pacman", ["-U", "--noconfirm", "--", packagePath]);
    installed = true;
    assert.equal(run("pacman", ["-Q", name]).trim(), `${name} ${archVersion(v)}-1`);
    assert.equal(JSON.parse(fs.readFileSync("/usr/lib/Resona/build.json", "utf8")).version, v);
    run("pacman", ["-Qk", name]);
    run("desktop-file-validate", ["/usr/share/applications/io.github.vki.resona.desktop"]);
    for (const mime of ["audio/mpeg", "audio/vnd.wave", "audio/flac"]) assert.ok(fs.readFileSync("/usr/share/applications/mimeinfo.cache", "utf8").includes(`${mime}=io.github.vki.resona.desktop;`));
    if (v === version) {
      run("pacman", ["-U", "--noconfirm", "--", packagePath]);
      if (version.includes("-")) {
        const stable = version.split("-")[0];
        const stableOut = path.join(root, "stable");
        packageArch(process.cwd(), stableOut, process.env.RESONA_PACKAGE_BINARY, buildIdentity(stable, 1, 1, "a".repeat(40)), stable);
        run("chown", ["-R", "resona-builder:resona-builder", stableOut]);
        run("runuser", ["-u", "resona-builder", "--", "makepkg", "--nodeps", "--noconfirm"], { cwd: path.join(stableOut, "package") });
        const stableFile = fs.readdirSync(path.join(stableOut, "package")).find(f => f.endsWith(".pkg.tar.zst"));
        assert.ok(stableFile);
        run("pacman", ["-U", "--noconfirm", "--", path.join(stableOut, "package", stableFile)]);
        assert.equal(run("pacman", ["-Q", name]).trim(), `${name} ${archVersion(stable)}-1`);
        assert.equal(run("pacman", ["-Qoq", "/usr/bin/resona"]).trim(), name);
        assert.equal(JSON.parse(fs.readFileSync("/usr/lib/Resona/build.json", "utf8")).version, stable);
      }
      console.log("Arch install, upgrade, reinstall, preview-to-stable and desktop registration passed");
    }
  }
} finally {
  if (installed) run("pacman", ["-R", "--noconfirm", name]);
  assert.equal(fs.existsSync("/usr/bin/resona"), false);
  fs.rmSync(root, { recursive: true, force: true });
}
