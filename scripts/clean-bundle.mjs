import { rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const bundleDir = resolve(here, "..", "src-tauri", "target", "release", "bundle");

rmSync(bundleDir, { recursive: true, force: true });
console.log(`cleaned: ${bundleDir}`);
