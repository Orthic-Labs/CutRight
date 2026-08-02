import React from "react";
import { Composition } from "remotion";
import { CANVAS_HEIGHT, CANVAS_WIDTH, DURATION_IN_FRAMES, FPS } from "./layout";
import { CtaEndCard } from "./compositions/CtaEndCard";
import { LowerThirdIdentityCard } from "./compositions/LowerThirdIdentityCard";
import { QuoteCard } from "./compositions/QuoteCard";
import { StatCounter } from "./compositions/StatCounter";
import {
  ctaEndCardSchema,
  lowerThirdIdentityCardSchema,
  quoteCardSchema,
  statCounterSchema,
} from "./schemas";

// One <Composition> per Remotion-rendered registry effect. Composition ids
// are the registry's effect_id with "." replaced by "-"
// (EFFECT_ID_TO_COMPOSITION_ID in ./schemas.ts documents the mapping).
// `caption.bold-karaoke.v1` is not registered here — it renders through the
// `ass` renderer (crates/video-media), not Remotion.
export const RemotionRoot: React.FC = () => {
  return (
    <>
      <Composition
        id="lower-third-identity-card-v1"
        component={LowerThirdIdentityCard}
        durationInFrames={DURATION_IN_FRAMES}
        fps={FPS}
        width={CANVAS_WIDTH}
        height={CANVAS_HEIGHT}
        schema={lowerThirdIdentityCardSchema}
        defaultProps={{
          name: "Adrian D'souza",
          title: "Founder",
          accent_color: "#FF5630",
          reducedMotion: false,
        }}
      />
      <Composition
        id="stat-counter-v1"
        component={StatCounter}
        durationInFrames={DURATION_IN_FRAMES}
        fps={FPS}
        width={CANVAS_WIDTH}
        height={CANVAS_HEIGHT}
        schema={statCounterSchema}
        defaultProps={{
          label: "Founders shipped",
          value: 149,
          accent_color: "#DF6428",
          reducedMotion: false,
        }}
      />
      <Composition
        id="quote-card-v1"
        component={QuoteCard}
        durationInFrames={DURATION_IN_FRAMES}
        fps={FPS}
        width={CANVAS_WIDTH}
        height={CANVAS_HEIGHT}
        schema={quoteCardSchema}
        defaultProps={{
          quote: "Local-first, always.",
          attribution: "HeardRight",
          accent_color: "#A78BFA",
          reducedMotion: false,
        }}
      />
      <Composition
        id="cta-end-card-v1"
        component={CtaEndCard}
        durationInFrames={DURATION_IN_FRAMES}
        fps={FPS}
        width={CANVAS_WIDTH}
        height={CANVAS_HEIGHT}
        schema={ctaEndCardSchema}
        defaultProps={{
          headline: "Try HeardRight",
          subtext: "Local-first dictation",
          accent_color: "#FF5630",
          reducedMotion: false,
        }}
      />
    </>
  );
};
