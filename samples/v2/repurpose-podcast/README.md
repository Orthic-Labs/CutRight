# Repurpose podcast (sample)

A 60-minute podcast audio reduced to one short vertical clip. This
sample exercises the speech lane without a camera source.

1. **Open**: `File → Open sample → repurpose-podcast`.
2. **Probe**: the media pack reports a 48 kHz stereo audio source.
3. **Transcribe**: the speech pack produces a timed transcript; the
   verifier pack cross-checks a fraction of words.
4. **Beat**: the director pack scores beats and selects a single clip.
5. **Render**: the native compositor renders a still-image-on-audio
   timeline.
6. **Review**: the run lands in `ready`.

The sample ships its sources, transcript and timeline. The operator's
real episode is loaded by reference id (`podcast-mic-1`) from the
offline fixture directory, never by absolute path.

The sample exercises the `speech` lane; pack set `creator-v2` is
required.
