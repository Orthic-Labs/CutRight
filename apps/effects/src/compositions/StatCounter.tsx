import React from "react";
import { AbsoluteFill, interpolate, useCurrentFrame, useVideoConfig } from "remotion";
import type { StatCounterProps } from "../schemas";
import { FONT_STACK, safeColor } from "../layout";

// Registry footprint: top 14% / bottom 70% / left 60% / right 8% — an
// upper-right band, matching `stat-counter.v1` in
// schemas/effects/registry.json. `motion_profile` is "expressive": a real
// count-up numeral animation, not a restrained fade.
export const StatCounter: React.FC<StatCounterProps> = ({
  label,
  value,
  accent_color,
  reducedMotion,
}) => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const accent = safeColor(accent_color);

  // Count-up settles a few frames before the clip ends so the final value
  // is visibly held, not still animating on the last frame.
  const settleFrame = Math.max(1, durationInFrames - 6);
  const displayed = reducedMotion
    ? value
    : interpolate(frame, [0, settleFrame], [0, value], {
        extrapolateLeft: "clamp",
        extrapolateRight: "clamp",
      });

  const formatted = Number.isInteger(value)
    ? Math.round(displayed).toLocaleString()
    : displayed.toFixed(1);

  return (
    <AbsoluteFill style={{ backgroundColor: "transparent" }}>
      <div
        style={{
          position: "absolute",
          top: "14%",
          right: "8%",
          display: "flex",
          flexDirection: "column",
          alignItems: "flex-end",
          fontFamily: FONT_STACK,
        }}
      >
        <div
          style={{
            fontSize: 88,
            fontWeight: 800,
            color: accent,
            lineHeight: 1,
            textShadow: "0 2px 16px rgba(0,0,0,0.6)",
            fontVariantNumeric: "tabular-nums",
          }}
        >
          {formatted}
        </div>
        <div
          style={{
            fontSize: 22,
            fontWeight: 500,
            color: "#F5F0E8",
            marginTop: 8,
            textTransform: "uppercase",
            letterSpacing: 1.2,
          }}
        >
          {label}
        </div>
      </div>
    </AbsoluteFill>
  );
};
