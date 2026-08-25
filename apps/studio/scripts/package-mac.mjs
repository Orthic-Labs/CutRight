import { chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, symlinkSync, writeFileSync } from "node:fs";
import { join, resolve, sep } from "node:path";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { resolveTargetRoot } from "@rightkit/release/cargo-target.mjs";

const appRoot = fileURLToPath(new URL("..", import.meta.url));
const repoRoot = resolve(appRoot, "../..");
const version = JSON.parse(await (await import("node:fs/promises")).readFile(join(appRoot, "package.json"), "utf8")).version;
// A managed build owns the target root, so the bundle is not under src-tauri,
// and CARGO_TARGET_DIR can be set but stale on a broker-managed host. Cargo's
// own `cargo metadata` target_directory is the sole source of truth for
// locating build output; it never reads the env var to find it.
const cargoTargetRoot = resolveTargetRoot(join(appRoot, "src-tauri", "Cargo.toml"));
const target = join(cargoTargetRoot, "universal-apple-darwin/release/bundle");
const macos = join(target, "macos");
const dmgDir = join(target, "dmg");
const app = join(macos, "CutRight Studio.app");
const updater = join(macos, "CutRight Studio.app.tar.gz");
const dmgName = `CutRight_Studio_${version}_universal.dmg`;
const identity = "Developer ID Application: Adrian D'souza (6KLGD3LLKF)";
const entitlements = join(appRoot, "src-tauri/Entitlements.plist");

function run(command, args, env = {}, unset = []) {
  const childEnv = { ...process.env, ...env };
  for (const name of unset) delete childEnv[name];
  const result = spawnSync(command, args, { cwd: repoRoot, stdio: "inherit", env: childEnv });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

function signNested(bundle) {
  const files = spawnSync("find", [bundle, "-type", "f"], { encoding: "utf8" }).stdout.trim().split("\n").filter(Boolean).reverse();
  for (const file of files) {
    const kind = spawnSync("file", [file], { encoding: "utf8" }).stdout;
    if (kind.includes("Mach-O")) {
      run("codesign", ["--force", "--options", "runtime", "--timestamp", "--sign", identity, file]);
    }
  }
}

function refreshPackHashes(bundle) {
  const packsRoot = join(bundle, "Contents/Resources/packs");
  for (const pack of ["media", "speech", "verifier"]) {
    const packRoot = resolve(packsRoot, pack);
    const manifestPath = join(packRoot, "PACK.json");
    const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
    const shipped = [];
    for (const item of manifest.files ?? []) {
      const payload = resolve(packRoot, item.path);
      if (!payload.startsWith(`${packRoot}${sep}`)) throw new Error(`pack path escapes root: ${item.path}`);
      if (!existsSync(payload)) continue;
      const hash = spawnSync("shasum", ["-a", "256", payload], { encoding: "utf8" });
      if (hash.status !== 0) throw new Error(`failed to hash signed pack payload: ${item.path}`);
      item.sha256 = hash.stdout.trim().split(/\s+/)[0];
      item.bytes = statSync(payload).size;
      shipped.push(item);
    }
    manifest.files = shipped;
    writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  }
}

function buildSidecars() {
  const binDir = join(appRoot, "src-tauri/bin");
  const triples = ["aarch64-apple-darwin", "x86_64-apple-darwin"];
  const sidecars = [
    { name: "videoctl", package: "videoctl" },
    { name: "cutright-mcp", package: "video-agent" },
    { name: "cutrightd", package: "video-daemon" },
  ];
  mkdirSync(binDir, { recursive: true });
  for (const triple of triples) {
    // One cargo invocation per triple builds all three sidecars; the broker
    // owns the target root so binaries are resolved from cargo's own
    // artifact stream instead of a predicted --target-dir path.
    const args = ["build", "--release", "--locked", "--target", triple, "--message-format=json-render-diagnostics"];
    for (const sidecar of sidecars) args.push("-p", sidecar.package, "--bin", sidecar.name);
    const result = spawnSync("cargo", args, {
      cwd: repoRoot, encoding: "utf8", maxBuffer: 512 * 1024 * 1024, stdio: ["inherit", "pipe", "inherit"],
    });
    if (result.error) throw result.error;
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
      const destination = join(binDir, `${sidecar.name}-${triple}`);
      copyFileSync(source, destination);
      chmodSync(destination, 0o755);
    }
  }
  for (const sidecar of sidecars) {
    run("lipo", ["-create",
      join(binDir, `${sidecar.name}-aarch64-apple-darwin`),
      join(binDir, `${sidecar.name}-x86_64-apple-darwin`),
      "-output", join(binDir, `${sidecar.name}-universal-apple-darwin`)]);
  }
}

const key = spawnSync("security", ["find-generic-password", "-a", process.env.USER, "-s", "rightsuite-updater-key", "-w"], { encoding: "utf8" });
if (key.status !== 0 || !key.stdout.trim()) throw new Error("shared updater key unavailable");
buildSidecars();
run("pnpm", ["--dir", "apps/studio", "exec", "tauri", "build", "--target", "universal-apple-darwin", "--bundles", "app,dmg", "--config", '{"bundle":{"createUpdaterArtifacts":false}}'], {
  APPLE_SIGNING_IDENTITY: identity,
  TAURI_SIGNING_PRIVATE_KEY: key.stdout.trim(), TAURI_SIGNING_PRIVATE_KEY_PASSWORD: "",
}, ["APPLE_API_ISSUER", "APPLE_API_KEY", "APPLE_API_KEY_PATH", "APPLE_ID", "APPLE_PASSWORD", "APPLE_TEAM_ID"]);
if (!existsSync(app)) throw new Error("universal app artifact missing");
signNested(app);
refreshPackHashes(app);
run("codesign", ["--force", "--options", "runtime", "--timestamp", "--entitlements", entitlements, "--sign", identity, app]);
for (const binary of ["cutright-studio", "cutright-macos-media", "videoctl", "cutright-mcp", "cutrightd"].map(name => join(app, "Contents/MacOS", name))) {
  if (!existsSync(binary)) throw new Error(`universal binary missing: ${binary}`);
  run("lipo", [binary, "-verify_arch", "arm64", "x86_64"]);
}
run("codesign", ["--verify", "--deep", "--strict", "--verbose=2", app]);
run("ditto", ["-c", "-k", "--keepParent", app, `${app}.zip`]);
run("xcrun", ["notarytool", "submit", `${app}.zip`, "--keychain-profile", "apple-dev-notary", "--wait"]);
run("xcrun", ["stapler", "staple", app]);
run("xcrun", ["stapler", "validate", app]);
run("spctl", ["--assess", "--type", "execute", "--verbose", app]);
run("pnpm", ["--dir", "apps/studio", "exec", "right-release", "create-mac-updater", "--app", app, "--output", updater, "--cwd", appRoot], { TAURI_SIGNING_PRIVATE_KEY: key.stdout.trim(), TAURI_SIGNING_PRIVATE_KEY_PASSWORD: "" });
if (!existsSync(updater)) throw new Error("signed updater artifact missing");
mkdirSync(dmgDir, { recursive: true });
const builtDmg = join(dmgDir, `CutRight Studio_${version}_universal.dmg`);
const stage = mkdtempSync(join(tmpdir(), "cutright-dmg-"));
try {
  run("ditto", [app, join(stage, "CutRight Studio.app")]);
  symlinkSync("/Applications", join(stage, "Applications"));
  rmSync(builtDmg, { force: true });
  run("hdiutil", ["create", "-fs", "HFS+", "-volname", "CutRight Studio", "-srcfolder", stage, "-ov", "-format", "UDZO", builtDmg]);
} finally {
  rmSync(stage, { recursive: true, force: true });
}
if (process.argv.includes("--notarize")) {
  run("codesign", ["--force", "--timestamp", "--sign", identity, builtDmg]);
  run("xcrun", ["notarytool", "submit", builtDmg, "--keychain-profile", "apple-dev-notary", "--wait"]);
  run("xcrun", ["stapler", "staple", builtDmg]);
  run("xcrun", ["stapler", "validate", builtDmg]);
}
run("pnpm", ["--dir", "apps/studio", "exec", "right-release", "mirror-root-artifact", "--file", builtDmg, "--package-root", appRoot, "--name", dmgName]);
