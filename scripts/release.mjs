import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const npx = process.platform === "win32" ? "npx.cmd" : "npx";
const rustFlags = [
  process.env.USERPROFILE && `--remap-path-prefix=${process.env.USERPROFILE}=~`,
  `--remap-path-prefix=${root}=.`,
].filter(Boolean);
const encodedRustFlags = [
  process.env.CARGO_ENCODED_RUSTFLAGS,
  ...rustFlags,
].filter(Boolean).join("\x1f");

const run = (command, args, options = {}) => {
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: "inherit",
    ...options,
  });

  if (result.error) {
    console.error(result.error.message);
    process.exit(1);
  }

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
};

run(process.execPath, [join(here, "clean-bundle.mjs")]);
run(npx, ["tauri", "build"], {
  env: {
    ...process.env,
    CARGO_ENCODED_RUSTFLAGS: encodedRustFlags,
    NO_STRIP: "1",
  },
});
run(process.execPath, [join(here, "stage-release.mjs")]);
