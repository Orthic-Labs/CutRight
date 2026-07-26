# Phase 1 transcription benchmark

CutRight uses HeardRight Parakeet TDT as the local primary transcript and WhisperX as an independent forced-alignment verifier. Neither provider is authorized to drive destructive word-edge cuts until this benchmark records a decision.

```text
videoctl bench transcribe <project> \
  --primary heardright \
  --verifier whisperx \
  --boundaries 20 \
  --padding-ms 40
```

The project must contain at least three immutable, independently recorded source clips. The command runs both providers, keeps the HeardRight canonical transcript intact, writes WhisperX transcripts with a `.whisperx.json` suffix, and writes `analysis/bench/transcribe/report.json`. Every provider call also preserves its untouched response in `cache/provider-responses/` and an adjacent `*.envelope.json` with the source hash, request hash, local-model ID, normalized transcript path, and cost record. This makes a later timestamp dispute reproducible without rerunning ASR.

For each provider boundary, the benchmark checks whether it lands more than the configured padding inside a word from the independent provider. A collision is a clipped-word failure. The report only selects a provider when exactly one provider has zero clipped boundaries; tied clean results and any nonzero result on both sides are `unresolved` and make the command fail. No downstream cut plan may treat an unresolved report as timestamp authority.

The report is an engineering gate, not an editorial approval. It must be rerun when the provider model, speaking setup, microphone, room, or word-edge policy changes.
