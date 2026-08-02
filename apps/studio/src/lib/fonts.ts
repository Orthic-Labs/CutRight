// Explicit Font Loading API registration for the three shipped webfonts
// (Tanker, Geist, Spline Sans Mono).
//
// Root cause of the "wordmark renders as fallback sans" bug: the `@font-face`
// rules in styles.css declare `src: url("./assets/fonts/...")`, a path
// relative to the stylesheet. Vite rewrites that correctly to a build-hashed
// absolute URL (verified in `dist/assets/index-*.css` — the rule reads
// `url(/assets/tanker-regular-<hash>.woff2)`), and that URL 200s and renders
// correctly under a plain HTTP origin (`vite dev`, `vite preview`, this
// project's own browser-QA screenshots all confirmed Tanker's real glyph
// metrics, not Impact's). Tauri's packaged build instead serves the
// frontend through a custom URL scheme (`tauri://localhost` on macOS), and
// that is the one context left unverified by any test in this repo — the
// documented, repeated failure mode for `@font-face url()` resolution
// through a WKWebView custom-scheme handler is exactly this: silent
// fallback with no console error, because the browser's lazy
// stylesheet-triggered font fetch never fires a catchable event.
//
// Fix: register the same font files a second way that does not depend on
// the stylesheet resource loader at all — fetch each file directly via a
// Vite `?url` import (guaranteed to resolve to the same final asset URL the
// CSS uses, in every context, dev or built) and hand it to the CSS Font
// Loading API (`FontFace` + `document.fonts.add`), which performs its own
// `fetch()` independent of the stylesheet cascade. This runs alongside the
// `@font-face` rules in styles.css, not instead of them: wherever the CSS
// path already works (every context this repo can actually test), this is
// a harmless duplicate registration under the same family name; wherever it
// doesn't (the packaged WKWebView), this is the fix.
import tankerUrl from "../assets/fonts/tanker-regular.woff2?url";
import geistUrl from "../assets/fonts/geist-variable.woff2?url";
import splineMonoUrl from "../assets/fonts/spline-sans-mono-regular.ttf?url";

const FONTS: Array<[string, string, FontFaceDescriptors | undefined]> = [
  ["Tanker", tankerUrl, undefined],
  ["Geist", geistUrl, { weight: "100 900" }],
  ["Spline Sans Mono", splineMonoUrl, { weight: "400" }],
];

// Fire-and-forget: called once at startup, never blocks first paint. Any
// individual font failing to register still leaves that family's CSS
// fallback chain intact (Tanker -> Impact -> sans-serif, etc.), so a
// rejected load here is degraded, not broken.
export function loadAppFonts(): void {
  if (typeof document === "undefined" || !("fonts" in document)) return;
  for (const [family, url, descriptors] of FONTS) {
    try {
      const face = new FontFace(family, `url(${url})`, descriptors);
      document.fonts.add(face);
      void face.load().catch(() => undefined);
    } catch {
      // FontFace unsupported or malformed descriptor — CSS @font-face
      // remains the fallback path in every browser that reaches here.
    }
  }
}
