import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const bundleDir = join(root, "src-tauri", "target", "release", "bundle");
const releaseBinDir = join(root, "src-tauri", "target", "release");

const { version } = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));

const platforms = {
  win32: {
    dir: "windows",
    portableName: `easy-read_${version}_x64.exe`,
    portableSource: join(releaseBinDir, "easy-read.exe"),
    installerSources: [
      { from: join(bundleDir, "msi"), match: /\.msi$/i },
      { from: join(bundleDir, "nsis"), match: /-setup\.exe$/i },
    ],
  },
  linux: {
    dir: "linux",
    portableName: `easy-read_${version}_amd64.AppImage`,
    portableSourceDir: join(bundleDir, "appimage"),
    portableMatch: /\.AppImage$/i,
    installerSources: [
      { from: join(bundleDir, "deb"), match: /\.deb$/i },
      { from: join(bundleDir, "rpm"), match: /\.rpm$/i },
    ],
  },
  darwin: {
    dir: "macos",
    installerSources: [
      { from: join(bundleDir, "dmg"), match: /\.dmg$/i },
    ],
  },
};

const cfg = platforms[process.platform];
if (!cfg) {
  console.error(`unsupported platform: ${process.platform}`);
  process.exit(1);
}

const outDir = join(root, "releases", cfg.dir);
rmSync(outDir, { recursive: true, force: true });
const installerDir = join(outDir, "installers");
const portableDir = join(outDir, "portable");
mkdirSync(installerDir, { recursive: true });
if (cfg.portableName) mkdirSync(portableDir, { recursive: true });

let copied = 0;

const copyMatching = (sourceDir, pattern, destDir) => {
  if (!existsSync(sourceDir)) return;
  for (const name of readdirSync(sourceDir)) {
    if (!pattern.test(name)) continue;
    const dest = join(destDir, name);
    copyFileSync(join(sourceDir, name), dest);
    console.log(`  ${dest}`);
    copied++;
  }
};

console.log("staging installers:");
for (const src of cfg.installerSources) {
  copyMatching(src.from, src.match, installerDir);
}

if (cfg.portableSource) {
  console.log("staging portable:");
  if (existsSync(cfg.portableSource)) {
    const dest = join(portableDir, cfg.portableName);
    copyFileSync(cfg.portableSource, dest);
    console.log(`  ${dest}`);
    copied++;
  } else {
    console.warn(`  portable binary not found at ${cfg.portableSource}`);
  }
} else if (cfg.portableSourceDir) {
  console.log("staging portable:");
  if (existsSync(cfg.portableSourceDir)) {
    for (const name of readdirSync(cfg.portableSourceDir)) {
      if (!cfg.portableMatch.test(name)) continue;
      const dest = join(portableDir, cfg.portableName);
      copyFileSync(join(cfg.portableSourceDir, name), dest);
      console.log(`  ${dest}`);
      copied++;
      break;
    }
  }
}

if (copied === 0) {
  console.error("no artifacts copied - did `tauri build` succeed?");
  process.exit(1);
}

console.log(`\ndone. ${copied} artifact(s) staged in ${outDir}`);
