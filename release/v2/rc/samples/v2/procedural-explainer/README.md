# Procedural explainer (sample)

A synthetic explainer built from a short script and procedural motion
graphics. No camera source, no third-party assets.

1. **Open**: `File → Open sample → procedural-explainer`.
2. **Resolve**: the script id resolves from the sample's `sources/`.
3. **Captions**: the speech pack produces per-word timing from the
   synthetic script.
4. **Motion**: the creative pack composes a procedural background.
5. **Render**: the native compositor renders the timeline.
6. **Review**: the run lands in `ready`.

The sample ships its sources, transcript and timeline. Every word and
frame is reproducible from the bytes in the working tree.

The sample exercises the `creative` lane; pack set `creator-v2` is
required.
