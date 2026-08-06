# CutRight v2 baseline freeze — Book 1 task 001 evidence

Task: CR-V2-B1-001 "Freeze the v2 baseline and corpus date".
This file is frozen evidence. It records the corpus date, the pinned source
revisions from the v2 source ledger, and the hash of every current lockfile.
Later corpus changes require a new corpus revision and a compatibility
decision, not an edit to this file.

```yaml
corpus_date: 2026-08-06
cutright_commit: 7f3e5a61c729d4d877715b9a083d13a2e5ebe277
workspace_commit: 6ee21f03a787e7b57dc412760a8996ea7a235302
heardright_commit: b60bff947f12ffa9d25e94ad27e8ff30db006a24
autoshorts_commit: f17b04cdd97ef65c32b81b31b36bb6eb5d013d5b
vox_director_commit: 8b034354dc443edcde7fdb2622e0491df5142fd3
palmier_pro_commit: 397b82e64093f986cbabd89f1a1c93812ff546c2
llama_cpp_commit: 6a32c29a746a2e44de463de647f9f6661eb5086b
whisper_cpp_commit: 306c88f4d1286aec1bf96e544632897886af5501
silero_vad_commit: 76e3dc408eb2a5c655c34e230d2d5459b4439daa
mediapipe_commit: f8ef212d5c962c0e853db7e59d217056b187084b
ffmpeg_commit: 9047fa1b084f76b1b4d065af2d743df1b40dfb56
```

## Source identity table

| Source | Location | Pinned revision | Disposition posture |
| --- | --- | --- | --- |
| CutRight | Orthic-Labs/CutRight | 7f3e5a61c729d4d877715b9a083d13a2e5ebe277 | shipping base (MIT) |
| Workspace capabilities | bogusyogi/claude | 6ee21f03a787e7b57dc412760a8996ea7a235302 | vendor selected closure |
| HeardRight | bogusyogi/heardright | b60bff947f12ffa9d25e94ad27e8ff30db006a24 | vendor and adapt |
| AutoShorts | JayWebtech/autoshorts | f17b04cdd97ef65c32b81b31b36bb6eb5d013d5b | behavior only |
| Vox Director | Alisa0808/vox-director | 8b034354dc443edcde7fdb2622e0491df5142fd3 | adapt with notice |
| Palmier Pro | palmier-io/palmier-pro | 397b82e64093f986cbabd89f1a1c93812ff546c2 | clean-room behavior only |
| llama.cpp | ggml-org/llama.cpp | 6a32c29a746a2e44de463de647f9f6661eb5086b | vendor runtime source |
| whisper.cpp | ggml-org/whisper.cpp | 306c88f4d1286aec1bf96e544632897886af5501 | vendor verifier source |
| Silero VAD | snakers4/silero-vad | 76e3dc408eb2a5c655c34e230d2d5459b4439daa | vendor model/runtime subset |
| MediaPipe | google-ai-edge/mediapipe | f8ef212d5c962c0e853db7e59d217056b187084b | conditional source component |
| FFmpeg | FFmpeg/FFmpeg n8.1 | 9047fa1b084f76b1b4d065af2d743df1b40dfb56 | LGPL build + corresponding source |

All revisions are immutable commit hashes. Mutable branches, `latest` tags,
and unversioned download URLs are invalid as corpus revisions.

## Lockfile hashes (SHA-256)

```text
b601c3b644409bea7efaf728077044e722cc7934c1564a2c1aab46eb0851aa9a  Cargo.lock
e673fdb62cf89de3eada99f5ca412c2abd4e02ed987270a3f908bd036c5d6e8c  apps/effects/pnpm-lock.yaml
ed32c32c7b083b46e481e6b4a6c2c1c655df653f3660e572831f95f2ced34bd7  apps/studio/pnpm-lock.yaml
```

## Repository-shape guard result at freeze time

No dedicated v2 repository-shape guard exists yet; task CR-V2-B1-005 creates
`scripts/gates/v2-repository-shape.sh`. Shape facts observed at freeze time:

- `.gitmodules`: absent.
- `.github/workflows`: absent (no hosted CI).
- Authoritative local gate: `scripts/gate.sh` (present, executable).
- Baseline commit verified: `git rev-parse HEAD` equals the pinned CutRight
  commit above; branch `main` at `origin/main`.

## Production-code statement

This task modifies no production code; the only written artefact is this
evidence file.
