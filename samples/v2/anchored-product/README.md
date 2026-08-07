# Anchored product (sample)

A static product photograph with an anchored callout. The vision lane
detects the subject, places the graphic and renders the still.

1. **Open**: `File → Open sample → anchored-product`.
2. **Probe**: the media pack reports a 2048×2048 still.
3. **Detect**: the vision pack reports a single subject box.
4. **Anchor**: the design pack places the callout on the anchor.
5. **Render**: the native compositor renders the still timeline.
6. **Review**: the run lands in `ready`.

The sample ships its sources, transcript and timeline. The operator's
real photograph is loaded by reference id (`product-photo-1`) from the
offline fixture directory, never by absolute path.

The sample exercises the `vision` lane; pack set `creator-v2` is
required.
