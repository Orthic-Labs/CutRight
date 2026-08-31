import { readFileSync } from "node:fs";
import path from "node:path";

const root = path.resolve(new URL(".", import.meta.url).pathname);
const pkg = JSON.parse(readFileSync(path.join(root, "package.json"), "utf8"));
const version = pkg.version;
const dmg = `CutRight_Studio_${version}_universal.dmg`;
const updater = "src-tauri/target/universal-apple-darwin/release/bundle/macos/CutRight Studio.app.tar.gz";
const windowsInstaller = `src-tauri/target/release/bundle/nsis/CutRight_Studio_${version}_x64-setup.exe`;

export default {
  schema: 1,
  app: "cutright",
  version,
  distribution: { provider: "github-releases", repository: "Orthic-Labs/CutRight" },
  hostedWorkflows: "right-git-ci-only",
  packageManager: "pnpm",
  deps: { workdirs: ["."] },
  checks: ["typecheck", "test"],
  buildInputs: { include: ["right-release.config.mjs", "package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml", "legal/**", "scripts/**", "src/**", "public/**", "src-tauri/**", "../../Cargo.toml", "../../Cargo.lock", "../../rust-toolchain.toml", "../../crates/**", "../../sidecars/**", "../../docs/legal/**", "../../runtime/packs/**", "../../skills/**", "../../release/v2/THIRD-PARTY-NOTICES.md", "../../LICENSE"], exclude: ["**/node_modules/**", "**/target/**", "**/*.test.*", "**/tests/**", ".right-release/**"] },
  targets: {
    mac: {
      signed: true,
      preflight: { minFreeGb: 20, executables: ["codesign", "hdiutil", "lipo", "minisign", "security", "spctl", "tar", "xcrun"] },
      package: { cmd: "pnpm", args: ["run", "rightkit:package:mac"] },
      artifacts: [dmg],
      hardening: [dmg, updater],
      installer: { artifacts: [{ file: dmg, key: "cutright/installers/mac/current/CutRight_Studio_universal.dmg" }] },
      updater: { artifacts: [
        { file: updater, signature: `${updater}.sig`, platform: "darwin-aarch64", key: "cutright/updates/mac/current/CutRight_Studio.app.tar.gz" },
        { file: updater, signature: `${updater}.sig`, platform: "darwin-x86_64", key: "cutright/updates/mac/current/CutRight_Studio.app.tar.gz" }
      ] }
    },
    win: {
      signed: true,
      preflight: { minFreeGb: 20, executables: ["cargo", "pnpm", "signtool"] },
      package: { cmd: "pnpm", args: ["run", "rightkit:package:win"] },
      artifacts: ["src-tauri/target/release/bundle/nsis"],
      sign: { files: [windowsInstaller] },
      hardening: ["src-tauri/target/release/bundle/nsis"],
      installer: { artifacts: [{ file: windowsInstaller, key: "cutright/installers/windows/current/CutRight_Studio_x64-setup.exe" }] },
      updater: { artifacts: [
        { file: windowsInstaller, signature: `${windowsInstaller}.sig`, platform: "windows-x86_64", key: "cutright/updates/windows/current/CutRight_Studio_x64-setup.exe" }
      ] }
    }
  }
};
