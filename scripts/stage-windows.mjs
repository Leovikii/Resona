// SPDX-License-Identifier: GPL-3.0-only
import { copyFileSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { buildIdentity, readProjectVersion } from "./release-channel.mjs";
const { version } = readProjectVersion();
const identity = buildIdentity(version, process.env.RESONA_BUILD_NUMBER, process.env.RESONA_BUILD_ATTEMPT, process.env.RESONA_BUILD_SHA);
const source = resolve("src-tauri/target/release/bundle/nsis");
const installers = readdirSync(source).filter((name) => name.endsWith(".exe"));
if (installers.length !== 1) throw new Error("Exactly one signed NSIS installer is required");
const output = resolve("src-tauri/target/delivery");
mkdirSync(output, { recursive: true });
const name = `Resona_${version}_${identity.id}_windows_x86_64-setup.exe`;
copyFileSync(resolve(source, installers[0]), resolve(output, name));
copyFileSync(resolve(source, `${installers[0]}.sig`), resolve(output, `${name}.sig`));
writeFileSync(resolve(output, "build.json"), `${JSON.stringify({ version, ...identity }, null, 2)}\n`);
