// Builds the real StatGlass app, running on its mock transport, into
// public/demo/ so the site can embed it (the landing page's live demo).
// Skips the build when nothing it depends on changed since the last one.
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

const repo = fileURLToPath(new URL("../..", import.meta.url));
const outDir = "docs-site/public/demo";
// Outside public/ so the stamp is never copied into the deployed site.
const stampFile = fileURLToPath(
  new URL("../node_modules/.cache/build-demo.stamp", import.meta.url)
);
const viteArgs = [
  "build",
  "--base",
  "/demo/",
  "--outDir",
  outDir,
  "--emptyOutDir",
];
// Mock mode makes ?window= and ?scenario= work in a production build.
const viteEnv = { VITE_TRANSPORT: "mock" };
const inputs = [
  "src",
  "index.html",
  "vite.config.ts",
  "package.json",
  "bun.lock",
];

if (!existsSync(`${repo}/node_modules/.bin/vite`)) {
  console.error(
    "build-demo: the app's dependencies are missing. Run `bun install` at the repo root (make install does both)."
  );
  process.exit(1);
}

// Tracked blobs plus the contents of anything modified or untracked, so a
// dirty working tree rebuilds too.
function inputHash() {
  const git = (...args) =>
    execFileSync("git", args, { cwd: repo, encoding: "utf8" });
  const hash = createHash("sha256");
  // This script and the build it runs, so changing either rebuilds.
  hash.update(readFileSync(fileURLToPath(import.meta.url)));
  hash.update(JSON.stringify([viteArgs, viteEnv]));
  hash.update(git("ls-files", "-s", "--", ...inputs));
  const dirty = git(
    "ls-files",
    "-m",
    "-o",
    "--exclude-standard",
    "--",
    ...inputs
  )
    .split("\n")
    .filter(Boolean)
    .sort();
  for (const file of dirty) {
    hash.update(file);
    if (existsSync(`${repo}/${file}`))
      hash.update(readFileSync(`${repo}/${file}`));
  }
  return hash.digest("hex");
}

const hash = inputHash();
if (existsSync(stampFile) && readFileSync(stampFile, "utf8") === hash) {
  console.log("build-demo: up to date");
  process.exit(0);
}

console.log(
  "build-demo: building the app with the mock transport into public/demo"
);
const result = spawnSync(`${repo}/node_modules/.bin/vite`, viteArgs, {
  cwd: repo,
  stdio: "inherit",
  env: { ...process.env, ...viteEnv },
});
if (result.status !== 0) {
  console.error("build-demo: vite build failed");
  process.exit(result.status ?? 1);
}
mkdirSync(dirname(stampFile), { recursive: true });
writeFileSync(stampFile, hash);
