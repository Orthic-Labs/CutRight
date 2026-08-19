import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const studioDir = resolve(scriptDir, "..");
const output = join(studioDir, "src-tauri/icons");
const source = resolve(process.argv[2] ?? join(output, "_source.png"));
const transparentMark = resolve(process.argv[3] ?? join(output, "_mark-transparent.png"));
const trayOutput = join(output, "tray");

function dimensions(file) {
  const probe = JSON.parse(execFileSync("ffprobe", [
    "-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height",
    "-of", "json", file,
  ], { encoding: "utf8" })).streams[0];
  if (!probe || probe.width !== probe.height || probe.width < 512) {
    throw new Error(`${file} must be square & at least 512 px`);
  }
  return probe;
}

dimensions(source);
dimensions(transparentMark);
mkdirSync(output, { recursive: true });
mkdirSync(trayOutput, { recursive: true });

const master = join(output, "icon.png");
if (source !== master) copyFileSync(source, master);
execFileSync("pnpm", ["exec", "tauri", "icon", master, "--output", output], {
  cwd: studioDir,
  stdio: "inherit",
});
rmSync(join(output, "android"), { recursive: true, force: true });
rmSync(join(output, "ios"), { recursive: true, force: true });

const tileBase64 = readFileSync(master).toString("base64");
writeFileSync(
  join(output, "icon.svg"),
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" role="img" aria-label="CutRight Studio"><image width="1024" height="1024" href="data:image/png;base64,${tileBase64}"/></svg>\n`,
);

execFileSync(
  "pnpm",
  ["exec", "tauri", "icon", transparentMark, "--output", trayOutput,
    "--png", "16", "--png", "20", "--png", "24", "--png", "32",
    "--png", "40", "--png", "48", "--png", "64", "--png", "128"],
  { cwd: studioDir, stdio: "inherit" },
);
for (const size of [16, 20, 24, 32, 40, 48, 64, 128]) {
  renameSync(join(trayOutput, `${size}x${size}.png`), join(trayOutput, `tray-icon-white-${size}.png`));
}

const markBase64 = readFileSync(transparentMark).toString("base64");
const traySvg = (filter = "") => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" role="img" aria-label="CutRight"><image width="1024" height="1024"${filter} href="data:image/png;base64,${markBase64}"/></svg>\n`;
writeFileSync(join(trayOutput, "tray-icon-white.svg"), traySvg());
writeFileSync(join(trayOutput, "tray-icon-template.svg"), traySvg(' style="filter:brightness(0)"'));
copyFileSync(join(trayOutput, "tray-icon-white-16.png"), join(trayOutput, "trayTemplate.png"));
copyFileSync(join(trayOutput, "tray-icon-white-32.png"), join(trayOutput, "trayTemplate@2x.png"));

console.log(`Generated ${output}`);
