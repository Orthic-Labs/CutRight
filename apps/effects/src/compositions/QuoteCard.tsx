import React from "react";
import { AbsoluteFill, interpolate, useCurrentFrame } from "remotion";
import type { QuoteCardProps } from "../schemas";
import { FONT_STACK, safeColor } from "../layout";

// Registry footprint: top 30% / bottom 30% / left 15% / right 15% — a
// centered band, matching `quote-card.v1`.
export const QuoteCard: React.FC<QuoteCardProps> = ({
  quote,
  attribution,
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
          top: "30%",
          bottom: "30%",
          left: "15%",
          right: "15%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
          alignItems: "center",
          textAlign: "center",
          opacity,
          fontFamily: FONT_STACK,
        }}
      >
        <div style={{ width: 56, height: 5, backgroundColor: accent, marginBottom: 20 }} />
        <div
          style={{
            fontSize: 40,
            fontWeight: 600,
            fontStyle: "italic",
            color: "#F5F0E8",
            lineHeight: 1.3,
            textShadow: "0 2px 12px rgba(0,0,0,0.6)",
          }}
        >
          &ldquo;{quote}&rdquo;
        </div>
        {attribution ? (
          <div
            style={{
              fontSize: 22,
              fontWeight: 500,
              color: accent,
              marginTop: 18,
              textTransform: "uppercase",
              letterSpacing: 1.2,
            }}
          >
            — {attribution}
          </div>
        ) : null}
      </div>
    </AbsoluteFill>
  );
};
