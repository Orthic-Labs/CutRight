# Recorded talking head (sample)

A 12-second vertical clip showing the primary lane end-to-end:

1. **Open**: `File → Open sample → recorded-talking-head`.
2. **Probe**: the media pack reports a 1080×1920 source at 29.97 fps.
3. **Transcribe**: the speech pack produces a timed transcript bound to
   the project's source hash.
4. **Plan**: the director pack returns an editorial plan; the visual
   critic pack passes the bounded evidence.
5. **Render**: the native compositor renders the timeline.
6. **Review**: the run lands in `ready`; you accept or note per clip.

The sample ships its sources, transcript and timeline. The operator's
real recording is loaded by reference id (`talking-head-cam-a`) from
the offline fixture directory, never by absolute path.

The sample exercises the `creator` lane; pack set
`creator-v2` is required.
