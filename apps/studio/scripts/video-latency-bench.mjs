#!/usr/bin/env node
// CR-F-B8-003 supporting evidence: measures HTMLVideoElement playback
// latency the same way CutRight Studio's SourcesMode component drives a
// <video> element (direct `src` assignment, direct `currentTime` seeks, no
// native <controls>). Runs a real headless Chrome via the Chrome DevTools
// Protocol using the zero-dependency raw-WebSocket technique already used by
// tools/skills/qa/scripts/qa.mjs (findBrowser/cdpConnect/runtimeEval below
// are a trimmed adaptation of that file — MINIMIZE rung: REUSE, no new
// dependency added).
//
// Usage:
//   pnpm exec node scripts/video-latency-bench.mjs [--iterations 20] [--media <path>]
//
// Prints one JSON object to stdout: { loadToFirstFrameMs, seekToFrameMs,
// accessibility, meta }.
import { spawn } from "node:child_process";
import { createConnection } from "node:net";
import { randomBytes } from "node:crypto";
import { existsSync, mkdirSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const DEFAULT_MEDIA = resolve(scriptDir, "../../../.audit/cutaway-finish/native/source.mp4");

function parseArgs(argv) {
  const args = { iterations: 20, media: DEFAULT_MEDIA, cdpPort: 9333 };
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === "--iterations") args.iterations = Number(argv[++i]);
    else if (argv[i] === "--media") args.media = resolve(argv[++i]);
    else if (argv[i] === "--cdp-port") args.cdpPort = Number(argv[++i]);
  }
  return args;
}

function findBrowser() {
  const override = process.env.CHROME_PATH || process.env.QA_BROWSER;
  if (override) {
    if (!existsSync(override)) throw new Error(`CHROME_PATH set but not found: ${override}`);
    return override;
  }
  const candidates = process.platform === "win32"
    ? [
        join(process.env.ProgramFiles || "C:\\Program Files", "Google\\Chrome\\Application\\chrome.exe"),
        join(process.env["ProgramFiles(x86)"] || "C:\\Program Files (x86)", "Google\\Chrome\\Application\\chrome.exe"),
      ]
    : [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "/usr/bin/google-chrome",
        "/usr/bin/chromium",
      ];
  const found = candidates.find((p) => existsSync(p));
  if (!found) throw new Error("No Chrome/Chromium/Edge executable found. Set CHROME_PATH.");
  return found;
}

function wait(ms) { return new Promise((r) => setTimeout(r, ms)); }

async function waitForHttp(url, timeoutMs = 30000) {
  const started = Date.now();
  let last = "";
  while (Date.now() - started < timeoutMs) {
    try {
      const res = await fetch(url);
      if (res.ok) return;
      last = `${res.status} ${res.statusText}`;
    } catch (error) { last = String(error?.message ?? error); }
    await wait(200);
  }
  throw new Error(`Timed out waiting for ${url}: ${last}`);
}

// --- Minimal CDP client over a raw WebSocket (no `ws` dependency — adapted
// from tools/skills/qa/scripts/qa.mjs's makeFrame/readFrames/cdpConnect). ---
function makeFrame(text) {
  const payload = Buffer.from(text);
  let header;
  if (payload.length < 126) header = Buffer.from([0x81, 0x80 | payload.length]);
  else {
    header = Buffer.alloc(4);
    header[0] = 0x81; header[1] = 0x80 | 126; header.writeUInt16BE(payload.length, 2);
  }
  const mask = randomBytes(4);
  const masked = Buffer.alloc(payload.length);
  for (let i = 0; i < payload.length; i += 1) masked[i] = payload[i] ^ mask[i % 4];
  return Buffer.concat([header, mask, masked]);
}
function readFrames(buffer) {
  const frames = [];
  let offset = 0;
  while (buffer.length - offset >= 2) {
    const b1 = buffer[offset + 1];
    let len = b1 & 0x7f;
    let pos = offset + 2;
    if (len === 126) { if (buffer.length - pos < 2) break; len = buffer.readUInt16BE(pos); pos += 2; }
    else if (len === 127) { if (buffer.length - pos < 8) break; len = Number(buffer.readBigUInt64BE(pos)); pos += 8; }
    if (buffer.length - pos < len) break;
    frames.push({ opcode: buffer[offset] & 0x0f, text: buffer.subarray(pos, pos + len).toString("utf8") });
    offset = pos + len;
  }
  return { frames, rest: buffer.subarray(offset) };
}
function cdpConnect(wsUrl) {
  return new Promise((resolveConnect, reject) => {
    const u = new URL(wsUrl);
    const socket = createConnection({ host: u.hostname, port: Number(u.port) || 80 });
    const key = randomBytes(16).toString("base64");
    const callbacks = new Map();
    let id = 0, handshaken = false, buffer = Buffer.alloc(0);
    socket.setNoDelay(true);
    socket.on("connect", () => {
      socket.write(`GET ${u.pathname}${u.search} HTTP/1.1\r\nHost: ${u.host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: ${key}\r\nSec-WebSocket-Version: 13\r\n\r\n`);
    });
    socket.on("data", (chunk) => {
      buffer = Buffer.concat([buffer, chunk]);
      if (!handshaken) {
        const idx = buffer.indexOf("\r\n\r\n");
        if (idx < 0) return;
        if (!buffer.subarray(0, idx).toString("utf8").includes("101")) {
          reject(new Error("CDP WebSocket handshake failed")); socket.destroy(); return;
        }
        handshaken = true;
        buffer = buffer.subarray(idx + 4);
        resolveConnect({
          send(method, params = {}) {
            const callId = ++id;
            socket.write(makeFrame(JSON.stringify({ id: callId, method, params })));
            return new Promise((res, rej) => callbacks.set(callId, { res, rej }));
          },
          close() { socket.end(); },
        });
      }
      if (!handshaken || buffer.length === 0) return;
      const parsed = readFrames(buffer);
      buffer = parsed.rest;
      for (const frame of parsed.frames) {
        if (frame.opcode !== 1) continue;
        const msg = JSON.parse(frame.text);
        const cb = callbacks.get(msg.id);
        if (!cb) continue;
        callbacks.delete(msg.id);
        if (msg.error) cb.rej(new Error(msg.error.message));
        else cb.res(msg.result);
      }
    });
    socket.on("error", reject);
  });
}
async function runtimeEval(client, expression, timeoutMs = 60000) {
  const result = await client.send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true, timeout: timeoutMs });
  if (result.exceptionDetails) throw new Error(result.exceptionDetails.exception?.description || result.exceptionDetails.text || "eval failed");
  return result.result.value;
}

function percentile(sorted, p) {
  if (sorted.length === 0) return null;
  const idx = Math.min(sorted.length - 1, Math.ceil((p / 100) * sorted.length) - 1);
  return sorted[Math.max(0, idx)];
}
function stats(samples) {
  const sorted = [...samples].sort((a, b) => a - b);
  return {
    samples: sorted,
    p50_ms: percentile(sorted, 50),
    p95_ms: percentile(sorted, 95),
    min_ms: sorted[0] ?? null,
    max_ms: sorted[sorted.length - 1] ?? null,
  };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!existsSync(args.media)) {
    throw new Error(`Media file not found: ${args.media} (pass --media <path to an mp4>)`);
  }
  const browser = findBrowser();
  const harnessUrl = pathToFileURL(join(scriptDir, "video-latency-harness.html")).href;
  const mediaUrl = pathToFileURL(args.media).href;
  const profile = resolve(scriptDir, "../.cache/qa-cdp-profile-video-latency");
  mkdirSync(profile, { recursive: true });

  const chrome = spawn(browser, [
    "--headless=new", "--disable-gpu", "--no-default-browser-check", "--no-first-run",
    `--remote-debugging-port=${args.cdpPort}`, `--user-data-dir=${profile}`,
    "--window-size=800,600", "--allow-file-access-from-files", "about:blank",
  ], { windowsHide: true, stdio: "ignore" });

  let client;
  try {
    await waitForHttp(`http://127.0.0.1:${args.cdpPort}/json/version`, 30000);
    const targets = await (await fetch(`http://127.0.0.1:${args.cdpPort}/json/list`)).json();
    const target = targets.find((t) => t.type === "page" && t.webSocketDebuggerUrl);
    if (!target) throw new Error("No CDP page target found.");
    client = await cdpConnect(target.webSocketDebuggerUrl);
    await client.send("Runtime.enable");
    await client.send("Page.enable");
    await client.send("Page.navigate", { url: harnessUrl });
    await wait(500);

    const seekOffsets = [0.1, 0.3, 0.5, 0.7, 0.9];
    const result = await runtimeEval(
      client,
      `window.__runLatencyBench(${JSON.stringify(mediaUrl)}, ${args.iterations}, ${JSON.stringify(seekOffsets)})`,
      120000,
    );

    const report = {
      schema: "cutright.video-latency-bench.v1",
      generated_at: new Date().toISOString(),
      iterations: args.iterations,
      player: "HTMLVideoElement (retain_html_video candidate, CR-F-B8-003)",
      media_path: args.media,
      browser,
      load_to_first_frame_ms: stats(result.load),
      seek_to_frame_ms: stats(result.seek),
      accessibility: {
        keyboard_controls: "no `controls` attribute on Sources/Compare <video>; play/pause and scrub are custom React elements (button + input[type=range]) which ARE natively keyboard-operable; FinalsMode's <video controls> exposes native keyboard controls.",
        caption_track_support: result.a11y.hasCaptionTrack,
        accessible_name: result.a11y.accessibleName,
        facts_only: true,
      },
    };
    console.log(JSON.stringify(report, null, 2));
  } finally {
    try { client?.close(); } catch {}
    try { chrome.kill(); } catch {}
    try { rmSync(profile, { recursive: true, force: true }); } catch {}
  }
}

main().catch((error) => {
  console.error(`[video-latency-bench] ${error.stack || error.message}`);
  process.exit(1);
});
