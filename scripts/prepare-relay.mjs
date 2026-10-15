// Builds the ilot-hook relay and the ilot-cli tool and places them where Tauri bundles
// them next to ilot.exe (bundle.externalBin needs the target triple in the file name).
// Usage: node scripts/prepare-relay.mjs debug|release

import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const mode = process.argv[2] === "release" ? "release" : "debug";
const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = ["build", "-p", "ilot-hook", "-p", "ilot-cli", ...(mode === "release" ? ["--release"] : [])];
execFileSync("cargo", args, { cwd: root, stdio: "inherit" });
const binaries = join(root, "src-tauri", "binaries");
mkdirSync(binaries, { recursive: true });
for (const name of ["ilot-hook", "ilot-cli"]) {
  copyFileSync(join(root, "target", mode, `${name}.exe`), join(binaries, `${name}-x86_64-pc-windows-msvc.exe`));
}
