// Shared canvas + typography constants for every composition. One fixed
// canvas (matches the registry's pre-Remotion ffmpeg preview size) so
// footprint math stays comparable across the whole effect library.
export const CANVAS_WIDTH = 1280;
export const CANVAS_HEIGHT = 720;
export const FPS = 30;
// 1.5s @ 30fps — the ffmpeg-era motion preview length this package
// preserves so preview durations did not change when the renderer did.
export const DURATION_IN_FRAMES = 45;

export const FONT_STACK =
  '"Helvetica Neue", Helvetica, Arial, sans-serif';

/** Parse `#rgb`/`#rrggbb` into a CSS-safe color, falling back to a neutral
 * accent for anything that doesn't parse — a composition must never throw
 * on a malformed (but schema-valid-as-a-string) accent_color prop. */
export function safeColor(value: string | undefined, fallback = "#DF6428"): string {
  if (!value) return fallback;
  const hex = value.trim();
  if (/^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/.test(hex)) {
    return hex;
  }
  return fallback;
}
