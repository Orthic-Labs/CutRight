import React from "react";
import { AbsoluteFill, interpolate, useCurrentFrame } from "remotion";
import type { CtaEndCardProps } from "../schemas";
import { FONT_STACK, safeColor } from "../layout";

// Registry footprint: top 20% / bottom 22% / left 8% / right 8% — a large
// centered band, matching `cta-end-card.v1`.
export const CtaEndCard: React.FC<CtaEndCardProps> = ({
  headline,
  subtext,
  accent_color,
  reducedMotion,
}) => {
  const frame = useCurrentFrame();
  const accent = safeColor(accent_color);
  const opacity = reducedMotion
    ? 1
    : interpolate(frame, [0, 12], [0, 1], { extrapolateRight: "clamp" });

  return (
    <AbsoluteFill style={{ backgroundColor: "transparent" }}>
      <div
        style={{
          position: "absolute",
          top: "20%",
          bottom: "22%",
          left: "8%",
          right: "8%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
          alignItems: "center",
          textAlign: "center",
          opacity,
          fontFamily: FONT_STACK,
        }}
      >
        <div
          style={{
            fontSize: 52,
            fontWeight: 800,
            color: "#F5F0E8",
            lineHeight: 1.1,
            textShadow: "0 2px 14px rgba(0,0,0,0.6)",
          }}
        >
          {headline}
        </div>
        {subtext ? (
          <div
            style={{
              fontSize: 24,
              fontWeight: 500,
              color: "#F5F0E8",
              opacity: 0.85,
              marginTop: 14,
            }}
          >
            {subtext}
          </div>
        ) : null}
        <div
          style={{
            marginTop: 24,
            padding: "10px 28px",
            borderRadius: 6,
            backgroundColor: accent,
            color: "#111110",
            fontSize: 20,
            fontWeight: 700,
            textTransform: "uppercase",
            letterSpacing: 1,
          }}
        >
          Learn more
        </div>
      </div>
    </AbsoluteFill>
  );
};
