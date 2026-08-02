import React from "react";
import { AbsoluteFill, interpolate, spring, useCurrentFrame, useVideoConfig } from "remotion";
import type { LowerThirdIdentityCardProps } from "../schemas";
import { FONT_STACK, safeColor } from "../layout";

// Registry footprint: top 64% / bottom 14% / left 8% / right 45% — a band
// low on the frame, left-anchored, matching the registry's
// `lower-third.identity-card.v1` entry (schemas/effects/registry.json).
export const LowerThirdIdentityCard: React.FC<LowerThirdIdentityCardProps> = ({
  name,
  title,
  accent_color,
  reducedMotion,
}) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const accent = safeColor(accent_color);

  const progress = reducedMotion
    ? 1
    : spring({ frame, fps, config: { damping: 18, stiffness: 140 }, durationInFrames: 20 });
  const translateX = interpolate(progress, [0, 1], [-40, 0]);
  const opacity = interpolate(progress, [0, 1], [0, 1]);

  return (
    <AbsoluteFill style={{ backgroundColor: "transparent" }}>
      <div
        style={{
          position: "absolute",
          left: "8%",
          bottom: "14%",
          transform: `translateX(${translateX}px)`,
          opacity,
          display: "flex",
          flexDirection: "column",
          fontFamily: FONT_STACK,
          maxWidth: "47%",
        }}
      >
        <div style={{ width: 64, height: 6, backgroundColor: accent, marginBottom: 12 }} />
        <div
          style={{
            fontSize: 44,
            fontWeight: 700,
            color: "#F5F0E8",
            lineHeight: 1.1,
            textShadow: "0 2px 12px rgba(0,0,0,0.6)",
          }}
        >
          {name}
        </div>
        <div
          style={{
            fontSize: 24,
            fontWeight: 500,
            color: accent,
            marginTop: 6,
            textTransform: "uppercase",
            letterSpacing: 1.5,
          }}
        >
          {title}
        </div>
      </div>
    </AbsoluteFill>
  );
};
