import { copyFileSync, existsSync, mkdirSync, readFileSync, renameSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { resolveTargetRoot } from "@rightkit/release/cargo-target.mjs";

const appRoot = fileURLToPath(new URL("..", import.meta.url));
const repoRoot = resolve(appRoot, "../..");
const version = JSON.parse(readFileSync(join(appRoot, "package.json"), "utf8")).version;
// A managed build owns the target root, so src-tauri/target is not a valid
// fallback, and CARGO_TARGET_DIR can be set but stale on a broker-managed
// host. Cargo's own `cargo metadata` target_directory is the sole source of
// truth for locating build output; it never reads the env var to find it.
const targetDir = resolveTargetRoot(join(appRoot, "src-tauri", "Cargo.toml"));
const bundleDir = join(targetDir, "release/bundle/nsis");
const sourceInstaller = join(bundleDir, `CutRight Studio_${version}_x64-setup.exe`);
const installer = join(bundleDir, `CutRight_Studio_${version}_x64-setup.exe`);
const triple = "x86_64-pc-windows-msvc";

function run(command, args) {
  const result = spawnSync(command, args, { cwd: repoRoot, stdio: "inherit", env: process.env });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

const binDir = join(appRoot, "src-tauri/bin");
mkdirSync(binDir, { recursive: true });
const sidecars = [
  { name: "videoctl", package: "videoctl" },
  { name: "cutright-mcp", package: "video-agent" },
  { name: "cutrightd", package: "video-daemon" },
];
// One cargo invocation builds all three sidecars; the broker owns the target
// root, so binaries are resolved from cargo's own artifact stream rather than
// a predicted --target-dir path.
const args = ["build", "--release", "--locked", "--target", triple, "--message-format=json-render-diagnostics"];
for (const sidecar of sidecars) args.push("-p", sidecar.package, "--bin", sidecar.name);
const result = spawnSync("cargo", args, {
  cwd: repoRoot, encoding: "utf8", maxBuffer: 512 * 1024 * 1024, env: process.env,
  stdio: ["inherit", "pipe", "inherit"],
});
if (result.status !== 0) process.exit(result.status ?? 1);

const produced = new Map();
for (const line of (result.stdout ?? "").split("\n")) {
  if (!line.startsWith("{")) continue;
  let message;
  try { message = JSON.parse(line); } catch { continue; }
  if (message.reason !== "compiler-artifact" || !message.executable) continue;
  produced.set(message.target?.name, message.executable);
}
for (const sidecar of sidecars) {
  const source = produced.get(sidecar.name);
  if (!source) throw new Error(`sidecar build produced no executable named ${sidecar.name}`);
  copyFileSync(source, join(binDir, `${sidecar.name}-${triple}.exe`));
}

run("pnpm", ["--dir", "apps/studio", "exec", "tauri", "build", "--bundles", "nsis", "--config",
  '{"bundle":{"externalBin":["bin/videoctl","bin/cutright-mcp","bin/cutrightd"]}}']);
if (!existsSync(sourceInstaller)) throw new Error(`Windows installer missing: ${sourceInstaller}`);
rmSync(installer, { force: true });
renameSync(sourceInstaller, installer);
