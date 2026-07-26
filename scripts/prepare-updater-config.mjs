// SPDX-License-Identifier: GPL-3.0-only

import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const publicKey = process.env.RESONA_UPDATER_PUBLIC_KEY?.trim();
if (!publicKey) {
  throw new Error("RESONA_UPDATER_PUBLIC_KEY is required for a signed release build");
}

const targetDirectory = resolve(projectRoot, "src-tauri", "target");
const configPath = resolve(targetDirectory, "updater.config.json");
mkdirSync(targetDirectory, { recursive: true });
writeFileSync(
  configPath,
  `${JSON.stringify({
    plugins: {
      updater: {
        endpoints: [],
        pubkey: publicKey,
      },
    },
  }, null, 2)}\n`,
);
process.stdout.write(`${configPath}\n`);
